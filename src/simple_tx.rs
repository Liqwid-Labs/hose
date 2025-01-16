use betterfrost_client::addresses as betterfrost;
use pallas::{
    ledger::addresses::{self, Address, PaymentKeyHash},
    txbuilder::{self, BuildConway, BuiltTransaction, Input, Output, StagingTransaction},
};

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

#[derive(Debug)]
#[allow(clippy::enum_variant_names)]
pub enum Error {
    #[allow(dead_code)]
    ClientError(betterfrost_client::Error),
    #[allow(dead_code)]
    TxBuilderError(txbuilder::TxBuilderError),
    #[allow(dead_code)]
    AddressError(addresses::Error),
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
    client: &betterfrost_client::Client,
    target_user: TargetUser,
    config: &Config,
) -> Result<BuiltTransaction> {
    let tx = StagingTransaction::new();

    let address = target_user.address.clone();

    let own_utxos = client
        .address_utxos(config.wallet_address.clone(), Default::default())
        .await?;

    // TODO: estimate the fee
    let fee = 0; // 200_000;

    let valid_fee_utxos = own_utxos
        .iter()
        .filter(|utxo| utxo_lovelace(utxo) >= fee)
        .collect::<Vec<_>>();

    let tx = tx.fee(fee);

    let utxo_to_spend = valid_fee_utxos.first().unwrap();

    let tx = tx.input(address_utxo_to_input(utxo_to_spend));

    let o1 = Output::new(
        Address::from_bech32(&config.wallet_address.clone())?,
        utxo_lovelace(utxo_to_spend) - fee,
    );

    let tx = tx.output(o1.clone());

    let tx = tx.change_address(address.clone());

    let tx = tx.network_id(config.network.clone().into());

    let tx = tx.collateral_input(address_utxo_to_input(utxo_to_spend));

    // Collateral outputs are a CIP-40 feature. We don't need them for now.
    // let tx = tx.collateral_output(o1);

    let tx = tx.disclosed_signer(address_payment_key_hash(&address));

    let built_tx = tx.build_conway_raw()?;

    Ok(built_tx)
}
