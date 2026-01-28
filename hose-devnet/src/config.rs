use std::path::PathBuf;

use anyhow::Context as _;
use clap::Parser;
use hydrant::GenesisConfig;
use pallas::ledger::addresses::Network;

#[derive(Parser, Debug, PartialEq, Eq, Clone)]
pub struct Config {
    #[arg(long, env)]
    pub private_key_hex: String,

    /// The network to use. Either `testnet` or `mainnet`. For devnet tests, you should set this to `testnet`.
    #[arg(long, env, value_parser = parse_network)]
    pub network: Network,

    #[arg(long, env)]
    pub db_path: PathBuf,

    #[arg(long, env)]
    pub node_host: String,

    #[arg(long, env)]
    pub ogmios_url: String,

    /// Path to the byron genesis file. This file can be found in the local-testnet repository.
    #[arg(long, env)]
    pub genesis_byron_path: Option<PathBuf>,

    /// Path to the shelley genesis file. This file can be found in the local-testnet repository.
    #[arg(long, env)]
    pub genesis_shelley_path: Option<PathBuf>,
}

fn parse_network(s: &str) -> Result<Network, String> {
    let s = s.to_lowercase();
    match s.as_ref() {
        "testnet" => Ok(Network::Testnet),
        "mainnet" => Ok(Network::Mainnet),
        s => s.parse::<u8>().map(Network::Other).map_err(|_| {
            format!(
                "Invalid network: {}, valid networks are: `testnet`, `mainnet`",
                s
            )
        }),
    }
}

const GENESIS_BYRON_PATH_ERROR: &str = concat!(
    "Genesis byron path is required, for devnet tests, you should set the GENESIS_BYRON_PATH environment variable.",
    " You can find this file in the local-testnet repository, inside of `config`."
);

const GENESIS_SHELLEY_PATH_ERROR: &str = concat!(
    "Genesis shelley path is required, for devnet tests, you should set the GENESIS_SHELLEY_PATH environment variable.",
    " You can find this file in the local-testnet repository, inside of `config`."
);

pub fn genesis_config(config: &Config) -> anyhow::Result<GenesisConfig> {
    let byron_str = std::fs::read_to_string(
        config
            .genesis_byron_path
            .as_ref()
            .context(GENESIS_BYRON_PATH_ERROR)?,
    )?;
    let shelley_str = std::fs::read_to_string(
        config
            .genesis_shelley_path
            .as_ref()
            .context(GENESIS_SHELLEY_PATH_ERROR)?,
    )?;
    GenesisConfig::new(Some(&byron_str), Some(&shelley_str)).map_err(anyhow::Error::from)
}
