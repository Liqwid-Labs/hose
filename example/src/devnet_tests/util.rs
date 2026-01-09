use crate::devnet_tests::test::DevnetContext;

pub async fn wait_n_slots(context: &mut DevnetContext, n: u64) -> anyhow::Result<()> {
    // TODO: Use ogmios API to check slots

    // Currently, we just wait N * 100ms
    std::thread::sleep(std::time::Duration::from_millis(n * 100));

    Ok(())
}
