use crate::config::Config;
use betterfrost_client::addresses as betterfrost;
use betterfrost_client::Client;
use clap::Parser;
use pallas_addresses::Address;
use pallas_addresses::PaymentKeyHash;
use pallas_primitives::Fragment;
use pallas_txbuilder::BuildConway;
use pallas_txbuilder::BuiltTransaction;
use pallas_txbuilder::Input;
use pallas_txbuilder::Output;
use pallas_txbuilder::StagingTransaction;

mod config;
mod submission;

#[derive(Debug)]
pub enum Error {
    ClientError(betterfrost_client::Error),
    TxBuilderError(pallas_txbuilder::TxBuilderError),
    AddressError(pallas_addresses::Error),
}

impl From<pallas_addresses::Error> for Error {
    fn from(e: pallas_addresses::Error) -> Self {
        Self::AddressError(e)
    }
}

impl From<betterfrost_client::Error> for Error {
    fn from(e: betterfrost_client::Error) -> Self {
        Self::ClientError(e)
    }
}

impl From<pallas_txbuilder::TxBuilderError> for Error {
    fn from(e: pallas_txbuilder::TxBuilderError) -> Self {
        Self::TxBuilderError(e)
    }
}

pub type Result<T, E = Error> = std::result::Result<T, E>;

fn utxo_lovelace(utxo: &betterfrost::AddressUtxo) -> u64 {
    utxo.amount
        .iter()
        .filter(|amount| amount.0.unit == "lovelace")
        .map(|amount| amount.0.quantity.parse::<u64>().unwrap())
        .sum()
}

fn address_utxo_to_input(utxo: &betterfrost::AddressUtxo) -> Input {
    Input {
        tx_hash: serde_json::from_value(serde_json::Value::String(utxo.tx_hash.clone()))
            .expect("to parse tx_hash"),
        txo_index: utxo
            .tx_index
            .abs()
            .try_into()
            .expect("Extending i16 to u64"),
    }
}

fn address_utxo_to_output(utxo: &betterfrost::AddressUtxo) -> Output {
    Output::new(
        Address::from_bech32(&utxo.address.clone()).expect("to parse address"),
        utxo.amount
            .iter()
            .map(|x| x.0.quantity.parse::<u64>().unwrap())
            .sum(),
    )
}

fn address_payment_key_hash(address: &Address) -> PaymentKeyHash {
    match address {
        Address::Shelley(x) => *x.payment().as_hash(),
        Address::Stake(x) => *x.payload().as_hash(),
        _ => panic!("not a payment address"),
    }
}

async fn simple_transaction(client: &Client, config: &Config) -> Result<BuiltTransaction> {
    let tx = StagingTransaction::new();

    let address = Address::from_bech32(&config.wallet_address.clone())?;

    let own_utxos = client
        .address_utxos(config.wallet_address.clone(), Default::default())
        .await?;

    // TODO: estimate the fee
    let fee = 2_000_000;

    let valid_fee_utxos = own_utxos
        .iter()
        .filter(|utxo| utxo_lovelace(utxo) >= fee)
        .collect::<Vec<_>>();

    let tx = tx.fee(fee);

    let utxo_to_spend = valid_fee_utxos.first().unwrap();

    let tx = tx.input(address_utxo_to_input(utxo_to_spend));

    let o1 = Output::new(
        Address::from_bech32(&config.wallet_address.clone())?,
        utxo_lovelace(utxo_to_spend) - fee,
    );

    let tx = tx.output(o1.clone());

    let tx = tx.change_address(address.clone());

    let tx = tx.network_id(0);

    let tx = tx.collateral_input(address_utxo_to_input(utxo_to_spend));

    let tx = tx.collateral_output(o1);

    let tx = tx.disclosed_signer(address_payment_key_hash(&address));

    println!("{:?}", own_utxos);

    let built_tx = tx.build_conway_raw()?;

    Ok(built_tx)
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

    let tx = simple_transaction(&client, &config)
        .await
        .expect("Could not create transaction");

    let tx = tx.sign(private_key).expect("Could not sign transaction");

    let conway_tx = pallas_primitives::conway::Tx::decode_fragment(&tx.tx_bytes.0).expect("ok");

    println!("{:?}", conway_tx);

    println!("{:?}", hex::encode(&minicbor::to_vec(&conway_tx)?));

    Ok(())
}
