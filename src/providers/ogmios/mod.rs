use reqwest::Url;
use serde::Serialize;
use serde::de::DeserializeOwned;
use uuid::Uuid;

pub mod codec;
pub mod evaluate;
pub mod script;
pub mod submit;
pub mod utxo;

use codec::{RpcRequest, RpcResponse, TxCbor, TxOutputPointer};
use evaluate::{EvaluateRequestParams, Evaluation, EvaluationError};
use utxo::{Utxo, UtxoError, UtxoRequestParams};

pub struct OgmiosClient {
    url: Url,
    client: reqwest::Client,
}

impl OgmiosClient {
    pub fn new(url: Url) -> Self {
        Self {
            url,
            client: reqwest::Client::new(),
        }
    }

    async fn request<T: Serialize, U: DeserializeOwned, E: DeserializeOwned>(
        &self,
        method: &str,
        params: T,
    ) -> Result<RpcResponse<U, E>, reqwest::Error> {
        self.client
            .post(self.url.clone())
            .json(&RpcRequest {
                jsonrpc: "2.0".to_string(),
                method: method.to_string(),
                id: Uuid::new_v4(),
                params,
            })
            .send()
            .await?
            .json()
            .await
    }

    pub async fn evaluate(
        &self,
        tx_cbor: &[u8],
        additional_utxo: Vec<Utxo>,
    ) -> Result<Vec<Evaluation>, EvaluationError> {
        let params = EvaluateRequestParams {
            transaction: TxCbor {
                cbor: hex::encode(tx_cbor),
            },
            additional_utxo,
        };
        // TODO: handle reqwest error
        self.request("evaluateTransaction", params)
            .await
            .unwrap()
            .into()
    }

    pub async fn utxos_by_addresses(&self, addresses: &[&str]) -> Result<Vec<Utxo>, UtxoError> {
        let params = UtxoRequestParams::ByAddress {
            addresses: addresses.iter().map(|s| s.to_string()).collect(),
        };
        // TODO: handle reqwest error
        self.request("queryLedgerState/utxo", params)
            .await
            .unwrap()
            .into()
    }

    pub async fn utxos_by_output_reference(
        &self,
        output_references: &[TxOutputPointer],
    ) -> Result<Vec<Utxo>, UtxoError> {
        let params = UtxoRequestParams::ByOutputReference {
            output_references: output_references.to_vec(),
        };
        // TODO: handle reqwest error
        self.request("queryLedgerState/utxo", params)
            .await
            .unwrap()
            .into()
    }
}
