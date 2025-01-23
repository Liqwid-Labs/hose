use bip32::ChildNumber;
use hose_primitives::NetworkId;
use pallas::wallet::keystore::hd::Bip32PrivateKey;

use clap::Parser;

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
    #[arg(long, env, value_parser = clap::value_parser!(NetworkId))]
    pub network: NetworkId,

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
    pub network: NetworkId,

    /// Ogmios url
    pub ogmios_url: Option<String>,
}

impl Config {
    pub fn parse() -> anyhow::Result<Self> {
        let config = ConfigInput::parse();
        let payment_key = Self::load_private_key_from_mnemonic(config.wallet_mnemonic.clone())?;

        Ok(Self {
            database_url: config.database_url,
            utxo_database_url: config.utxo_database_url,
            wallet_payment_key: payment_key,
            wallet_address: config.wallet_address.clone(),
            network: config.network,
            ogmios_url: config.ogmios_url,
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
