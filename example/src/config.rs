use std::path::PathBuf;

use clap::Parser;
use hydrant::GenesisConfig;
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

    #[arg(long, env)]
    pub genesis_byron_path: Option<PathBuf>,

    #[arg(long, env)]
    pub genesis_shelley_path: Option<PathBuf>,
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

pub fn genesis_config(config: &Config) -> anyhow::Result<GenesisConfig> {
    let byron_str = std::fs::read_to_string(config.genesis_byron_path.as_ref().unwrap())?;
    let shelley_str = std::fs::read_to_string(config.genesis_shelley_path.as_ref().unwrap())?;
    GenesisConfig::new(Some(&byron_str), Some(&shelley_str)).map_err(anyhow::Error::from)
}
