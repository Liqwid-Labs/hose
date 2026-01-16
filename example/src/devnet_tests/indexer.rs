use std::sync::{Arc, LazyLock};
use std::time::Duration;

use anyhow::{Context, Result};
use hydrant::{GenesisConfig, UtxoIndexer};
use pallas::ledger::addresses::{Network, ShelleyAddress};
use tokio::signal;
use tokio::sync::{Mutex, MutexGuard};
use tokio::task::JoinHandle;
use tracing::{error, info, warn};

use crate::config;

/// For use in test contexts. We should only construct this once, so we can use std::sync::LazyLock to ensure that.
pub struct IndexerContext {
    pub indexer: Arc<Mutex<UtxoIndexer>>,
    _sync_handle: JoinHandle<()>,
}

static INDEXER_CONTEXT: LazyLock<Arc<Mutex<Option<IndexerContext>>>> =
    LazyLock::new(|| Arc::new(Mutex::new(None)));

impl IndexerContext {
    pub async fn acquire_indexer(
        config: &config::Config,
        address: ShelleyAddress,
    ) -> anyhow::Result<Arc<Mutex<UtxoIndexer>>> {
        // Check if we need to initialize, but don't hold the lock across await
        let needs_init = {
            let context = INDEXER_CONTEXT.lock().await;
            context.is_none()
        };

        if needs_init {
            let (indexer, sync_handle) = sync_indexer(config, address).await?;
            let mut context = INDEXER_CONTEXT.lock().await;
            // Double-check pattern: another thread might have initialized it
            if context.is_none() {
                *context = Some(IndexerContext {
                    indexer,
                    _sync_handle: sync_handle,
                });
            }
        }

        let ctx = INDEXER_CONTEXT.lock().await;

        if let Some(ctx) = &*ctx {
            return Ok(ctx.indexer.clone());
        } else {
            anyhow::bail!("Indexer context not found, but we just initialized it");
        }
    }
}

pub async fn sync_indexer(
    config: &config::Config,
    address: ShelleyAddress,
) -> anyhow::Result<(Arc<Mutex<UtxoIndexer>>, JoinHandle<()>)> {
    fn get_magic(network: Network) -> u64 {
        match network {
            Network::Testnet => 5,
            Network::Mainnet => 764824073,
            Network::Other(n) => n as u64,
        }
    }
    let db = hydrant::Db::new(config.db_path.to_str().unwrap())?;

    let indexer = UtxoIndexer::builder()
        .address(address.to_vec())
        .build(&db.env)?;
    let indexer = Arc::new(Mutex::new(indexer));

    info!("Connecting to node...");
    let magic = get_magic(config.network);
    let node = pallas::network::facades::PeerClient::connect(&config.node_host, magic)
        .await
        .expect("failed to connect to node");

    let genesis_config = config::genesis_config(config)?;

    let idxr_copy = indexer.clone();

    let mut sync = hydrant::Sync::new(node, &db, &vec![idxr_copy.clone()], genesis_config)
        .await
        .expect("failed to start sync");
    let sync_result = sync.run_until_synced().await;

    if let Err(error) = sync_result {
        error!(?error, "Error while syncing to tip");
    }
    db.persist().expect("failed to persist database");

    info!("Starting background sync...");
    let sync_handle = tokio::task::spawn(async move {
        if let Err(error) = sync.run().await {
            error!(?error, "Error while syncing to tip");
        }
        db.persist().expect("failed to persist database");
        warn!("Sync handle finished.");
    });

    Ok((indexer, sync_handle))
}
