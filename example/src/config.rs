use std::path::PathBuf;

use clap::{Parser, ValueEnum};
use pallas::ledger::addresses::Network;

#[derive(Parser, Debug, PartialEq, Eq, Clone)]
pub struct Config {
    #[arg(long, env)]
    pub private_key_hex: String,

    #[arg(long, env, value_parser = parse_network)]
    pub network: Network,

    #[arg(long, env)]
    pub db_path: PathBuf,

    #[arg(long, env)]
    pub node_host: String,

    #[arg(long, env)]
    pub ogmios_url: String,
}

fn parse_network(s: &str) -> Result<Network, String> {
    let s = s.to_lowercase();
    match s.as_ref() {
        "testnet" => Ok(Network::Testnet),
        "mainnet" => Ok(Network::Mainnet),
        s => s
            .parse::<u8>()
            .map(|n| Network::Other(n))
            .map_err(|_| format!("Invalid network: {}", s)),
    }
}
