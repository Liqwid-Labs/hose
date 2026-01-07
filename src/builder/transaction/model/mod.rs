pub use hydrant::primitives::Hash;
pub use pallas::ledger::addresses::Address;

mod assets;
mod built_tx;
mod script;
mod signer;
mod staging_tx;
mod txo;

pub use assets::*;
pub use built_tx::*;
pub use script::*;
pub use signer::*;
pub use staging_tx::*;
pub use txo::*;

#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub struct Bytes(pub Vec<u8>);

impl From<Bytes> for pallas::codec::utils::Bytes {
    fn from(value: Bytes) -> Self {
        value.0.into()
    }
}

impl From<Vec<u8>> for Bytes {
    fn from(value: Vec<u8>) -> Self {
        Bytes(value)
    }
}

impl AsRef<[u8]> for Bytes {
    fn as_ref(&self) -> &[u8] {
        &self.0
    }
}

pub type PublicKey = Hash<32>;
pub type Signature = Hash<64>;

pub type TxHash = Hash<32>;
pub type PubKeyHash = Hash<28>;
pub type ScriptHash = Hash<28>;
pub type ScriptBytes = Bytes;
pub type PolicyId = ScriptHash;
pub type DatumHash = Hash<32>;
pub type DatumBytes = Bytes;
pub type AssetName = Bytes;
