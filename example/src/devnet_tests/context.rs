use std::sync::Arc;
use std::sync::atomic::AtomicBool;

use anyhow::Context as _;
use clap::Parser as _;
use hose::ogmios::OgmiosClient;
use hose::wallet::{Wallet, WalletBuilder};
use hydrant::UtxoIndexer;
use pallas::ledger::primitives::NetworkId;
use test_context::AsyncTestContext;
use tokio::sync::Mutex;
use url::Url;

use crate::config::{self, Config};
use crate::devnet_tests::indexer::IndexerContext;
use crate::devnet_tests::lock::TestLock;

static LOCK: AtomicBool = AtomicBool::new(false);

pub struct DevnetContext {
    pub config: Config,
    pub network_id: NetworkId,
    pub ogmios: OgmiosClient,
    pub protocol_params: hose::ogmios::pparams::ProtocolParams,
    pub wallet: Wallet,
    pub indexer: Arc<Mutex<UtxoIndexer>>,
}

impl AsyncTestContext for DevnetContext {
    async fn setup() -> Self {
        let _lock = TestLock::wait_and_lock().await;

        match tracing_subscriber::fmt::try_init() {
            Ok(_) => (),
            Err(e) => {
                // Ignore error, tracing probably is already initialized
                // TODO: Could we catch this better?
            }
        }
        dotenv::dotenv()
            .context("could not load .env file")
            .unwrap();

        let config = config::Config::parse();
        let network_id = NetworkId::try_from(config.network.value())
            .expect("failed to convert network to network id");

        let ogmios = OgmiosClient::new(Url::parse(&config.ogmios_url).unwrap());

        let protocol_params = ogmios.protocol_params().await.unwrap();

        let wallet = WalletBuilder::new(config.network.clone())
            .from_hex(config.private_key_hex.clone())
            .unwrap();

        let indexer = IndexerContext::acquire_indexer(&config, wallet.address().clone())
            .await
            .unwrap();

        Self {
            config,
            network_id,
            ogmios,
            protocol_params,
            wallet,
            indexer,
        }
    }

    fn teardown(self) -> impl std::future::Future<Output = ()> + Send {
        async {}
    }
}
