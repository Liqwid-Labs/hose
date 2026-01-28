pub mod config;
pub mod context;
pub use context::DevnetContext;
use hose::primitives::TxHash;
pub use hose_devnet_macros::devnet_test;
use hydrant::primitives::TxOutputPointer;
use tracing::debug;
pub use {serial_test, test_context, tokio};

pub mod prelude {
    pub use super::{DevnetContext, devnet_test, serial_test, test_context, tokio};
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
            hex::encode(output_pointer.hash.as_ref()),
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
