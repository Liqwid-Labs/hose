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

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
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

#[cfg(test)]
mod tests {
    use std::str::FromStr as _;

    use serde_json::json;

    use super::*;

    macro_rules! test_rpc_response_success {
        ($name:ident, $json:expr, $type:ty, $error:ty, $value:expr) => {
            #[test]
            fn $name() {
                let json = json!($json);
                let evaluation: RpcResponse<$type, $error> = serde_json::from_value(json).unwrap();
                match evaluation {
                    RpcResponse::Success(success) => assert_eq!(success.result, $value),
                    RpcResponse::Error(error) => panic!("Expected success, got error: {:?}", error),
                }
            }
        };
    }
    test_rpc_response_success!(
        deserialize_evaluation_response,
        json!({"jsonrpc":"2.0","method":"evaluateTransaction","result":[{"validator":{"index":0,"purpose":"spend"},"budget":{"memory":6125,"cpu":1583505}}],"id":null}),
        Vec<Evaluation>,
        EvaluationError,
        vec![Evaluation {
            validator: RedeemerPointer {
                index: 0,
                purpose: RedeemerPurpose::Spend
            },
            budget: ExecutionUnits {
                memory: Ratio(num_rational::BigRational::from_integer(6125.into())),
                cpu: Ratio(num_rational::BigRational::from_integer(1583505.into()))
            },
        }]
    );
    test_rpc_response_success!(
        deserialize_evaluation_mint_response,
        json!({"jsonrpc":"2.0","method":"evaluateTransaction","result":[{"validator":{"index":0,"purpose":"mint"},"budget":{"memory":"1/10","cpu":"1/10"}}],"id":null}),
        Vec<Evaluation>,
        EvaluationError,
        vec![Evaluation {
            validator: RedeemerPointer {
                index: 0,
                purpose: RedeemerPurpose::Mint
            },
            budget: ExecutionUnits {
                memory: Ratio(num_rational::BigRational::from_str("1/10").unwrap()),
                cpu: Ratio(num_rational::BigRational::from_str("1/10").unwrap())
            },
        }]
    );
}
