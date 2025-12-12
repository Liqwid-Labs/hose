use bech32::{FromBase32, ToBase32};
use bip39::rand_core::{CryptoRng, RngCore};
use bip39::{Language, Mnemonic};
use cryptoxide::{hmac::Hmac, pbkdf2::pbkdf2, sha2::Sha512};
use ed25519_bip32::{XPRV_SIZE, XPrv, XPub};
use pallas::crypto::key::ed25519::{
    self, PublicKey, SecretKey, SecretKeyExtended, Signature, TryFromSecretKeyExtendedError,
};
use pallas::ledger::addresses::{
    Address, Network, PaymentKeyHash, ShelleyAddress, ShelleyDelegationPart, ShelleyPaymentPart,
};
use pallas::ledger::traverse::ComputeHash;
use pallas::txbuilder::TxBuilderError;
use thiserror::Error;

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
    InvalidBech32(bech32::Error),
    /// Decoded bech32 data of unexpected length
    #[error("Decoded bech32 data of unexpected length")]
    UnexpectedBech32Length,
    /// Error relating to ed25519-bip32 private key
    #[error("Error relating to ed25519-bip32 private key: {0}")]
    Xprv(ed25519_bip32::PrivateKeyError),
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

/// A standard or extended Ed25519 secret key
pub enum PrivateKey {
    Normal(SecretKey),
    Extended(SecretKeyExtended),
}

impl PrivateKey {
    const BECH32_HRP: &'static str = "ed25519_sk";

    pub fn from_bech32(bech32_str: &str) -> Result<Self, Error> {
        let (hrp, data, _) = bech32::decode(bech32_str).map_err(Error::InvalidBech32)?;
        if hrp != Self::BECH32_HRP {
            return Err(Error::InvalidBech32Hrp);
        }
        let bytes = Vec::<u8>::from_base32(&data).map_err(Error::InvalidBech32)?;

        // Try extended key first (64 bytes)
        if bytes.len() == SecretKeyExtended::SIZE {
            let key_bytes: [u8; SecretKeyExtended::SIZE] = bytes
                .try_into()
                .map_err(|_| Error::UnexpectedBech32Length)?;
            let secret_key = SecretKeyExtended::from_bytes(key_bytes)?;
            return Ok(PrivateKey::Extended(secret_key));
        }

        // Try standard key (32 bytes)
        if bytes.len() == SecretKey::SIZE {
            let key_bytes: [u8; SecretKey::SIZE] = bytes
                .try_into()
                .map_err(|_| Error::UnexpectedBech32Length)?;
            let secret_key = SecretKey::from(key_bytes);
            return Ok(PrivateKey::Normal(secret_key));
        }

        Err(Error::UnexpectedBech32Length)
    }

    #[allow(clippy::len_without_is_empty)]
    pub fn len(&self) -> usize {
        match self {
            Self::Normal(_) => SecretKey::SIZE,
            Self::Extended(_) => SecretKeyExtended::SIZE,
        }
    }

    pub fn public_key(&self) -> PublicKey {
        match self {
            Self::Normal(x) => x.public_key(),
            Self::Extended(x) => x.public_key(),
        }
    }

    pub fn sign<T>(&self, msg: T) -> Signature
    where
        T: AsRef<[u8]>,
    {
        match self {
            Self::Normal(x) => x.sign(msg),
            Self::Extended(x) => x.sign(msg),
        }
    }

    pub fn sign_tx(
        &self,
        tx: pallas::txbuilder::BuiltTransaction,
    ) -> Result<pallas::txbuilder::BuiltTransaction, TxBuilderError> {
        match self {
            Self::Normal(x) => tx.sign(x),
            Self::Extended(x) => tx.sign(x),
        }
    }

    pub(crate) fn as_bytes(&self) -> Vec<u8> {
        match self {
            Self::Normal(x) => {
                let bytes: [u8; SecretKey::SIZE] = unsafe { SecretKey::leak_into_bytes(x.clone()) };
                bytes.to_vec()
            }
            Self::Extended(x) => {
                let bytes: [u8; SecretKeyExtended::SIZE] =
                    unsafe { SecretKeyExtended::leak_into_bytes(x.clone()) };
                bytes.to_vec()
            }
        }
    }

