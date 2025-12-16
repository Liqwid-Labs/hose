use serde::{Deserialize, Serialize};

use super::codec::{Era, TxPointer};
use super::script::Script;
use super::*;
use crate::define_ogmios_error;

#[derive(Debug, Clone, Serialize)]
#[serde(untagged)]
#[serde(rename_all = "camelCase")]
pub enum UtxoRequestParams {
    ByOutputReference {
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
    /// A Blake2b 32-byte hash digest, hex-encoded
    pub datum_hash: Option<String>,
    /// A hex-encoded CBOR value
    pub datum: Option<String>,
    pub script: Option<Script>,
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
