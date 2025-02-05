#![allow(clippy::from_over_into)]

use super::client::*;
use super::types::{Request, RequestMethod};
use crate::EvaluateTx;
use serde_json::json;

impl EvaluateTx for OgmiosClient {
    type Error = ClientError;
    async fn evaluate_tx(
        &mut self,
        cbor: &[u8],
    ) -> std::result::Result<Vec<crate::ScriptEvaluation>, Self::Error> {
        let response = self.request(EvaluateRequest::new(cbor).into()).await?;
        let script_evaluation: Vec<ScriptEvaluation> = serde_json::from_value(response)?;
        Ok(script_evaluation.into_iter().map(|s| s.into()).collect())
    }
}

pub struct EvaluateRequest<'a> {
    transaction: &'a [u8],
}

impl<'a> EvaluateRequest<'a> {
    pub fn new(transaction: &'a [u8]) -> Self {
        Self { transaction }
    }
}

impl Into<Request> for EvaluateRequest<'_> {
    fn into(self) -> Request {
        Request {
            jsonrpc: "2.0".into(),
            method: RequestMethod::EvaluateTransaction.into(),
            id: None,
            params: Some(json!({ "transaction": { "cbor": hex::encode(self.transaction) } })),
        }
    }
}

#[derive(serde::Deserialize)]
pub struct ScriptEvaluation {
    validator: String,
    budget: ScriptBudget,
}

#[derive(serde::Deserialize)]
struct ScriptBudget {
    memory: u64,
    cpu: u64,
}

impl Into<crate::ScriptEvaluation> for ScriptEvaluation {
    fn into(self) -> crate::ScriptEvaluation {
        // TODO: remove unwraps and panics
        crate::ScriptEvaluation {
            script_type: match self.validator.as_str() {
                "spend" => crate::ScriptType::Spend,
                "certificate" => crate::ScriptType::Certificate,
                "mint" => crate::ScriptType::Mint,
                "withdrawal" => crate::ScriptType::Withdrawal,
                _ => panic!("Unknown validator type: {}", self.validator),
            },
            script_index: self
                .validator
                .split(':')
                .next_back()
                .unwrap()
                .parse()
                .unwrap(),
            memory_budget: self.budget.memory,
            cpu_budget: self.budget.cpu,
        }
    }
}
