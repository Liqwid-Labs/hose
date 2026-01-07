use serde::{Deserialize, Serialize};

use super::codec::{Balance, Era, TxPointer};
use super::script::Script;
use super::*;
use crate::define_ogmios_error;

#[derive(Debug, Clone, Serialize)]
#[serde(untagged)]
#[serde(rename_all = "camelCase")]
pub enum UtxoRequestParams {
    ByOutputReference {
        // For some reason rename_all doesn't work for this field.
        #[serde(rename = "outputReferences")]
        output_references: Vec<TxOutputPointer>,
    },
    ByAddress {
        addresses: Vec<String>,
    },
}
pub type UtxoRequest = RpcRequest<UtxoRequestParams>;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Utxo {
    pub transaction: TxPointer,
    pub index: u32,
    /// A Cardano address (either legacy format or new format)
    /// New: `addr1q9d34spgg2kdy47n82e7x9pdd6vql6d2engxmpj20jmhuc2047yqd4xnh7u6u5jp4t0q3fkxzckph4tgnzvamlu7k5psuahzcp`
    /// Legacy: `DdzFFzCqrht8mbSTZHqpM2u4HeND2mdspsaBhdQ1BowPJBMzbDeBMeKgqdoKqo1D4sdPusEdZJVrFJRBBxX1jUEofNDYCJSZLg8MkyCE`
    pub address: String,
    pub value: Balance,
    /// A Blake2b 32-byte hash digest, hex-encoded
    pub datum_hash: Option<String>,
    /// A hex-encoded CBOR value
    pub datum: Option<String>,
    pub script: Option<Script>,
}
impl From<Utxo> for hydrant::primitives::TxOutputPointer {
    fn from(utxo: Utxo) -> Self {
        let mut hash = [0u8; 32];
        hex::decode_to_slice(utxo.transaction.id, &mut hash).unwrap();
        Self {
            hash: hash.into(),
            index: utxo.index as u64,
        }
    }
}
impl From<Utxo> for pallas::txbuilder::Input {
    fn from(utxo: Utxo) -> Self {
        let pointer: hydrant::primitives::TxOutputPointer = utxo.into();
        pallas::txbuilder::Input::new(pointer.hash.0.into(), pointer.index as u64)
    }
}

impl From<Utxo> for crate::builder::transaction::model::Input {
    fn from(utxo: Utxo) -> Self {
        let pointer: hydrant::primitives::TxOutputPointer = utxo.into();
        crate::builder::transaction::model::Input::new(pointer.hash.0.into(), pointer.index as u64)
    }
}

define_ogmios_error! {
    #[derive(Debug, Clone)]
    pub enum UtxoError {
        2001 => EraMismatch {
            query_era: Era,
            ledger_era: Era,
        },
        2002 => UnavailableInCurrentEra,
        2003 => StateAcquiredExpired(String)
        _ => Unknown { error: Value }
    }
}

pub type UtxoResponse = RpcResponse<Vec<Utxo>, UtxoError>;

define_ogmios_error! {
    #[derive(Debug, Clone)]
    pub enum ProtocolParamsError {
        2001 => EraMismatch {
            query_era: Era,
            ledger_era: Era,
        },
        2002 => UnavailableInCurrentEra,
        2003 => StateAcquiredExpired(String)
        _ => Unknown { error: Value }
    }
}
