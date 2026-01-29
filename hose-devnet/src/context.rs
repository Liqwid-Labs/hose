use std::sync::Arc;

use anyhow::Context as _;
use clap::Parser as _;
use hose::builder::BuiltTx;
use hose::wallet::{Wallet, WalletBuilder};
use hydrant::UtxoIndexer;
use ogmios_client::OgmiosHttpClient;
use ogmios_client::method::pparams::ProtocolParams;
use ogmios_client::method::submit::SubmitResult;
use pallas::ledger::addresses::Network;
use pallas::ledger::primitives::NetworkId;
use pallas::network::facades::PeerClient;
use test_context::AsyncTestContext;
use tokio::sync::Mutex;
use tracing_subscriber::layer::SubscriberExt as _;
use tracing_subscriber::util::SubscriberInitExt as _;
use tracing_subscriber::{EnvFilter, Layer as _};
use url::Url;

use crate::config::{self, Config};

pub struct DevnetContext {
    pub config: Config,
    pub network_id: NetworkId,
    pub ogmios: OgmiosHttpClient,
    pub protocol_params: ProtocolParams,
    pub wallet: Wallet,
    pub sync_handle: tokio::task::JoinHandle<()>,
    pub indexer: Arc<Mutex<UtxoIndexer>>,
}

impl AsyncTestContext for DevnetContext {
    async fn setup() -> Self {
        Self::new().await
    }

    async fn teardown(self) {
        self.sync_handle.abort();
    }
}

impl DevnetContext {
    pub async fn new() -> Self {
        dotenv::dotenv()
            .context("could not load .env file")
            .unwrap();
        init_tracing();

        let config = config::Config::parse_from::<_, String>(vec![]);
        let network_id = NetworkId::try_from(config.network.value())
            .expect("failed to convert network to network id");

        let ogmios = OgmiosHttpClient::new(Url::parse(&config.ogmios_url).unwrap());

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

        let ws_ogmios_url = Some(config.ogmios_url.replace("http", "ws").parse().unwrap());

        let mut sync = hydrant::Sync::new(
            node,
            &db,
            &vec![indexer.clone()],
            genesis_config,
            ws_ogmios_url,
        )
        .await
        .expect("failed to start sync");

        sync.run_until_synced().await.expect("failed to sync");

        let sync_handle = tokio::spawn(async move {
            if let Err(e) = sync.run().await {
                tracing::error!("Sync task failed: {:?}", e);
            }
        });

        Self {
            config,
            network_id,
            ogmios,
            protocol_params,
            wallet,
            sync_handle,
            indexer,
        }
    }

    pub async fn sign_and_submit_tx(&self, tx: BuiltTx) -> anyhow::Result<(BuiltTx, SubmitResult)> {
        let signed = tx.sign(&self.wallet)?;
        tracing::info!("Submitting transaction: {}", signed.hash()?);
        match self.ogmios.submit(&signed.cbor()).await {
            Ok(res) => {
                tracing::debug!("Submitted transaction: {:?}", res.transaction.id);
                assert_eq!(res.transaction.id, signed.hash()?.to_string());
                crate::wait_until_tx_is_included(self, signed.hash()?).await?;
                Ok((signed, res))
            }
            Err(e) => {
                tracing::info!("Failed transaction CBOR: {:?}", signed.cbor_hex());
                Err(anyhow::anyhow!("Failed to submit transaction: {:?}", e))
            }
        }
    }
}

fn init_tracing() {
    let fmt = tracing_subscriber::fmt::layer().with_filter(EnvFilter::from_default_env());
    let _ = tracing_subscriber::registry().with(fmt).try_init();
}

fn get_magic(network: Network) -> u64 {
    match network {
        Network::Testnet => 5,
        Network::Mainnet => 764824073,
        Network::Other(n) => n as u64,
    }
}
