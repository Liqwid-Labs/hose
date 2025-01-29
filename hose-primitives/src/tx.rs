use std::{collections::HashMap, ops::Deref};

pub use pallas::txbuilder::{Input, Output};
use pallas::{crypto::hash::Hash, ledger::addresses::Address};

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct TxHash(Hash<32>);

impl From<Hash<32>> for TxHash {
    fn from(hash: Hash<32>) -> Self {
        Self(hash)
    }
}

impl Deref for TxHash {
    type Target = Hash<32>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Asset {
    pub key: AssetKey,
    pub amount: u64,
}

impl Asset {
    pub fn new(policy_id: Hash<28>, name: Vec<u8>, amount: u64) -> Self {
        Self {
            key: AssetKey { policy_id, name },
            amount,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct AssetKey {
    pub policy_id: Hash<28>,
    pub name: Vec<u8>,
}

#[derive(Debug, Clone)]
pub struct TxO {
    pub address: Address,
    pub tx_hash: TxHash,
    pub txo_index: u64,
    pub lovelace: u64,
    pub assets: HashMap<AssetKey, i64>,
    // datum
    // scripts
}

impl PartialEq for TxO {
    fn eq(&self, other: &Self) -> bool {
        self.tx_hash == other.tx_hash && self.txo_index == other.txo_index
    }
}

impl From<TxO> for Input {
    fn from(txo: TxO) -> Self {
        Input::new(*txo.tx_hash, txo.txo_index)
    }
}

impl From<TxO> for Output {
    fn from(txo: TxO) -> Self {
        Output::new(txo.address, txo.lovelace)
    }
}

#[derive(Debug, Clone)]
pub struct UTxO {
    pub address: Address,
    pub tx_hash: TxHash,
    pub txo_index: u64,
    pub lovelace: u64,
    pub assets: HashMap<AssetKey, i64>,
}

impl PartialEq for UTxO {
    fn eq(&self, other: &Self) -> bool {
        self.tx_hash == other.tx_hash && self.txo_index == other.txo_index
    }
}

impl From<TxO> for UTxO {
    fn from(txo: TxO) -> Self {
        Self {
            address: txo.address,
            tx_hash: txo.tx_hash,
            txo_index: txo.txo_index,
            lovelace: txo.lovelace,
            assets: txo.assets,
        }
    }
}
