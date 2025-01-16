use bip32::ChildNumber;
use pallas::{
    applying::MultiEraProtocolParameters, ledger::primitives::NetworkId,
    wallet::keystore::hd::Bip32PrivateKey,
};
use std::str::FromStr;

use clap::Parser;

use crate::params::get_protocol_parameters;

/// Represents the network to use
#[derive(Debug, Clone)]
pub struct Network(NetworkId);

impl Network {
    pub fn network_magic(&self) -> u32 {
        match self.0 {
            NetworkId::Mainnet => 764824073,
            NetworkId::Testnet => 2,
        }
    }
}

impl Into<NetworkId> for Network {
    fn into(self) -> NetworkId {
        self.0
    }
}

impl From<Network> for u8 {
    fn from(val: Network) -> Self {
        match val.0 {
            NetworkId::Mainnet => 1,
            NetworkId::Testnet => 0,
        }
    }
}

impl FromStr for Network {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "Mainnet" => Ok(Network(NetworkId::Mainnet)),
            "Testnet" => Ok(Network(NetworkId::Testnet)),
            _ => Err(format!("unknown network {}", s)),
        }
    }
}

/// The configuration parameters for the application.
///
/// These can either be passed on the command line, or pulled from environment variables.
/// The latter is preferred as environment variables are one of the recommended ways to
/// get configuration from Kubernetes Secrets in deployment.
///
/// For development convenience, these can also be read from a `.env` file in the working
/// directory where the application is started.
///
/// See `.env.sample` in the repository root for details.
#[derive(Parser)]
struct ConfigInput {
    /// The connection URL for the Postgres database this application should use.
    /// This should be an instance of cardano-db-sync with `conumed_by_tx_id`
    /// via the `tx_out.value = 'consumed'` config option.
    #[arg(long, env)]
    pub database_url: String,

    /// The connection URL for the UTXO Postgres database this application should use.
    /// This should be an instance of cardano-db-sync with the `utxo_only` preset
    #[arg(long, env)]
    pub utxo_database_url: String,

    /// The mnemonic for the wallet to use for signing transactions
    #[arg(long, env)]
    pub wallet_mnemonic: String,

    /// The address for the wallet to use for signing transactions
    #[arg(long, env)]
    pub wallet_address: String,

    /// The network to use
    #[arg(long, env, value_parser = clap::value_parser!(Network))]
    pub network: Network,

    /// Ogmios URL
    #[arg(long, env)]
    pub ogmios_url: Option<String>,
}

pub struct Config {
    /// The connection URL for the Postgres database this application should use.
    /// This should be an instance of cardano-db-sync with `conumed_by_tx_id`
    /// via the `tx_out.value = 'consumed'` config option.
    pub database_url: String,

    /// The connection URL for the UTXO Postgres database this application should use.
    /// This should be an instance of cardano-db-sync with the `utxo_only` preset
    pub utxo_database_url: String,

    /// The mnemonic for the wallet to use for signing transactions
    pub wallet_payment_key: Bip32PrivateKey,

    /// The address for the wallet to use for signing transactions
    pub wallet_address: String,

    /// The network to use
    pub network: Network,

    /// The protocol parameters
    pub protocol_params: MultiEraProtocolParameters,
}

impl Config {
    pub fn parse() -> anyhow::Result<Self> {
        let config = ConfigInput::parse();
        let protocol_params = get_protocol_parameters(config.network.0)?;
        let payment_key = Self::load_private_key_from_mnemonic(config.wallet_mnemonic.clone())?;

        Ok(Self {
            database_url: config.database_url,
            utxo_database_url: config.utxo_database_url,
            wallet_payment_key: payment_key,
            wallet_address: config.wallet_address.clone(),
            network: config.network,
            protocol_params,
        })
    }

    fn load_private_key_from_mnemonic(mnemonic: String) -> anyhow::Result<Bip32PrivateKey> {
        let private_key = Bip32PrivateKey::from_bip39_mnenomic(mnemonic, "".into())?;

        // https://cardano.stackexchange.com/questions/7671/what-is-the-derivation-path-in-a-cardano-address
        let account_key = private_key
            .derive(ChildNumber::HARDENED_FLAG + 1852)
            .derive(ChildNumber::HARDENED_FLAG + 1815)
            .derive(ChildNumber::HARDENED_FLAG + 0);

        let payment_key = account_key.derive(0).derive(0);

        Ok(payment_key)
    }
}
