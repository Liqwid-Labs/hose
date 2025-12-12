use bip32::ChildNumber;
use pallas::ledger::addresses::Network;

use super::{Bip32PrivateKey, Error, Wallet};

pub enum AddressType {
    Base,
    Enterprise,
}

pub struct WalletBuilder {
    network: Network,
    address_type: AddressType,
    account_index: u32,
    address_index: u32,
}

impl WalletBuilder {
    pub fn new(network: Network) -> Self {
        Self {
            network,
            address_type: AddressType::Base,
            account_index: 0,
            address_index: 0,
        }
    }

    pub fn address_type(mut self, address_type: AddressType) -> Self {
        self.address_type = address_type;
        self
    }

    pub fn account_index(mut self, account_index: u32) -> Self {
        self.account_index = account_index;
        self
    }

    pub fn address_index(mut self, address_index: u32) -> Self {
        self.address_index = address_index;
        self
    }

    pub fn from_mnemonic(self, mnemonic: String, address_index: u32) -> Result<Wallet, Error> {
        let private_key = Bip32PrivateKey::from_bip39_mnenomic(mnemonic, "".to_string())?;

        let account_key = self.derive_account_key(private_key);
        let payment_key = self.derive_payment_key(account_key);
        let stake_key = self.derive_stake_key(account_key);

        Ok(Wallet {
            network: self.network,
            address: self.address(payment_key, stake_key),
            private_key,
            payment_key,
            stake_key,
        })
    }

    pub fn from_bech32(self, bech32: String) -> Result<Wallet, Error> {
        let private_key = Bip32PrivateKey::from_bech32(bech32)?;

        let account_key = self.derive_account_key(private_key);
        let payment_key = self.derive_payment_key(account_key);
        let stake_key = self.derive_stake_key(account_key);

        Ok(Wallet {
            network: self.network,
            address: self.address(payment_key, stake_key),
            private_key,
            payment_key,
            stake_key,
        })
    }

    fn derive_account_key(&self, private_key: Bip32PrivateKey) -> Bip32PrivateKey {
        private_key
            .derive(ChildNumber::HARDENED_FLAG + 1852) // purpose (shelley)
            .derive(ChildNumber::HARDENED_FLAG + 1815) // coin type (ADA)
            .derive(ChildNumber::HARDENED_FLAG + self.account_index) // account
    }

    fn derive_payment_key(&self, account_key: Bip32PrivateKey) -> Bip32PrivateKey {
        account_key
            .derive(0) // role (external)
            .derive(self.address_index) // address index
    }

    fn derive_stake_key(&self, account_key: Bip32PrivateKey) -> Bip32PrivateKey {
        account_key
            .derive(2) // role (stake)
            .derive(self.address_index) // address index
    }

    fn hrp(&self) -> bech32::Hrp {
        match self.network {
            Network::Testnet => bech32::Hrp::parse("addr_test").unwrap(),
            _ => bech32::Hrp::parse("addr").unwrap(),
        }
    }

    fn address(&self, payment_key: Bip32PrivateKey, stake_key: Bip32PrivateKey) -> String {
        let payment_part = payment_key.to_public().hash();
        let stake_part = stake_key.to_public().hash();

        match self.address_type {
            AddressType::Base => {
                let header = 0b0000_0000 + self.network.value();
                let data = vec![&vec![header], payment_part.as_ref(), stake_part.as_ref()].concat();
                bech32::encode::<bech32::Bech32>(self.hrp(), &data).unwrap()
            }
            AddressType::Enterprise => {
                let header = 0b0110_0000 + self.network.value();
                let data = vec![&vec![header], payment_part.as_ref()].concat();
                bech32::encode::<bech32::Bech32>(self.hrp(), &data).unwrap()
            }
        }
    }
}
