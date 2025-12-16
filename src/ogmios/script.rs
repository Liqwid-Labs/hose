use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::codec::{ExecutionUnits, Language, RedeemerPointer, TxOutputPointer};
use crate::define_ogmios_error;

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(tag = "language")]
pub enum Script {
    #[serde(rename = "native")]
    Native {
        json: ScriptClause,
        /// Hex-encoded
        cbor: Option<String>,
    },
    #[serde(rename = "plutus:v1")]
    PlutusV1 { cbor: String },
    #[serde(rename = "plutus:v2")]
    PlutusV2 { cbor: String },
    #[serde(rename = "plutus:v3")]
    PlutusV3 { cbor: String },
}

impl Script {
    pub fn cbor(&self) -> Option<&str> {
        match self {
            Script::Native { cbor, .. } => cbor.as_ref().map(|x| x.as_str()),
            Script::PlutusV1 { cbor, .. } => Some(cbor),
            Script::PlutusV2 { cbor, .. } => Some(cbor),
            Script::PlutusV3 { cbor, .. } => Some(cbor),
        }
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(tag = "clause")]
#[serde(rename_all = "camelCase")]
pub enum ScriptClause {
    Signature {
        /// Hex-encoded 28-byte blake2b hash digest
        from: String,
    },
    Any {
        from: Vec<Script>,
    },
    All {
        from: Vec<Script>,
    },
    Some {
        at_least: usize,
        from: Vec<Script>,
    },
    Before {
        slot: u64,
    },
    After {
        slot: u64,
    },
}

#[derive(Debug, Clone, Deserialize)]
pub struct ScriptError {
    pub validator: RedeemerPointer,
    pub error: ScriptExecutionError,
}

define_ogmios_error! {
    #[derive(Debug, Clone)]
    pub enum ScriptExecutionError {
        3011 => InvalidRedeemerPointers {
            missing_scripts: Vec<RedeemerPointer>,
        },
        3012 => ValidationFailure {
            validation_error: String,
            traces: Vec<String>,
        },
        3013 => UnsuitableOutputReference {
            unsuitable_output_reference: TxOutputPointer,
        },
        3110 => ExtraneousRedeemers {
            extraneous_redeemers: Vec<RedeemerPointer>,
        },
        3111 => MissingDatums {
            missing_datums: Vec<String>,
        },
        3115 => MissingCostModels {
            missing_cost_models: Vec<Language>,
        },
        3117 => UnknownOutputReferences {
            unknown_output_references: Vec<TxOutputPointer>,
        },
        3161 => ExecutionBudgetOutOfBounds {
            budget_used: ExecutionUnits,
        },
        _ => Unknown { error: Value }
    }
}

#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "purpose")]
#[serde(rename_all = "camelCase")]
pub enum ScriptPurpose {
    #[serde(rename = "spend")]
    Spend { output_reference: TxOutputPointer },
    #[serde(rename = "mint")]
    Mint {
        /// Hex-encoded 28-byte blake2b hash digest
        policy: String,
    },
    #[serde(rename = "publish")]
    Publish {
        certificate: Value, // TODO:
    },
    #[serde(rename = "withdraw")]
    Withdraw {
        /// Stake address (stake1...)
        reward_account: String,
    },
    #[serde(rename = "propose")]
    Propose {
        proposal: Value, // TODO:
    },
    #[serde(rename = "vote")]
    Vote {
        issuer: Value, // TODO:
    },
}
