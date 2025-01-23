use crate::config::Config;
use anyhow::Context;
use betterfrost_client::Client;
use hose_submission::NodeClient;
use hose_submission::OgmiosClient;
use pallas::ledger::primitives::{conway::Tx, Fragment};
use simple_tx::simple_transaction;
use simple_tx::TargetUser;
use sqlx::postgres::PgPoolOptions;
use hose_submission::SubmitTx;

mod config;
mod simple_tx;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // This returns an error if the `.env` file doesn't exist, but that's not what we want
    // since we're not going to use a `.env` file if we deploy this application
    dotenv::dotenv().ok();

    // Initialize logger
    env_logger::init();

    // Parse our configuration from the environment
    // This will exit with a help message if something is wrong
    let config = Config::parse()?;

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

    let target_user = TargetUser::from_local_config(&config)?;

    let tx = simple_transaction(&client, target_user.address.clone(), &config)
        .await
        .expect("Could not create transaction");

    let tx = tx.sign(config.wallet_payment_key.to_ed25519_private_key())?;

    let conway_tx = Tx::decode_fragment(&tx.tx_bytes.0).expect("ok");

    // Alternatively, we can submit the transaction directly to the node
    let mut direct_to_node = NodeClient::new(config.network, "/tmp/node.socket".into(), &client);

    let result = if let Some(ogmios_url) = config.ogmios_url.clone() {
        let mut ogmios = OgmiosClient::new(&ogmios_url).await?;

        ogmios
            .submit_tx(&minicbor::to_vec(&conway_tx)?)
            .await?
    } else {
        direct_to_node
            .submit_tx(&minicbor::to_vec(&conway_tx)?)
            .await?
    };

    println!("Submission result: {:?}", result);

    Ok(())
}
