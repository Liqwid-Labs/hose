#[derive(Debug, Clone, PartialEq, thiserror::Error)]
pub enum TxBuilderError {
    /// Provided bytes could not be decoded into a script
    #[error("Transaction has no inputs")]
    MalformedScript,
    /// Provided bytes could not be decoded into a datum
    #[error("Could not decode datum bytes")]
    MalformedDatum,
    /// Input, policy, etc pointed to by a redeemer was not found in the
    /// transaction
    #[error("Input/policy pointed to by redeemer not found in tx")]
    RedeemerTargetMissing,
    /// Provided network ID is invalid (must be 0 or 1)
    #[error("Invalid network ID")]
    InvalidNetworkId,
    /// Transaction bytes in built transaction object could not be decoded
    #[error("Corrupted transaction bytes in built transaction")]
    CorruptedTxBytes,
    /// Public key generated from private key was of unexpected length
    #[error("Public key for private key is malformed")]
    MalformedKey,
    /// Asset name is too long, it must be 32 bytes or less
    #[error("Asset name must be 32 bytes or less")]
    AssetNameTooLong,
    /// Unsupported era
    #[error("Unsupported era")]
    UnsupportedEra,
    /// Registration deposit missing
    #[error("Missing stake credential deposit")]
    MissingStakeCredentialDeposit,
    /// Mint/burn amount is out of range
    #[error("Invalid mint amount")]
    InvalidMintAmount,
    /// Native scripts do not take redeemers
    #[error("Cannot use redeemers with native scripts")]
    RedeemerForNativeScript,
    /// Plutus scripts need a redeemer
    #[error("Plutus scripts must always take a redeemer")]
    RedeemerMissing,
    #[error(
        "Validity interval is disjoint with existing interval, making transaction invalid. This is likely a bug in your code."
    )]
    InvalidValidityInterval,
}
