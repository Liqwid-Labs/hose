#![allow(clippy::from_over_into)]

use std::collections::HashMap;

use super::client::*;
use super::types::{Request, RequestMethod};
use crate::EvaluateTx;
use serde_json::json;
use serde::{Deserialize, Serialize};

impl EvaluateTx for OgmiosClient {
    type Error = ClientError;
    async fn evaluate_tx(
        &mut self,
        cbor: &[u8],
    ) -> std::result::Result<Vec<crate::ScriptEvaluation>, Self::Error> {
        let response = self.request(EvaluateRequest::new(cbor, None).into()).await?;
        let script_evaluation: Vec<ScriptEvaluation> = serde_json::from_value(response)?;
        Ok(script_evaluation.into_iter().map(|s| s.into()).collect())
    }
}

#[derive(Deserialize, Serialize)]
pub struct AdditionalUtxo {
    transaction: AdditionalUtxoTransaction,
    index: u32,
    address: String,
    // TODO: requires 'ada' key with { lovelace: u64 }
    value: HashMap<String, u64>,
    #[serde(rename = "datumHash")]
    datum_hash: Option<String>,
    datum: Option<String>,
    script: Option<AdditionalUtxoScript>
}

#[derive(Deserialize, Serialize)]
pub struct AdditionalUtxoTransaction {
    id: String
}

#[derive(Deserialize, Serialize)]
#[serde(tag = "language")]
pub enum AdditionalUtxoScript {
    #[serde(rename = "native")]
    Native { json: serde_json::Value, cbor: Option<String> },
    #[serde(rename = "plutus:v1")]
    PlutusV1 { cbor: String },
    #[serde(rename = "plutus:v2")]
    PlutusV2 { cbor: String },
    #[serde(rename = "plutus:v3")]
    PlutusV3 { cbor: String },
}

#[derive(Deserialize, Serialize)]
pub struct AdditionalUtxoPlutusScript {
    cbor: String,
}

pub struct EvaluateRequest<'a> {
    transaction: &'a [u8],
    additional_utxo: Option<Vec<AdditionalUtxo>>,
}

impl<'a> EvaluateRequest<'a> {
    pub fn new(transaction: &'a [u8], additional_utxo: Option<Vec<AdditionalUtxo>>) -> Self {
        Self { transaction, additional_utxo }
    }
}

impl From<EvaluateRequest<'_>> for Request {
    fn from(val: EvaluateRequest<'_>) -> Self {
        Request {
            jsonrpc: "2.0".into(),
            method: RequestMethod::EvaluateTransaction.into(),
            id: None,
            params: Some(json!({
                "transaction": {
                    "cbor": hex::encode(val.transaction),
                    "additionalUtxo": val.additional_utxo
                }
            })),
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

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_evaluation() {
        let mut client = OgmiosClient::new("ws://mainnet-ogmios:1337").await.unwrap();

        let tx_cbor_hex = "84a90083825820177fdf2ab641ed86e85698ec9c4fbe88d3f6e3588fe9c26e0d4d22e8f0f8617b0182582065687c478eb701efdb4ae3207b79d47720ddc524e995378e60fe12a7197cb79400825820736970507ff50d29de90e561c9941e20f9d3ad12f696f70007de450591095979020183a300583911af97793b8702f381976cec83e303e9ce17781458c73c4bb16fe02b83fb8e4eff7b4a0dbf7cfdb36ae571d9e030849b9597c16d6c79fa323f01821b000000357dac633ca2581c6fdc63a1d71dc2c65502b79baae7fb543185702b12c3c5fb639ed737a2414c01582093237c26780971289912e3fc907bd7b2cc1ca33ff248616e13299a1219be3ed01b7fffffd6209c3472581cc48cbb3d5e57ed56e276bc45f99ab39abe94e6cd7ac39fb402da47ada1480014df105553444d1b0000002700e9d850028201d8185878d8799f581cc134d839a64a5dfb9b155869ef3f34280751a622f69958baa8ffd29c4040581cc48cbb3d5e57ed56e276bc45f99ab39abe94e6cd7ac39fb402da47ad480014df105553444d181e0500001927101a001e84801b00000194dc5140601a050180981a03e7d3ff00000000d87a80d87a80d87980ff8258390131f0b55b23dc2d732b482a271034e7e6a7da5c289274f0560bded8cbe60ac09e29648ce015366b1338b5b144ddb01aec289a1f2ea7e08a0f821a001e8480a1581cc48cbb3d5e57ed56e276bc45f99ab39abe94e6cd7ac39fb402da47ada1480014df105553444d1a1e68a7e7825839010b46751422f2357dd4ecee5a84b288b982b20375a0503cc7ecdda5aa47754858ddd2795aab89617d72d7d240abccd036fa4b761fa39787a3821a00d7cbd6a1581c1ad3767073087df4fc97fba7ac4a71a0a6cd556f1ad96a7b1c9870c4a1415801021a000754e6031a08c7a73f081a08c792290b5820ca6798c789232cde7b8b658919c374bed9ddd419e2c5919d1582c733803775b60d8182582063c5dea8da9f5241ac8d6359354f2b322aea0698ebd59845d62f362d6dddb6f6000e81581c0b46751422f2357dd4ecee5a84b288b982b20375a0503cc7ecdda5aa12828258205ec56338104fcbfe32288c649d9633f0d9060abce8b8608b156294f0a81d29e201825820babc647257b8d78b86e862ba9769401714ed403e7c46ed1b59c3fc32e0247c8200a20081825820fbc53e7aa4e5497d8662e8f0d5337441f629d1f237217bc24ac41bb6de89f8415840a9fa10d49d92598fcf3d6545a3bdd20472855cd9041384467d4468efa50801954564270a76fdab51091b4bd72f69a5348aa80378adb8467df1581bee7541f40c0582840000d8799f01ff8219759c1a0079bd28840001d8799f01029fd8799f0000ffffff821a000f02831a12c82d02f5f6";
        let tx_cbor = hex::decode(tx_cbor_hex).unwrap();
        let request: Request = EvaluateRequest::new(&tx_cbor, None).into();
        let json = serde_json::to_string(&request).unwrap();

        let response = reqwest::Client::new().post("http://mainnet-ogmios:1337").body(json).send().await.unwrap();

        // let response = client.request(request.into()).await;
        println!("{:?}", response);

        let body = response.text().await.unwrap();
        println!("{}", body);
    }
}
