use std::str::FromStr;
use std::sync::{Arc, Mutex};

use anyhow::{Context as _, Result};
use clap::Parser as _;
use hose::builder::{BuiltTx, Input, Output, TxBuilder};
use hose::ogmios::OgmiosClient;
use hose::ogmios::pparams::ProtocolParams;
use hose::ogmios::utxo::Utxo;
use hose::wallet::{Wallet, WalletBuilder};
use hydrant::UtxoIndexer;
use pallas::crypto::hash::Hash;
use pallas::ledger::addresses::{Address, Network, ShelleyAddress};
use pallas::ledger::primitives::NetworkId;
use pallas::txbuilder::BuiltTransaction;
use tokio::signal;
use tracing::instrument::WithSubscriber;
use tracing::{Level, error, info};
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

    let wallet =
        WalletBuilder::new(config.network.clone()).from_hex(config.private_key_hex.clone())?;

    info!("Wallet address: {}", wallet.address().to_bech32()?);

    let indexer = start_indexer(&config, wallet.address().clone())?;

    info!("Indexer started");

    let bech32_address = wallet.address().to_bech32()?;

    let utxos = ogmios.utxos_by_addresses(&[&bech32_address]).await?;

    info!("UTXOs: {:?}", utxos);

    let tx = create_collateral_tx(network_id, utxos, &wallet, &ogmios, &protocol_params).await?;
    let cbor = tx.cbor_hex();
    info!("CBOR: {:?}", cbor);

    let tx = tx.sign(&wallet)?;

    let res = ogmios.submit(&tx.cbor()).await?;

    info!("Submitted transaction: {:?}", res);

    Ok(())
}

async fn create_collateral_tx(
    network_id: NetworkId,
    utxos: Vec<Utxo>,
    wallet: &Wallet,
    ogmios: &OgmiosClient,
    protocol_params: &ProtocolParams,
) -> anyhow::Result<BuiltTx> {
    let utxo = utxos.first().context("no utxo found")?;
    let input = input_from_utxo(utxo.clone())?;
    let collateral_size = 10_000_000;
    let tx = TxBuilder::new(network_id)
        .add_input(input)
        .add_output(Output::new(
            Address::Shelley(wallet.address().clone()),
            utxo.value.lovelace - collateral_size,
        ))
        .add_output(Output::new(
            Address::Shelley(wallet.address().clone()),
            collateral_size,
        ))
        .build(ogmios, protocol_params)
        .await?;

    Ok(tx)
}

fn input_from_utxo(utxo: Utxo) -> anyhow::Result<Input> {
    let tx_hash = Hash::from_str(&utxo.transaction.id)?;
    Ok(Input::new(tx_hash, utxo.index as u64))
}

fn get_magic(network: Network) -> u64 {
    match network {
        Network::Testnet => 5,
        Network::Mainnet => 764824073,
        Network::Other(n) => n as u64,
    }
}

const MAX_ROLLBACK_BLOCKS: usize = 2160;
fn start_indexer(
    config: &config::Config,
    address: ShelleyAddress,
) -> anyhow::Result<Arc<Mutex<UtxoIndexer>>> {
    let magic = get_magic(config.network);
    let db = hydrant::Db::new(config.db_path.to_str().unwrap(), MAX_ROLLBACK_BLOCKS)?;
    let indexer = hydrant::UtxoIndexerBuilder::new("utxo")
        .address(address.to_vec())
        .build(&db.env)?;

    let indexer = Arc::new(Mutex::new(indexer));

    let idx_copy = indexer.clone();
    let config_copy = config.clone();

    tokio::spawn(async move {
        info!("Connecting to node...");
        let node = pallas::network::facades::PeerClient::connect(config_copy.node_host, magic)
            .await
            .expect("failed to connect to node");

        // Listen for chain-sync events until shutdown or error
        info!("Starting sync...");
        let mut sync = hydrant::Sync::new(node, &db, &vec![idx_copy])
            .await
            .expect("failed to start sync");
        let sync_result = tokio::select! {
            res = sync.run() => res,
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
    });

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
