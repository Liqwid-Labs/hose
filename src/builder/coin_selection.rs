use std::cmp::Reverse;
use std::sync::Arc;

use hydrant::UtxoIndexer;
use hydrant::primitives::{AssetsDelta, TxOutput};
use ogmios_client::method::pparams::ProtocolParams;
use pallas::ledger::addresses::Address as PallasAddress;
use tokio::sync::Mutex;

use crate::builder::{Output, StagingTransaction};
use crate::primitives::Assets;

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

pub fn get_certificate_deposit(tx: &StagingTransaction) -> u64 {
    tx.certificates.iter().map(|cert| cert.deposit()).sum()
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
) -> Vec<TxOutput> {
    let mut selected_utxos = vec![];

    // Filter utxos already used as inputs
    // TODO: should also filter out utxos with scripts? utxos with datums?
    let mut possible_utxos = possible_utxos
        .iter()
        .filter(|utxo| !tx.inputs.iter().any(|input| input == *utxo))
        .collect::<Vec<_>>();

    // TODO: consider minted assets

    // assume a change output of maximum 500 bytes
    // TODO: technically we should use the actual size of the change output
    let min_change_lovelace = pparams.min_utxo_deposit_coefficient * 500;
    let deposit_lovelace = get_certificate_deposit(tx);
    let mut required_lovelace = (get_output_lovelace(tx)
        + fee
        + min_change_lovelace
        + deposit_lovelace)
        .saturating_sub(input_lovelace);
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

    // TODO: give a proper error
    assert!(
        required_lovelace == 0 && required_assets.only_positive().is_empty(),
        "failed to select coins, wallet doesn't contain enough funds"
    );

    selected_utxos
}

/// Create change output if needed because transaction is not balanced.
pub async fn handle_change(
    indexer: Arc<Mutex<UtxoIndexer>>,
    change_address: &PallasAddress,
    tx: &StagingTransaction,
    fee: u64,
) -> anyhow::Result<Option<Output>> {
    // TODO: consider minted assets
    let input_lovelace = get_input_lovelace(indexer.clone(), tx).await?;
    let deposit_lovelace = get_certificate_deposit(tx);
    let output_lovelace = get_output_lovelace(tx);
    let change_lovelace = input_lovelace
        .saturating_sub(output_lovelace)
        .saturating_sub(fee);
    let change_lovelace = change_lovelace.saturating_sub(deposit_lovelace);

    let input_assets: AssetsDelta = get_input_assets(indexer, tx).await?.into();
    let output_assets: AssetsDelta = get_output_assets(tx).into();
    let change_assets = input_assets - output_assets;

    if change_lovelace == 0 && change_assets.only_positive().is_empty() {
        return Ok(None);
    }

    let change_output =
        Output::new(change_address.clone(), change_lovelace).add_assets(change_assets.into());
    Ok(Some(change_output.expect("failed to create change output")))
}
