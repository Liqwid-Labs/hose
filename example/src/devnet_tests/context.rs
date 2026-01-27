use std::sync::Arc;
use std::time::Duration;

use anyhow::Context as _;
use clap::Parser as _;
use hose::ogmios::OgmiosClient;
use hose::primitives::AssetId;
use hose::wallet::{Wallet, WalletBuilder};
use hydrant::primitives::AssetIdResolver;
use hydrant::{Sync, UtxoIndexer};
use pallas::ledger::addresses::Network;
use pallas::ledger::primitives::NetworkId;
use pallas::network::facades::PeerClient;
use test_context::AsyncTestContext;
use tokio::sync::Mutex;
use tokio_blocked::TokioBlockedLayer;
use tracing_subscriber::layer::SubscriberExt as _;
use tracing_subscriber::util::SubscriberInitExt as _;
use tracing_subscriber::{EnvFilter, Layer};
use url::Url;

use crate::config::{self, Config};

pub struct DevnetContext {
    pub config: Config,
    pub network_id: NetworkId,
    pub ogmios: OgmiosClient,
    pub protocol_params: hose::ogmios::pparams::ProtocolParams,
    pub wallet: Wallet,
    pub sync: Sync,
    pub indexer: Arc<Mutex<UtxoIndexer>>,
}

impl AsyncTestContext for DevnetContext {
    async fn setup() -> Self {
        Self::new().await
    }

    async fn teardown(self) {
        self.sync.stop().await.unwrap();
    }
}

impl DevnetContext {
    pub async fn new() -> Self {
        dotenv::dotenv()
            .context("could not load .env file")
            .unwrap();
        init_tracing();

        let config = config::Config::parse();
        let network_id = NetworkId::try_from(config.network.value())
            .expect("failed to convert network to network id");

        let ogmios = OgmiosClient::new(Url::parse(&config.ogmios_url).unwrap());

        let protocol_params = ogmios.protocol_params().await.unwrap();

        let wallet = WalletBuilder::new(config.network)
            .from_hex(config.private_key_hex.clone())
            .unwrap();

        let db = hydrant::Db::new(config.db_path.to_str().unwrap()).expect("failed to open db");

        let indexer = UtxoIndexer::builder()
            .build(&db.env)
            .expect("failed to build indexer");
        let indexer = Arc::new(Mutex::new(indexer));

        tracing::info!("Connecting to node...");
        let node = PeerClient::connect(&config.node_host, get_magic(config.network))
            .await
            .expect("failed to connect to node");

        let genesis_config = config::genesis_config(&config).unwrap();
        let mut sync = hydrant::Sync::new(node, &db, &vec![indexer.clone()], genesis_config)
            .await
            .expect("failed to start sync");

        // Sync to tip at least once
        sync.run_until_synced().await.unwrap();

        Self {
            config,
            network_id,
            ogmios,
            protocol_params,
            wallet,
            sync,
            indexer,
        }
    }
}

fn init_tracing() {
    let fmt = tracing_subscriber::fmt::layer().with_filter(EnvFilter::from_default_env());
    let blocked =
        TokioBlockedLayer::new().with_warn_busy_single_poll(Some(Duration::from_micros(150)));
    let _ = tracing_subscriber::registry()
        .with(fmt)
        .with(blocked)
        .try_init();
}

fn get_magic(network: Network) -> u64 {
    match network {
        Network::Testnet => 5,
        Network::Mainnet => 764824073,
        Network::Other(n) => n as u64,
    }
}
