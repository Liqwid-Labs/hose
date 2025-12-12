use pallas::crypto::key::ed25519::TryFromSecretKeyExtendedError;
use pallas::ledger::addresses::Network;
use thiserror::Error;

mod builder;
mod hd;
pub use builder::{AddressType, WalletBuilder};
pub use hd::{Bip32PrivateKey, Bip32PublicKey};

pub struct Wallet {
    network: Network,
    address: String,
    private_key: Bip32PrivateKey,
    /// Key used for signing/receiving transactions (derivation path: m/1852'/1815'/0'/0/address_index)
    payment_key: Bip32PrivateKey,
    /// Key used for receiving staking rewards (derivation path: m/1852'/1815'/0'/2/address_index)
    stake_key: Bip32PrivateKey,
}

impl Wallet {
    pub fn builder(network: Network) -> WalletBuilder {
        WalletBuilder::new(network)
    }

    pub fn network(&self) -> Network {
        self.network
    }

    pub fn address(&self) -> &str {
        &self.address
    }
}

#[derive(Error, Debug)]
pub enum Error {
    /// Private key wrapper data of unexpected length
    #[error("Wrapped private key data invalid length")]
    WrapperDataInvalidSize,
    /// Failed to decrypt private key wrapper data
    #[error("Failed to decrypt private key wrapper data")]
    WrapperDataFailedToDecrypt,
    /// Unexpected bech32 HRP prefix
    #[error("Unexpected bech32 HRP prefix")]
    InvalidBech32Hrp,
    /// Unable to decode bech32 string
    #[error("Unable to decode bech32: {0}")]
    InvalidBech32(#[from] bech32::DecodeError),
    /// Decoded bech32 data of unexpected length
    #[error("Decoded bech32 data of unexpected length")]
    UnexpectedBech32Length,
    /// Error relating to ed25519-bip32 private key
    #[error("Error relating to ed25519-bip32 private key: {0}")]
    Xprv(#[from] ed25519_bip32::PrivateKeyError),
    /// Error relating to bip39 mnemonic
    #[error("Error relating to bip39 mnemonic: {0}")]
    Mnemonic(bip39::Error),
    /// Error when attempting to derive ed25519-bip32 key
    #[error("Error when attempting to derive ed25519-bip32 key: {0}")]
    DerivationError(ed25519_bip32::DerivationError),
    /// Error that may occurs when trying to decrypt a private key
    /// which is not valid.
    #[error("Invalid Ed25519 Extended Secret Key: {0}")]
    InvalidSecretKeyExtended(#[from] TryFromSecretKeyExtendedError),
}
