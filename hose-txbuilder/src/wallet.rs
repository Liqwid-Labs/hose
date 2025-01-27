use bip32::{secp256k1::elliptic_curve::rand_core, ChildNumber};
use pallas::wallet::keystore::{hd::Bip32PrivateKey, Error, PrivateKey};

pub struct Wallet {
    payment_key: Bip32PrivateKey,
}

impl Wallet {
    pub fn generate() -> Self {
        let private_key = Bip32PrivateKey::generate(rand_core::OsRng::default());
        let payment_key = payment_key_from_private(&private_key);
        Self { payment_key }
    }

    pub fn from_mnemonic(mnemonic: &str, password: Option<&str>) -> Result<Self, Error> {
        let private_key = Bip32PrivateKey::from_bip39_mnenomic(
            mnemonic.into(),
            password.unwrap_or_default().into(),
        )?;
        let payment_key = payment_key_from_private(&private_key);
        Ok(Self { payment_key })
    }

    pub fn from_pkey(pkey: &str) -> Result<Self, Error> {
        let private_key = Bip32PrivateKey::from_bech32(pkey.into())?;
        let payment_key = payment_key_from_private(&private_key);
        Ok(Self { payment_key })
    }
}

impl From<Wallet> for PrivateKey {
    fn from(wallet: Wallet) -> Self {
        wallet.payment_key.to_ed25519_private_key()
    }
}

fn payment_key_from_private(private_key: &Bip32PrivateKey) -> Bip32PrivateKey {
    // https://cardano.stackexchange.com/questions/7671/what-is-the-derivation-path-in-a-cardano-address
    let account_key = private_key
        .derive(ChildNumber::HARDENED_FLAG + 1852)
        .derive(ChildNumber::HARDENED_FLAG + 1815)
        .derive(ChildNumber::HARDENED_FLAG);
    let payment_key = account_key.derive(0).derive(0);

    payment_key
}
