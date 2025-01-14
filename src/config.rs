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
pub struct Config {
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

    /// The password for the wallet to use for signing transactions
    #[arg(long, env)]
    pub wallet_password: String,

    /// The address for the wallet to use for signing transactions
    #[arg(long, env)]
    pub wallet_address: String,
}
