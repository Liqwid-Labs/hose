use serde::{Deserialize, Serialize};

use super::codec::*;
use super::script::ScriptError;
use crate::define_ogmios_error;

// -----------
// Request
// -----------

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EvaluateRequestParams {
    pub transaction: TxCbor,
}

// -----------
// Response
// -----------

#[derive(Debug, Clone, Deserialize)]
pub struct Evaluation {
    pub validator: RedeemerPointer,
    pub budget: ExecutionUnits,
}

define_ogmios_error! {
    #[derive(Debug, Clone)]
    pub enum EvaluationError {
        3000 => IncompatibleEra {
            incompatible_era: Era,
        },
        3001 => UnsupportedEra {
            unsupported_era: Era,
        },
        3002 => OverlappingAdditionalUtxo {
            overlapping_output_references: Vec<TxOutputPointer>,
        },
        3003 => NodeTipTooOld {
            minimum_required_era: Era,
            current_node_era: Era,
        },
        3004 => CannotCreateEvaluationContext {
            reason: String,
        },
        3010 => ScriptExecution {
            errors: Vec<ScriptError>,
        },
        -32602 => Deserialization {
            byron: String,
            shelley: String,
            allegra: String,
            mary: String,
            alonzo: String,
            babbage: String,
            conway: String,
        },
        _ => Unknown { error: Value }
    }
}

pub type EvaluateResponse = RpcResponse<Vec<Evaluation>, EvaluationError>;