    pub fn address_testnet(&self) -> Address {
        self.address_with_network(Network::Testnet)
    }

    pub fn address_mainnet(&self) -> Address {
        self.address_with_network(Network::Mainnet)
    }

    pub fn address_with_network(&self, network: Network) -> Address {
        let public_key = self.public_key();
        let hash = public_key.compute_hash();

        // PaymentKeyHash is Hash<28>, so we need to take the first 28 bytes
        // compute_hash() typically returns Hash<32>, so we take a slice
        let hash_bytes = hash.as_ref();
        let mut payment_key_hash_bytes = [0u8; 28];
        payment_key_hash_bytes.copy_from_slice(&hash_bytes[..28]);
        let payment_key_hash: PaymentKeyHash = payment_key_hash_bytes.into();

        // Create enterprise address (no staking/delegation)
        let shelley_address = ShelleyAddress::new(
            network,
            ShelleyPaymentPart::key_hash(payment_key_hash),
            ShelleyDelegationPart::Null,
        );

        Address::Shelley(shelley_address)
    }
}

impl From<SecretKey> for PrivateKey {
    fn from(key: SecretKey) -> Self {
        PrivateKey::Normal(key)
    }
}

impl From<SecretKeyExtended> for PrivateKey {
    fn from(key: SecretKeyExtended) -> Self {
        PrivateKey::Extended(key)
    }
}

/// Ed25519-BIP32 HD Private Key
#[derive(Debug, PartialEq, Eq)]
pub struct Bip32PrivateKey(ed25519_bip32::XPrv);

impl Bip32PrivateKey {
    const BECH32_HRP: &'static str = "xprv";

    pub fn generate<T: RngCore + CryptoRng>(mut rng: T) -> Self {
        let mut buf = [0u8; XPRV_SIZE];
        rng.fill_bytes(&mut buf);
        let xprv = XPrv::normalize_bytes_force3rd(buf);

        Self(xprv)
    }

    pub fn generate_with_mnemonic<T: RngCore + CryptoRng>(
        mut rng: T,
        password: String,
    ) -> (Self, Mnemonic) {
        let mut buf = [0u8; 64];
        rng.fill_bytes(&mut buf);

        let bip39 = Mnemonic::generate_in_with(&mut rng, Language::English, 24).unwrap();

        let entropy = bip39.clone().to_entropy();

        let mut pbkdf2_result = [0; XPRV_SIZE];

        const ITER: u32 = 4096; // TODO: BIP39 says 2048, CML uses 4096?

        let mut mac = Hmac::new(Sha512::new(), password.as_bytes());
        pbkdf2(&mut mac, &entropy, ITER, &mut pbkdf2_result);

        (Self(XPrv::normalize_bytes_force3rd(pbkdf2_result)), bip39)
    }

    pub fn from_bytes(bytes: [u8; 96]) -> Result<Self, Error> {
        XPrv::from_bytes_verified(bytes)
            .map(Self)
            .map_err(Error::Xprv)
    }

    pub fn as_bytes(&self) -> Vec<u8> {
        self.0.as_ref().to_vec()
    }

    pub fn from_bip39_mnenomic(mnemonic: String, password: String) -> Result<Self, Error> {
        let bip39 = Mnemonic::parse(mnemonic).map_err(Error::Mnemonic)?;
        let entropy = bip39.to_entropy();

        let mut pbkdf2_result = [0; XPRV_SIZE];

        const ITER: u32 = 4096; // TODO: BIP39 says 2048, CML uses 4096?

        let mut mac = Hmac::new(Sha512::new(), password.as_bytes());
        pbkdf2(&mut mac, &entropy, ITER, &mut pbkdf2_result);

        Ok(Self(XPrv::normalize_bytes_force3rd(pbkdf2_result)))
    }

    pub fn derive(&self, index: u32) -> Self {
        Self(self.0.derive(ed25519_bip32::DerivationScheme::V2, index))
    }

    pub fn to_ed25519_private_key(&self) -> PrivateKey {
        PrivateKey::Extended(unsafe {
            // The use of unsafe is allowed here. The key is an Extended Secret Key
            // already because it passed through the ed25519_bip32 crates checks
            SecretKeyExtended::from_bytes_unchecked(self.0.extended_secret_key())
        })
    }

