use bip32::ChildNumber;
use bip39::Mnemonic;
use cryptoxide::hmac::Hmac;
use cryptoxide::pbkdf2::pbkdf2;
use cryptoxide::sha2::Sha512;
use ed25519_bip32::{XPRV_SIZE, XPrv};
use pallas::crypto::key::ed25519::SecretKeyExtended;

use super::{Error, PrivateKey};

/// Ed25519-BIP32 HD Private Key
#[derive(Debug, PartialEq, Eq)]
pub struct HDPrivateKey(ed25519_bip32::XPrv);

impl HDPrivateKey {
    pub const BECH32_HRP: &'static str = "xprv";

    pub fn from_bip39_mnenomic(mnemonic: &str, password: &str) -> Result<Self, Error> {
        let bip39 = Mnemonic::parse(mnemonic).map_err(Error::Mnemonic)?;
        let entropy = bip39.to_entropy();

        let mut pbkdf2_result = [0; XPRV_SIZE];

        // https://github.com/cardano-foundation/CIPs/blob/master/CIP-0003/Icarus.md
        const ITER: u32 = 4096;
        let mut mac = Hmac::new(Sha512::new(), password.as_bytes());
        pbkdf2(&mut mac, &entropy, ITER, &mut pbkdf2_result);

        Ok(Self(XPrv::normalize_bytes_force3rd(pbkdf2_result)))
    }

    pub fn from_bech32(bech32: &str) -> Result<Self, Error> {
        let (hrp, data) = bech32::decode(bech32)?;
        if matches!(hrp.as_str(), "xprv" | "root_xsk") {
            return Err(Error::InvalidBech32Hrp(hrp.to_string()));
        }

        let data = data.try_into().map_err(|_| Error::UnexpectedKeyLength)?;
        Ok(Self(XPrv::from_bytes_verified(data)?))
    }

    pub fn private_key(&self) -> PrivateKey {
        PrivateKey::Extended(unsafe {
            SecretKeyExtended::from_bytes_unchecked(self.0.extended_secret_key())
        })
    }

    fn derive(&self, index: u32) -> Self {
        Self(self.0.derive(ed25519_bip32::DerivationScheme::V2, index))
    }

    pub fn derive_key_from_root(
        &self,
        account_index: u32,
        role: PrivateKeyRole,
        address_index: u32,
    ) -> Self {
        self.derive(ChildNumber::HARDENED_FLAG + 1852) // purpose (shelley)
            .derive(ChildNumber::HARDENED_FLAG + 1815) // coin type (ADA)
            .derive(ChildNumber::HARDENED_FLAG + account_index) // account
            .derive(role as u32) // 0 (external), 1 (internal), 2 (stake)
            .derive(address_index) // users may create multiple addresses per account
    }

    pub fn derive_key_from_account(&self, role: PrivateKeyRole, address_index: u32) -> Self {
        self.derive(role as u32) // 0 (external), 1 (internal), 2 (stake)
            .derive(address_index) // users may create multiple addresses per account
    }
}

#[derive(Debug, PartialEq, Eq)]
#[repr(u32)]
pub enum PrivateKeyRole {
    External = 0,
    Internal = 1,
    Stake = 2,
}
