use crate::config::Config;
use anyhow::Context;
use betterfrost_client::Client;
use bip32::ChildNumber;
use clap::Parser;
use pallas::ledger::primitives::{conway::Tx, Fragment};
use pallas::ledger::traverse::ComputeHash;
use pallas::wallet::keystore::hd::Bip32PrivateKey;
use simple_tx::simple_transaction;
use simple_tx::TargetUser;
use sqlx::postgres::PgPoolOptions;
use submission::direct_to_node::DirectToNode;
use submission::ogmios::OgmiosClient;
use submission::SubmitTx;

mod config;
mod params;
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

    let private_key =
        Bip32PrivateKey::from_bip39_mnenomic(config.wallet_mnemonic.clone(), "".into())?;

    // https://cardano.stackexchange.com/questions/7671/what-is-the-derivation-path-in-a-cardano-address
    let account_key = private_key
        .derive(ChildNumber::HARDENED_FLAG + 1852)
        .derive(ChildNumber::HARDENED_FLAG + 1815)
        .derive(ChildNumber::HARDENED_FLAG + 0);

    let payment_key = account_key.derive(0).derive(0);

    let target_user = TargetUser::from_local_config(&config)?;

    let tx = simple_transaction(&client, target_user.clone(), &config)
        .await
        .expect("Could not create transaction");

    let tx = tx
        .sign(payment_key.to_ed25519_private_key())
        .expect("Could not sign transaction");

    let conway_tx = Tx::decode_fragment(&tx.tx_bytes.0).expect("ok");

    println!("{:?}", hex::encode(&minicbor::to_vec(&conway_tx)?));

    // Alternatively, we can submit the transaction directly to the node
    let mut direct_to_node = DirectToNode::new(&config, &client);

    let ogmios = OgmiosClient::new(&config, "ws://mainnet-ogmios:1337").await?;

    let mut client_to_use = ogmios;

    // direct_to_node
    //     .submit_tx(hex::encode(tx.tx_hash.0), &minicbor::to_vec(&conway_tx)?)
    //     .await?;

    Ok(())
}
