use std::cmp::Reverse;
use std::sync::Arc;

use anyhow::Context;
use hydrant::UtxoIndexer;
use hydrant::primitives::{AssetsDelta, TxOutput};
use ogmios_client::method::pparams::ProtocolParams;
use pallas::ledger::addresses::Address as PallasAddress;
use pallas::ledger::primitives::Fragment;
use tokio::sync::Mutex;

use crate::builder::{Output, StagingTransaction};
use crate::primitives::{Assets, Certificate, DatumOption};

pub async fn get_input_lovelace(
    indexer: Arc<Mutex<UtxoIndexer>>,
    tx: &StagingTransaction,
) -> anyhow::Result<u64> {
    let indexer = indexer.lock().await;
    Ok(indexer
        .utxos(&tx.inputs)?
        .iter()
        .map(|utxo| utxo.lovelace)
        .sum())
}

pub async fn get_input_assets(
    indexer: Arc<Mutex<UtxoIndexer>>,
    tx: &StagingTransaction,
) -> anyhow::Result<Assets> {
    let indexer = indexer.lock().await;
    Ok(indexer
        .utxos(&tx.inputs)?
        .iter()
        .map(|utxo| utxo.assets.clone())
        .sum())
}

pub fn get_output_lovelace(tx: &StagingTransaction) -> u64 {
    tx.outputs.iter().map(|output| output.lovelace).sum()
}

// registration certificates consume a deposit from the inputs, while deregistration
// certificates refund them.
pub fn get_registration_deposit(tx: &StagingTransaction) -> u64 {
    tx.certificates
        .iter()
        .filter_map(|cert| match cert {
            Certificate::StakeRegistrationScript { deposit, .. } => *deposit,
            Certificate::StakeDeregistrationScript { .. } => None,
        })
        .sum()
}

pub fn get_deregistration_refund(tx: &StagingTransaction) -> u64 {
    tx.certificates
        .iter()
        .filter_map(|cert| match cert {
            Certificate::StakeRegistrationScript { .. } => None,
            Certificate::StakeDeregistrationScript { deposit, .. } => *deposit,
        })
        .sum()
}

pub fn get_withdrawal_lovelace(tx: &StagingTransaction) -> u64 {
    tx.withdrawals.values().copied().sum()
}
pub fn get_output_assets(tx: &StagingTransaction) -> Assets {
    tx.outputs
        .iter()
        .flat_map(|output| output.assets.clone())
        .sum()
}

pub async fn select_coins(
    input_lovelace: u64,
    input_assets: Assets,
    pparams: &ProtocolParams,
    possible_utxos: &[TxOutput],
    tx: &StagingTransaction,
    fee: u64,
    change_address: &PallasAddress,
    change_datum: Option<DatumOption>,
) -> anyhow::Result<Vec<TxOutput>> {
    let mut selected_utxos = vec![];

    // Filter utxos already used as inputs
    // TODO: should also filter out utxos with scripts? utxos with datums?
    let mut possible_utxos = possible_utxos
        .iter()
        .filter(|utxo| !tx.inputs.iter().any(|input| input == *utxo))
        .collect::<Vec<_>>();

    // TODO: consider minted assets

    let change_assets = input_assets
        .clone()
        .saturating_sub(get_output_assets(tx))
        .into();
    let mut dummy_change_output = Output::new(change_address.clone(), u64::MAX)
        .add_assets(change_assets)
        .context("failed to create dummy change output")?;

    if let Some(datum) = change_datum {
        dummy_change_output.datum = Some(datum);
    }

    let min_change_lovelace = pparams.min_utxo_deposit_coefficient
        * (dummy_change_output
            .build_babbage()
            .context("failed to build dummy change output")?
            .encode_fragment()
            .unwrap()
            .len() as u64
            + 160);

    let registration_deposit = get_registration_deposit(tx);
    let deregistration_refund = get_deregistration_refund(tx);
    let withdrawal_lovelace = get_withdrawal_lovelace(tx);
    let mut required_lovelace =
        (get_output_lovelace(tx) + fee + min_change_lovelace + registration_deposit)
            .saturating_sub(input_lovelace + withdrawal_lovelace + deregistration_refund);
    let mut required_assets: AssetsDelta =
        get_output_assets(tx).saturating_sub(input_assets).into();

    // Select for assets
    while !possible_utxos.is_empty()
        && let Some(asset) = required_assets.only_positive().keys().next()
    {
        // Largest-first but now by asset amount
        possible_utxos.sort_by_key(|utxo| Reverse(*utxo.assets.get(asset).unwrap_or(&0)));

        let utxo = possible_utxos.remove(0);
        if utxo.assets.get(asset).unwrap_or(&0) == &0 {
            break;
        }

        required_assets = required_assets - utxo.assets.clone().into();
        selected_utxos.push(utxo.clone());
    }

    // Select for lovelace
    possible_utxos.sort_by_key(|utxo| Reverse(utxo.lovelace)); // Largest-first
    while !possible_utxos.is_empty() && required_lovelace > 0 {
        let utxo = possible_utxos.remove(0);
        required_lovelace = required_lovelace.saturating_sub(utxo.lovelace);
        selected_utxos.push(utxo.clone());
    }

    if required_lovelace > 0 || !required_assets.only_positive().is_empty() {
        return Err(anyhow::anyhow!(
            "failed to select coins, wallet doesn't contain enough funds"
        ));
    }

    Ok(selected_utxos)
}

/// Create change output if needed because transaction is not balanced.
pub async fn handle_change(
    indexer: Arc<Mutex<UtxoIndexer>>,
    change_address: &PallasAddress,
    tx: &StagingTransaction,
    fee: u64,
    change_datum: Option<DatumOption>,
) -> anyhow::Result<Option<Output>> {
    // TODO: consider minted assets
    let input_lovelace = get_input_lovelace(indexer.clone(), tx).await?;
    let registration_deposit = get_registration_deposit(tx);
    let deregistration_refund = get_deregistration_refund(tx);
    let withdrawal_lovelace = get_withdrawal_lovelace(tx);
    let output_lovelace = get_output_lovelace(tx);
    let change_lovelace = (input_lovelace + withdrawal_lovelace + deregistration_refund)
        .saturating_sub(output_lovelace + fee + registration_deposit);

    let input_assets: AssetsDelta = get_input_assets(indexer, tx).await?.into();
    let output_assets: AssetsDelta = get_output_assets(tx).into();
    let change_assets = input_assets - output_assets;

    if change_lovelace == 0 && change_assets.only_positive().is_empty() {
        return Ok(None);
    }

    let mut change_output =
        Output::new(change_address.clone(), change_lovelace)
            .add_assets(change_assets.into())
            .expect("failed to create change output");

    if let Some(datum) = change_datum {
        change_output.datum = Some(datum);
    }

    Ok(Some(change_output))
}
