use crate::devnet_tests::context::DevnetContext;

pub async fn wait_n_slots(_context: &DevnetContext, n: u64) -> anyhow::Result<()> {
    // TODO: Use ogmios API to check slots

    // Currently, we just wait N * 100ms
    std::thread::sleep(std::time::Duration::from_millis(n * 100));

    Ok(())
}
