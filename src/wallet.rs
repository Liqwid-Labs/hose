use bip32::ChildNumber;
use pallas::wallet::keystore::hd::Bip32PrivateKey;

pub fn load_private_key_from_mnemonic(mnemonic: String) -> anyhow::Result<Bip32PrivateKey> {
    let private_key = Bip32PrivateKey::from_bip39_mnenomic(mnemonic, "".into())?;

    // https://cardano.stackexchange.com/questions/7671/what-is-the-derivation-path-in-a-cardano-address
    let account_key = private_key
        .derive(ChildNumber::HARDENED_FLAG + 1852)
        .derive(ChildNumber::HARDENED_FLAG + 1815)
        .derive(ChildNumber::HARDENED_FLAG + 0);

    let payment_key = account_key.derive(0).derive(0);

    Ok(payment_key)
}

