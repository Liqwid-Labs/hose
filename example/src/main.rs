use std::sync::{Arc, Mutex};

use anyhow::{Context, Result};
use clap::Parser;
use hose::builder::{BuiltTx, TxBuilder};
use hose::ogmios::OgmiosClient;
use hose::ogmios::pparams::ProtocolParams;
use hose::primitives::Output;
use hose::wallet::{Wallet, WalletBuilder};
use hydrant::UtxoIndexer;
use pallas::ledger::addresses::{Address, Network, ShelleyAddress};
use pallas::ledger::primitives::NetworkId;
use tokio::signal;
use tracing::{error, info};
use url::Url;

pub mod config;

#[tokio::main]
pub async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();
    dotenv::dotenv().context("could not load .env file")?;

    let config = config::Config::parse();
    let network_id = NetworkId::try_from(config.network.value())
        .expect("failed to convert network to network id");

    let ogmios = OgmiosClient::new(Url::parse(&config.ogmios_url)?);

    let protocol_params = ogmios.protocol_params().await?;
    info!("Protocol params: {:?}", protocol_params);

    let wallet = WalletBuilder::new(config.network).from_hex(config.private_key_hex.clone())?;
    info!("Wallet address: {}", wallet.address().to_bech32()?);

    info!("Starting indexer sync...");
    let indexer = sync_indexer(&config, wallet.address().clone()).await?;
    let indexer = indexer.lock().expect("indexer lock poisoned");
    println!("Indexer synced");

    let utxos = indexer.address_utxos(&wallet.address().to_vec())?;
    info!("UTXOs: {:?}", utxos);

    let tx = create_collateral_tx(network_id, &wallet, &indexer, &ogmios, &protocol_params).await?;
    let cbor = tx.cbor_hex();
    info!("CBOR: {:?}", cbor);

    let tx = tx.sign(&wallet)?;

    let res = ogmios.submit(&tx.cbor()).await?;

    info!("Submitted transaction: {:?}", res);

    Ok(())
}

async fn create_collateral_tx(
    network_id: NetworkId,
    wallet: &Wallet,
    indexer: &UtxoIndexer,
    ogmios: &OgmiosClient,
    protocol_params: &ProtocolParams,
) -> anyhow::Result<BuiltTx> {
    let collateral_size = 10_000_000;
    let change_address = wallet.address().clone();
    let tx = TxBuilder::new(network_id)
        .change_address(Address::Shelley(change_address))
        .add_output(Output::new(
            Address::Shelley(wallet.address().clone()),
            collateral_size,
        ))
        .build(indexer, ogmios, protocol_params)
        .await?;

    Ok(tx)
}

fn get_magic(network: Network) -> u64 {
    match network {
        Network::Testnet => 5,
        Network::Mainnet => 764824073,
        Network::Other(n) => n as u64,
    }
}

const MAX_ROLLBACK_BLOCKS: usize = 2160;
async fn sync_indexer(
    config: &config::Config,
    address: ShelleyAddress,
) -> anyhow::Result<Arc<Mutex<UtxoIndexer>>> {
    let db = hydrant::Db::new(config.db_path.to_str().unwrap(), MAX_ROLLBACK_BLOCKS)?;

    let indexer = UtxoIndexer::builder()
        .address(address.to_vec())
        .build(&db.env)?;
    let indexer = Arc::new(Mutex::new(indexer));

    info!("Connecting to node...");
    let magic = get_magic(config.network);
    let node = pallas::network::facades::PeerClient::connect(&config.node_host, magic)
        .await
        .expect("failed to connect to node");

    // Listen for chain-sync events until shutdown or reached tip
    info!("Starting sync...");
    let mut sync = hydrant::Sync::new(node, &db, &vec![indexer.clone()])
        .await
        .expect("failed to start sync");
    let sync_result = tokio::select! {
        res = sync.run_until_synced() => res,
        res = shutdown_signal() => {
            tracing::info!("Received shutdown signal");
            res
        }
    };
    if let Err(error) = sync_result {
        error!(?error, "Error while syncing");
    }

    info!("Stopping sync...");
    if let Err(error) = sync.stop().await {
        error!(?error, "Error while writing");
    }

    info!("Persisting database...");
    db.persist().expect("failed to persist database");

    Ok(indexer)
}

async fn shutdown_signal() -> Result<()> {
    let ctrl_c = async {
        signal::ctrl_c()
            .await
            .context("failed to install Ctrl+C handler")
    };

    #[cfg(unix)]
    let terminate = async {
        signal::unix::signal(signal::unix::SignalKind::terminate())
            .context("failed to install signal handler")?
            .recv()
            .await;
        Ok(())
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        res = ctrl_c => res,
        res = terminate => res,
    }
}
