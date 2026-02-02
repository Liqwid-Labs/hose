pub mod config;
pub mod context;
use std::time::{SystemTime, UNIX_EPOCH};

pub use context::DevnetContext;
use hose::primitives::{Address, Script, ScriptKind, TxHash};
pub use hose_devnet_macros::test;
use hydrant::primitives::TxOutputPointer;
use pallas::ledger::addresses::{
    Network, ShelleyAddress, ShelleyDelegationPart, ShelleyPaymentPart,
};
use pallas::ledger::primitives::NetworkId;
use tracing::debug;
use uplc::Fragment;
use uplc::tx::apply_params_to_script;
use uplc::tx::to_plutus_data::ToPlutusData;
pub use {serial_test, test_context, tokio};

pub mod prelude {
    pub use super::{DevnetContext, serial_test, test, test_context, tokio};
}

pub async fn wait_n_slots(_context: &DevnetContext, n: u64) -> anyhow::Result<()> {
    // TODO: Use ogmios API to check slots

    // Currently, we just wait N * 100ms
    tokio::time::sleep(std::time::Duration::from_millis(n * 100)).await;

    Ok(())
}

pub async fn wait_until_utxo_exists(
    context: &DevnetContext,
    output_pointer: TxOutputPointer,
) -> anyhow::Result<()> {
    loop {
        debug!(
            "Waiting for utxo to exist: {}#{}",
            output_pointer.hash.to_hex(),
            output_pointer.index
        );
        {
            let indexer = context.indexer.lock().await;
            if indexer.utxo(output_pointer.clone())?.is_some() {
                return Ok(());
            }
        }
        tokio::time::sleep(std::time::Duration::from_millis(1)).await;
    }
}

pub async fn wait_until_tx_is_included(
    context: &DevnetContext,
    tx_hash: TxHash,
) -> anyhow::Result<()> {
    // A transaction always has at least one output, so we can use the first output as a pointer.us
    let utxo_pointer = TxOutputPointer::new(tx_hash, 0);
    wait_until_utxo_exists(context, utxo_pointer).await?;
    Ok(())
}

pub fn empty_redeemer() -> Vec<u8> {
    hex::decode("00").unwrap()
}

pub fn network_from_network_id(network_id: NetworkId) -> Network {
    match network_id {
        NetworkId::Mainnet => Network::Mainnet,
        NetworkId::Testnet => Network::Testnet,
    }
}

pub fn validator_to_address(context: &DevnetContext, validator: &Script) -> Address {
    Address::Shelley(ShelleyAddress::new(
        network_from_network_id(context.network_id),
        ShelleyPaymentPart::Script(validator.hash.into()),
        ShelleyDelegationPart::Null,
    ))
}

pub fn nonced_always_succeeds_script() -> anyhow::Result<Script> {
    // This is just an always succeeds that takes an integer as a parameter and ignores it.
    let base_script_bytes = hex::decode("5601010022332259800a518a4d136564008ae68dd68011")?;
    // We apply the unix time as the nonce just so we have a different script for each run,
    // which avoids problems with rewards accounts (that cannot be registered twice in a row).
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)?
        // Theoretically unsafe, but  will fit into a u64 for the next few million years :)
        .as_millis() as u64;

    let params = vec![nonce].to_plutus_data();
    let params_bytes = params
        .encode_fragment()
        .map_err(|err| anyhow::anyhow!("failed to encode params: {err:?}"))?;
    let script_bytes = apply_params_to_script(&params_bytes, &base_script_bytes)
        .map_err(|err| anyhow::anyhow!("failed to apply params to script: {err:?}"))?;
    Ok(Script::new(ScriptKind::PlutusV3, script_bytes))
}
