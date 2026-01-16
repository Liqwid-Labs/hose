use std::time::Duration;

use hex;
use hose::primitives::TxHash;
use hydrant::primitives::TxOutputPointer;
use tracing::info;

use crate::devnet_tests::context::DevnetContext;

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
        info!(
            "Waiting for utxo to exist: {}#{}",
            hex::encode(output_pointer.hash.as_ref()),
            output_pointer.index
        );
        let utxo_exists = {
            let indexer = context.indexer.lock().await;
            indexer.utxo(output_pointer.clone())?.is_some()
        };
        if utxo_exists {
            return Ok(());
        }
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
}

pub async fn wait_until_tx_is_included(
    context: &DevnetContext,
    tx_hash: TxHash,
) -> anyhow::Result<()> {
    // A transaction always has at least one output, so we can use the first output as a pointer.us
    let utxo_pointer = TxOutputPointer::new(tx_hash.into(), 0);
    wait_until_utxo_exists(context, utxo_pointer).await?;
    Ok(())
}
