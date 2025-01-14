use crate::config::Config;
use anyhow::Context;
use betterfrost_client::Client;
use clap::Parser;
use pallas_txbuilder::StagingTransaction;
use sqlx::postgres::PgPoolOptions;

mod config;

#[derive(Debug)]
pub enum Error {
    ClientError(betterfrost_client::Error),
}

impl From<betterfrost_client::Error> for Error {
    fn from(e: betterfrost_client::Error) -> Self {
        Self::ClientError(e)
    }
}

pub type Result<T, E = Error> = std::result::Result<T, E>;

async fn simple_transaction(client: &Client, config: &Config) -> Result<StagingTransaction> {
    let tx = StagingTransaction::new();

    let own_utxos = client
        .address_utxos(config.wallet_address.clone(), Default::default())
        .await?;

    println!("{:?}", own_utxos);

    Ok(tx)
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // This returns an error if the `.env` file doesn't exist, but that's not what we want
    // since we're not going to use a `.env` file if we deploy this application
    dotenv::dotenv().ok();

    // Initialize logger
    env_logger::init();

    // Parse our configuration from the environment
    // This will exit with a help message if something is wrong
    let config = Config::parse();

    // We create a single connection pool for SQLx that's shared across the whole application
    // This saves us from opening a new connection for every API call, which is wasteful
    let db = PgPoolOptions::new()
        .max_connections(50)
        .connect(&config.database_url)
        .await
        .context("could not connect to database")?;
    let utxo_db = PgPoolOptions::new()
        .max_connections(50)
        .connect(&config.utxo_database_url)
        .await
        .context("could not connect to utxo database")?;

    let client = Client::new(db, utxo_db);

    let private_key = pallas_wallet::hd::Bip32PrivateKey::from_bip39_mnenomic(
        config.wallet_mnemonic.clone(),
        config.wallet_password.clone(),
    )?
    .to_ed25519_private_key();

    let wallet_utxos = client
        .address_utxos(config.wallet_address.clone(), Default::default())
        .await
        .expect("could not get wallet utxos");

    println!("{:?}", private_key.public_key());
    println!("{:?}", wallet_utxos);

    simple_transaction(&client, &config)
        .await
        .expect("Could not create transaction");

    Ok(())
}