    pub fn to_public(&self) -> Bip32PublicKey {
        Bip32PublicKey(self.0.public())
    }

    pub fn chain_code(&self) -> [u8; 32] {
        *self.0.chain_code()
    }

    pub fn to_bech32(&self) -> String {
        bech32::encode(
            Self::BECH32_HRP,
            self.as_bytes().to_base32(),
            bech32::Variant::Bech32,
        )
        .unwrap()
    }

    pub fn from_bech32(bech32: String) -> Result<Self, Error> {
        let (hrp, data, _) = bech32::decode(&bech32).map_err(Error::InvalidBech32)?;
        if hrp != Self::BECH32_HRP {
            Err(Error::InvalidBech32Hrp)
        } else {
            let data = Vec::<u8>::from_base32(&data).map_err(Error::InvalidBech32)?;
            Self::from_bytes(data.try_into().map_err(|_| Error::UnexpectedBech32Length)?)
        }
    }
}

/// Ed25519-BIP32 HD Public Key
#[derive(Debug, PartialEq, Eq)]
pub struct Bip32PublicKey(ed25519_bip32::XPub);

impl Bip32PublicKey {
    const BECH32_HRP: &'static str = "xpub";

    pub fn from_bytes(bytes: [u8; 64]) -> Self {
        Self(XPub::from_bytes(bytes))
    }

    pub fn as_bytes(&self) -> Vec<u8> {
        self.0.as_ref().to_vec()
    }

    pub fn derive(&self, index: u32) -> Result<Self, Error> {
        self.0
            .derive(ed25519_bip32::DerivationScheme::V2, index)
            .map(Self)
            .map_err(Error::DerivationError)
    }

    pub fn to_ed25519_pubkey(&self) -> ed25519::PublicKey {
        self.0.public_key().into()
    }

    pub fn chain_code(&self) -> [u8; 32] {
        *self.0.chain_code()
    }

    pub fn to_bech32(&self) -> String {
        bech32::encode(
            Self::BECH32_HRP,
            self.as_bytes().to_base32(),
            bech32::Variant::Bech32,
        )
        .unwrap()
    }

    pub fn from_bech32(bech32: String) -> Result<Self, Error> {
        let (hrp, data, _) = bech32::decode(&bech32).map_err(Error::InvalidBech32)?;
        if hrp != Self::BECH32_HRP {
            Err(Error::InvalidBech32Hrp)
        } else {
            let data = Vec::<u8>::from_base32(&data).map_err(Error::InvalidBech32)?;
            Ok(Self::from_bytes(
                data.try_into().map_err(|_| Error::UnexpectedBech32Length)?,
            ))
        }
    }
}

#[cfg(test)]
mod test {
    use bip39::rand_core::OsRng;

    use super::{Bip32PrivateKey, Bip32PublicKey};

    #[test]
    fn mnemonic_roundtrip() {
        let (xprv, mne) = Bip32PrivateKey::generate_with_mnemonic(OsRng, "".into());

        let xprv_from_mne =
            Bip32PrivateKey::from_bip39_mnenomic(mne.to_string(), "".into()).unwrap();

        assert_eq!(xprv, xprv_from_mne)
    }

    #[test]
    fn bech32_roundtrip() {
        let xprv = Bip32PrivateKey::generate(OsRng);

        let xprv_bech32 = xprv.to_bech32();

        let decoded_xprv = Bip32PrivateKey::from_bech32(xprv_bech32).unwrap();

        assert_eq!(xprv, decoded_xprv);

        let xpub = xprv.to_public();

        let xpub_bech32 = xpub.to_bech32();

        let decoded_xpub = Bip32PublicKey::from_bech32(xpub_bech32).unwrap();

        assert_eq!(xpub, decoded_xpub)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_faucet_private_key() {
        let faucet_private_key_str =
            "ed25519_sk1m8y3kdfd0lds729y3k5f5ccwyljms2j26y3ewu5gjxaf7q6p929sn64988";

        let private_key = PrivateKey::from_bech32(faucet_private_key_str).unwrap();

        // Verify we can get the public key
        let public_key = private_key.public_key();
        assert_eq!(public_key.as_ref().len(), 32);

        // Verify we can sign a message
        let message = b"test message";
        let signature = private_key.sign(message);
        assert!(public_key.verify(message, &signature));
    }
}
