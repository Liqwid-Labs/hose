use pallas::crypto::hash::{Hash, Hasher};
use pallas::crypto::key::ed25519::{PublicKey, SecretKey, SecretKeyExtended, Signature};

use super::Error;
use crate::wallet::HDPrivateKey;

/// A standard or extended Ed25519 secret key
pub enum PrivateKey {
    Normal(SecretKey),
    Extended(SecretKeyExtended),
}

impl PrivateKey {
    pub fn from_hex(hex: String) -> Result<Self, Error> {
        let data = hex::decode(hex)?;
        Self::from_bytes(&data)
    }

    pub fn from_bech32(bech32_str: &str) -> Result<Self, Error> {
        let (hrp, data) = bech32::decode(bech32_str)?;
        if matches!(hrp.as_str(), "ed25519_sk" | "ed25519e_sk") {
            return Err(Error::InvalidBech32Hrp(hrp.to_string()));
        }
        Self::from_bytes(&data)
    }

    pub fn from_bytes<T>(bytes: T) -> Result<Self, Error>
    where
        T: AsRef<[u8]>,
    {
        let bytes = bytes.as_ref();
        match bytes.len() {
            SecretKeyExtended::SIZE => {
                let key_bytes: [u8; SecretKeyExtended::SIZE] =
                    bytes.try_into().map_err(|_| Error::UnexpectedKeyLength)?;
                let secret_key = SecretKeyExtended::from_bytes(key_bytes)?;
                Ok(PrivateKey::Extended(secret_key))
            }
            SecretKey::SIZE => {
                let key_bytes: [u8; SecretKey::SIZE] =
                    bytes.try_into().map_err(|_| Error::UnexpectedKeyLength)?;
                let secret_key = SecretKey::from(key_bytes);
                Ok(PrivateKey::Normal(secret_key))
            }
            _ => Err(Error::UnexpectedKeyLength),
        }
    }

    pub fn public_key(&self) -> PublicKey {
        match self {
            Self::Normal(x) => x.public_key(),
            Self::Extended(x) => x.public_key(),
        }
    }

    pub fn hash(&self) -> Hash<28> {
        let public_key = self.public_key();
        let mut hasher = Hasher::<224>::new();
        hasher.input(public_key.as_ref());
        hasher.finalize()
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

    pub fn len(&self) -> usize {
        match self {
            Self::Normal(_) => SecretKey::SIZE,
            Self::Extended(_) => SecretKeyExtended::SIZE,
        }
    }
}

impl From<HDPrivateKey> for PrivateKey {
    fn from(key: HDPrivateKey) -> Self {
        key.private_key()
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
