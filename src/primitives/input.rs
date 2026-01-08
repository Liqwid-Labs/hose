use hydrant::primitives::{TxOutput, TxOutputPointer};

use super::*;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Input {
    pub hash: TxHash,
    pub index: u64,
}

impl Input {
    pub fn new(hash: TxHash, index: u64) -> Self {
        Self { hash, index }
    }
}

impl From<TxOutputPointer> for Input {
    fn from(txo: TxOutputPointer) -> Self {
        Self {
            hash: txo.hash,
            index: txo.index,
        }
    }
}
impl From<&TxOutputPointer> for Input {
    fn from(txo: &TxOutputPointer) -> Self {
        Self {
            hash: txo.hash,
            index: txo.index,
        }
    }
}
impl From<TxOutput> for Input {
    fn from(txo: TxOutput) -> Self {
        Self {
            hash: txo.hash,
            index: txo.index,
        }
    }
}
impl From<&TxOutput> for Input {
    fn from(txo: &TxOutput) -> Self {
        Self {
            hash: txo.hash,
            index: txo.index,
        }
    }
}
impl From<Input> for TxOutputPointer {
    fn from(input: Input) -> Self {
        Self {
            hash: input.hash,
            index: input.index,
        }
    }
}
impl From<&Input> for TxOutputPointer {
    fn from(input: &Input) -> Self {
        Self {
            hash: input.hash,
            index: input.index,
        }
    }
}

impl PartialEq<TxOutput> for Input {
    fn eq(&self, other: &TxOutput) -> bool {
        self.hash == other.hash && self.index == other.index
    }
}
