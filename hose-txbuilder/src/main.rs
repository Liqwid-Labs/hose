use crate::config::Config;
use anyhow::Context;
use betterfrost_client::v0::Client;

use hose_submission::NodeClient;
use hose_submission::OgmiosClient;
use hose_submission::SubmitTx;
use pallas::ledger::primitives::{conway::Tx, Fragment};
use simple_tx::simple_transaction;
use simple_tx::TargetUser;
use sqlx::postgres::PgPoolOptions;

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

    // TODO: Make this work again

    Ok(())
}
