use ed25519_bip32::XPub;
use pallas::crypto::hash::{Hash, Hasher};
use pallas::crypto::key::ed25519;

use super::Error;

/// Ed25519-BIP32 HD Public Key
#[derive(Debug, PartialEq, Eq)]
pub struct Bip32PublicKey(XPub);

impl Bip32PublicKey {
    const BECH32_HRP: &'static str = "xpub";

    pub fn new(xpub: XPub) -> Self {
        Self(xpub)
    }

    fn from_bytes(bytes: [u8; 64]) -> Self {
        Self(XPub::from_bytes(bytes))
    }

    pub fn as_bytes(&self) -> &[u8] {
        self.0.as_ref()
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

    pub fn hash(&self) -> Hash<28> {
        let mut hasher = Hasher::<224>::new();
        hasher.input(self.0.as_ref());
        hasher.finalize()
    }

    pub fn to_bech32(&self) -> String {
        let hrp = bech32::Hrp::parse(Self::BECH32_HRP).unwrap();
        bech32::encode::<bech32::Bech32>(hrp, self.0.as_ref()).unwrap()
    }

    pub fn from_bech32(bech32: String) -> Result<Self, Error> {
        let (hrp, data) = bech32::decode(&bech32).map_err(Error::InvalidBech32)?;
        if hrp.as_str() != Self::BECH32_HRP {
            return Err(Error::InvalidBech32Hrp);
        }

        let data = data.try_into().map_err(|_| Error::UnexpectedBech32Length)?;
        Ok(Self::from_bytes(data))
    }
}
