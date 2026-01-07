use pallas::crypto::key::ed25519::{self, TryFromSecretKeyExtendedError};
use pallas::ledger::addresses::{Network, ShelleyAddress};
use pallas::ledger::primitives::{Fragment as _, conway};
use thiserror::Error;

use crate::builder::transaction::model::BuiltTransaction;

mod builder;
mod hd_key;
mod key;
pub use builder::{AddressType, WalletBuilder};
pub use hd_key::HDPrivateKey;
pub use key::PrivateKey;

use crate::builder::BuiltTx;

pub struct Wallet {
    network: Network,
    address: ShelleyAddress,
    /// Key used for signing/receiving transactions (derivation path: m/1852'/1815'/0'/0/address_index)
    payment_key: PrivateKey,
    /// Key used for receiving staking rewards (derivation path: m/1852'/1815'/0'/2/address_index)
    stake_key: Option<PrivateKey>,
}

impl Wallet {
    pub fn builder(network: Network) -> WalletBuilder {
        WalletBuilder::new(network)
    }

    pub fn network(&self) -> Network {
        self.network
    }

    pub fn address(&self) -> &ShelleyAddress {
        &self.address
    }

    pub fn public_key(&self) -> ed25519::PublicKey {
        self.payment_key.public_key()
    }

    pub fn sign(&self, tx: &BuiltTransaction) -> anyhow::Result<BuiltTransaction> {
        let signature = self.payment_key.sign(tx.tx_hash.0);
        let signature = signature.as_ref().try_into().unwrap();
        let tx = tx.clone().add_signature(self.public_key(), signature)?;
        Ok(tx)
    }
}

#[derive(Error, Debug)]
pub enum Error {
    /// Unexpected bech32 HRP prefix
    #[error("Unexpected bech32 HRP prefix: {0}")]
    InvalidBech32Hrp(String),
    /// Unable to decode bech32 string
    #[error("Unable to decode bech32: {0}")]
    InvalidBech32(#[from] bech32::DecodeError),
    /// Decoded data of unexpected length
    #[error("Decoded data of unexpected length")]
    UnexpectedKeyLength,
    /// Unable to decode hex string
    #[error("Unable to decode hex: {0}")]
    InvalidHex(#[from] hex::FromHexError),
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
