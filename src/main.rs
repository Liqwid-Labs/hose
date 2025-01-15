use crate::config::Config;
use anyhow::Context;
use betterfrost_client::Client;
use clap::Parser;
use pallas_primitives::Fragment;
use simple_tx::simple_transaction;
use simple_tx::TargetUser;
use submission::submit_transaction;

mod config;
mod simple_tx;
mod submission;

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
    let db = sqlx::postgres::PgPoolOptions::new()
        .max_connections(50)
        .connect(&config.database_url)
        .await
        .context("could not connect to database")?;
    let utxo_db = sqlx::postgres::PgPoolOptions::new()
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

    let target_user = TargetUser::from_local_config(&config)?;

    let tx = simple_transaction(&client, target_user.clone(), &config)
        .await
        .expect("Could not create transaction");

    let tx = tx.sign(private_key).expect("Could not sign transaction");

    let conway_tx = pallas_primitives::conway::Tx::decode_fragment(&tx.tx_bytes.0).expect("ok");

    println!("{:?}", hex::encode(&minicbor::to_vec(&conway_tx)?));

    submit_transaction(
        &config,
        &client,
        hex::encode(tx.tx_hash.0),
        &minicbor::to_vec(&conway_tx)?,
    )
    .await?;

    Ok(())
}
