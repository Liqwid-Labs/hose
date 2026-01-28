use pallas::ledger::addresses::{
    Network, ShelleyAddress, ShelleyDelegationPart, ShelleyPaymentPart,
};

use super::hd_key::PrivateKeyRole;
use super::{Error, HDPrivateKey, PrivateKey, Wallet};

pub enum AddressType {
    Base,
    Enterprise,
}

pub struct WalletBuilder {
    network: Network,
    address: Option<ShelleyAddress>,
    address_type: AddressType,
    account_index: u32,
    address_index: u32,
}

impl WalletBuilder {
    pub fn new(network: Network) -> Self {
        Self {
            network,
            address: None,
            address_type: AddressType::Enterprise,
            account_index: 0,
            address_index: 0,
        }
    }

    pub fn address(mut self, address: ShelleyAddress) -> Self {
        self.address = Some(address);
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

    /// Derives the payment and (optionally, based on `address_type`) stake key from the given
    /// mnemonic and password. The account index and address index are set to 0 by default.
    /// The address will be derived from the keys, unless manually set.
    ///
    /// Payment derivation path: `m/1852'/1815'/$account_index'/0/$address_index`
    /// Stake derivation path: `m/1852'/1815'/$account_index'/2/$address_index`
    pub fn from_mnemonic(self, mnemonic: String, password: String) -> Result<Wallet, Error> {
        let private_key = HDPrivateKey::from_bip39_mnenomic(&mnemonic, &password)?;

        let payment_key = private_key
            .derive_key_from_root(
                self.account_index,
                PrivateKeyRole::External,
                self.address_index,
            )
            .into();
        let stake_key = match self.address_type {
            AddressType::Enterprise => None,
            AddressType::Base => Some(
                private_key
                    .derive_key_from_root(
                        self.account_index,
                        PrivateKeyRole::Stake,
                        self.address_index,
                    )
                    .into(),
            ),
        };

        Ok(Wallet {
            network: self.network,
            address: self
                .address
                .unwrap_or_else(|| address_from_parts(self.network, &payment_key, &stake_key)),
            payment_key,
            stake_key,
        })
    }

    /// Converts the given bech32 string into a payment and (optionally, based on `address_type`
    /// and key type) a stake key. The account index and address index are set to 0 by default.
    /// The address will be derived from the keys, unless manually set.
    ///
    /// Supported bech32 prefixes (HRP):
    /// - `root_xsk` or `xprv`: root key, derive the payment and stake keys
    /// - `acct_xsk`: account key, derived from root key: m/1852'/1815'/$account_index'
    /// - `addr_sk`: payment key, no stake key
    /// - `ed25519_sk` or `ed25519e_sk`: generic key, assume it's a payment key, no stake key
    ///
    /// Payment derivation path: `m/1852'/1815'/$account_index'/0/$address_index`
    /// Stake derivation path: `m/1852'/1815'/$account_index'/2/$address_index`
    pub fn from_bech32(self, bech32: String) -> Result<Wallet, Error> {
        let (hrp, _) = bech32::decode(&bech32)?;
        let (payment_key, stake_key) = match hrp.as_str() {
            // Root key, derive the payment and stake keys
            "root_xsk" | "xprv" => {
                let private_key = HDPrivateKey::from_bech32(&bech32)?;
                let payment_key = private_key
                    .derive_key_from_root(
                        self.account_index,
                        PrivateKeyRole::External,
                        self.address_index,
                    )
                    .into();
                let stake_key = match self.address_type {
                    AddressType::Enterprise => None,
                    AddressType::Base => Some(
                        private_key
                            .derive_key_from_root(
                                self.account_index,
                                PrivateKeyRole::Stake,
                                self.address_index,
                            )
                            .into(),
                    ),
                };
                (payment_key, stake_key)
            }

            // Account key, derived from root key: m/1852'/1815'/$account_index'
            "acct_xsk" => {
                let account_key = HDPrivateKey::from_bech32(&bech32)?;
                let payment_key = account_key
                    .derive_key_from_account(PrivateKeyRole::External, self.address_index)
                    .into();
                let stake_key = match self.address_type {
                    AddressType::Enterprise => None,
                    AddressType::Base => Some(
                        account_key
                            .derive_key_from_account(PrivateKeyRole::Stake, self.address_index)
                            .into(),
                    ),
                };
                (payment_key, stake_key)
            }

            // Generic ed25519 key, assume it's the payment key
            "ed25519_sk" | "ed25519e_sk" | "addr_sk" => (PrivateKey::from_bech32(&bech32)?, None),

            // Unrecognized
            _ => return Err(Error::InvalidBech32Hrp(hrp.to_string())),
        };

        Ok(Wallet {
            network: self.network,
            address: self
                .address
                .unwrap_or_else(|| address_from_parts(self.network, &payment_key, &stake_key)),
            payment_key,
            stake_key: None,
        })
    }

    /// Converts the given hex string into a payment key (32 or 64 bytes).
    ///
    /// Due to the nature of a raw payment key, we cannot derive the stake key for the address,
    /// so the address will always be an `Enterprise` address (no stake part), unless manually set.
    pub fn from_hex(self, hex_payment_key: String) -> Result<Wallet, Error> {
        let private_key = PrivateKey::from_hex(hex_payment_key)?;
        Ok(Wallet {
            network: self.network,
            address: self
                .address
                .unwrap_or_else(|| address_from_parts(self.network, &private_key, &None)),
            payment_key: private_key,
            stake_key: None,
        })
    }
}

fn address_from_parts(
    network: Network,
    payment_key: &PrivateKey,
    stake_key: &Option<PrivateKey>,
) -> ShelleyAddress {
    let payment_part = ShelleyPaymentPart::Key(payment_key.hash());
    let stake_part = match stake_key {
        Some(stake_key) => ShelleyDelegationPart::Key(stake_key.hash()),
        None => ShelleyDelegationPart::Null,
    };
    ShelleyAddress::new(network, payment_part, stake_part)
}
