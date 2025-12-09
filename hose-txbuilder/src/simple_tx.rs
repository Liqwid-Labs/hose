use betterfrost_client::v0::addresses::{self as betterfrost, AddressUtxo};
use pallas::{
    applying::MultiEraProtocolParameters,
    ledger::addresses::{self, Address, PaymentKeyHash},
    txbuilder::{self, BuildConway, BuiltTransaction, Input, Output, StagingTransaction},
};
use thiserror::Error;

use crate::config::Config;

#[derive(Clone)]
pub struct TargetUser {
    pub address: Address,
}

impl TargetUser {
    pub fn from_local_config(config: &Config) -> anyhow::Result<Self> {
        Ok(TargetUser {
            address: Address::from_bech32(&config.wallet_address.clone())?,
        })
    }
}

#[derive(Error, Debug)]
#[allow(clippy::enum_variant_names)]
pub enum Error {
    #[allow(dead_code)]
    #[error("Betterfrost error")]
    ClientError(betterfrost_client::Error),
    #[allow(dead_code)]
    #[error("Transaction builder error: {0}")]
    TxBuilderError(txbuilder::TxBuilderError),
    #[allow(dead_code)]
    #[error("Address error: {0}")]
    AddressError(addresses::Error),

    #[error("No utxos to spend")]
    NoUtxosToSpend,

    #[error("Fee calculation failed")]
    FeeCalculationFailed,
}

impl From<addresses::Error> for Error {
    fn from(e: addresses::Error) -> Self {
        Self::AddressError(e)
    }
}

impl From<betterfrost_client::Error> for Error {
    fn from(e: betterfrost_client::Error) -> Self {
        Self::ClientError(e)
    }
}

impl From<txbuilder::TxBuilderError> for Error {
    fn from(e: txbuilder::TxBuilderError) -> Self {
        Self::TxBuilderError(e)
    }
}

pub type Result<T, E = Error> = std::result::Result<T, E>;

pub fn utxo_lovelace(utxo: &betterfrost::AddressUtxo) -> u64 {
    utxo.amount
        .iter()
        .filter(|amount| amount.0.unit == "lovelace")
        .map(|amount| amount.0.quantity.parse::<u64>().unwrap())
        .sum()
}

pub fn address_utxo_to_input(utxo: &betterfrost::AddressUtxo) -> Input {
    Input {
        tx_hash: serde_json::from_value(serde_json::Value::String(utxo.tx_hash.clone()))
            .expect("to parse tx_hash"),
        txo_index: utxo
            .tx_index
            .abs()
            .try_into()
            .expect("Extending i16 to u64"),
    }
}

pub fn address_payment_key_hash(address: &Address) -> PaymentKeyHash {
    match address {
        Address::Shelley(x) => *x.payment().as_hash(),
        Address::Stake(x) => *x.payload().as_hash(),
        _ => panic!("not a payment address"),
    }
}

pub async fn simple_transaction(
    client: &betterfrost_client::v0::Client,
    address: Address,
    config: &Config,
) -> Result<BuiltTransaction> {
    let own_utxos = client
        .address_utxos(config.wallet_address.clone(), Default::default())
        .await?;

    // We create the transaction twice. First we create it with a fixed fee, that's larger
    // than any possible fee we might calculate. Then we calculate the actual fee and
    // recreate the transaction
    let tmp_fee = 1_000_000;
    let tx = build_tx(&address, tmp_fee, &own_utxos, config)?;

    let actual_fee = calculate_fee(tx.clone(), config)?;
    let tx = build_tx(&address, actual_fee, &own_utxos, config)?;

    if calculate_fee(tx.clone(), config)? != actual_fee {
        return Err(Error::FeeCalculationFailed);
    }

    Ok(tx.build_conway_raw()?)
}

fn build_tx(
    address: &Address,
    fee: u64,
    utxos: &[AddressUtxo],
    config: &Config,
) -> Result<StagingTransaction> {
    let valid_fee_utxos = utxos
        .iter()
        .filter(|utxo| utxo_lovelace(utxo) >= fee)
        .collect::<Vec<_>>();
    let utxo_to_spend = valid_fee_utxos.first().ok_or(Error::NoUtxosToSpend)?;

    let input = address_utxo_to_input(utxo_to_spend);
    let output = Output::new(
        Address::from_bech32(&config.wallet_address.clone())?,
        utxo_lovelace(utxo_to_spend) - fee,
    );

    Ok(StagingTransaction::new()
        .fee(fee)
        .input(input)
        .output(output)
        .change_address(address.clone())
        .network_id(config.network.into())
        .collateral_input(address_utxo_to_input(utxo_to_spend))
        // Collateral outputs are a CIP-40 feature. We don't need them for now.
        // .collateral_output(output);
        .disclosed_signer(address_payment_key_hash(address)))
}

fn calculate_fee(tx: StagingTransaction, config: &Config) -> Result<u64> {
    let signed_tx = tx
        .build_conway_raw()?
        .sign(config.wallet_payment_key.to_ed25519_private_key())?;

    let params = match config.network.into() {
        MultiEraProtocolParameters::Conway(params) => params,
        _ => todo!("Implement support for non-conway protocol parameters in fee computation"),
    };

    // TODO: calculate the fee for the script the simple way by setting a maximum mem and cpu usage
    // params.execution_costs.mem_price
    // params.execution_costs.cpu_price

    let coefficient = params.minfee_a;
    let constant = params.minfee_b;

    Ok((coefficient * (signed_tx.tx_bytes.0.len() as u32) + constant).into())
}
