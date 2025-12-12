use bip39::Mnemonic;
use cryptoxide::hmac::Hmac;
use cryptoxide::pbkdf2::pbkdf2;
use cryptoxide::sha2::Sha512;
use ed25519_bip32::{XPRV_SIZE, XPrv};
use pallas::crypto::key::ed25519::{PublicKey, SecretKey, SecretKeyExtended, Signature};

use super::{Bip32PublicKey, Error};

/// Ed25519-BIP32 HD Private Key
#[derive(Debug, PartialEq, Eq)]
pub struct Bip32PrivateKey(ed25519_bip32::XPrv);

impl Bip32PrivateKey {
    const BECH32_HRP: &'static str = "xprv";

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
        Bip32PublicKey::new(self.0.public())
    }

    pub fn to_bech32(&self) -> String {
        let hrp = bech32::Hrp::parse(Self::BECH32_HRP).unwrap();
        bech32::encode::<bech32::Bech32>(hrp, self.0.as_ref()).unwrap()
    }

    pub fn from_bech32(bech32: String) -> Result<Self, Error> {
        let (hrp, data) = bech32::decode(&bech32)?;
        if hrp.as_str() != Self::BECH32_HRP {
            return Err(Error::InvalidBech32Hrp);
        }

        let data = data.try_into().map_err(|_| Error::UnexpectedBech32Length)?;
        Ok(Self(XPrv::from_bytes_verified(data)?))
    }
}

/// A standard or extended Ed25519 secret key
pub enum PrivateKey {
    Normal(SecretKey),
    Extended(SecretKeyExtended),
}

impl PrivateKey {
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
