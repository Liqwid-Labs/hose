use std::str::FromStr;

use clap::Parser;
use pallas::crypto::hash::Hash;
use pallas::ledger::addresses::Network;
use pallas::txbuilder::{BuildConway, Input, Output};
use reqwest::Url;
use tracing::info;

use crate::ogmios::OgmiosClient;
use crate::ogmios::utxo::Utxo;
use crate::wallet::PrivateKey;

mod builder;
mod ogmios;
mod wallet;

#[derive(clap::Parser)]
struct Options {
    /// Example: ed25519_sk1... (Bech32 encoded)
    #[arg(long, env)]
    faucet_private_key: String,
    /// Example: http://localhost:1337
    #[arg(long, env)]
    ogmios_url: String,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();

    dotenv::dotenv().ok();

    let options = Options::try_parse()?;

    let ogmios_client = OgmiosClient::new(Url::parse(&options.ogmios_url).unwrap());

    let faucet_private_key_str = options.faucet_private_key;
    let faucet_private_key = PrivateKey::from_bech32(&faucet_private_key_str)?;

    let addr = faucet_private_key.address_testnet();

    let utxos = ogmios_client
        .utxos_by_addresses(&[&addr.to_bech32().unwrap()])
        .await?;

    info!("UTXOs: {:?}", utxos);

    let fee = 10000000;

    // FIXME: Obviously we need to determine how much lovelace is at the UTxO automatically.
    let utxo_existing_amount = 999999999990000000;

    // Simple test transaction
    let tx = pallas::txbuilder::StagingTransaction::new()
        .input(input_from_utxo(&utxos[0]))
        .output(Output::new(addr.clone(), utxo_existing_amount - fee))
        .change_address(addr.clone())
        .collateral_input(input_from_utxo(&utxos[0]))
        .fee(fee)
        .network_id(Network::Testnet.value());

    let built = tx.build_conway_raw()?;

    let built = faucet_private_key.sign_tx(built)?;

    info!("Signed transaction: {}", hex::encode(&built.tx_bytes.0));

    let res = ogmios_client.submit(built.tx_bytes.0.as_slice()).await?;
    info!("Submitted transaction: {:?}", res);

    Ok(())
}

fn input_from_utxo(utxo: &Utxo) -> Input {
    let bs = hex::decode(&utxo.transaction.id).unwrap();
    let bs: [u8; 32] = bs[..32].try_into().expect("Invalid transaction hash");
    let hash = Hash::new(bs);
    Input::new(hash, utxo.index as u64)
}
