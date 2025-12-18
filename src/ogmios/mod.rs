use reqwest::{Response, Url};
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};

pub mod codec;
pub mod evaluate;
pub mod pparams;
pub mod script;
pub mod submit;
pub mod utxo;

use codec::{RpcRequest, RpcResponse, TxCbor, TxOutputPointer};
use evaluate::{EvaluateRequestParams, Evaluation, EvaluationError};
use submit::{SubmitError, SubmitRequestParams, SubmitResult};
use utxo::{Utxo, UtxoError, UtxoRequestParams};

use crate::ogmios::pparams::ProtocolParams;
use crate::ogmios::utxo::ProtocolParamsError;

pub struct OgmiosClient {
    url: Url,
    client: reqwest::Client,
}

// TODO: handle reqwest error
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
        params: Option<T>,
    ) -> Result<RpcResponse<U, E>, reqwest::Error> {
        let res = self
            .client
            .post(self.url.clone())
            .json(&RpcRequest {
                jsonrpc: "2.0".to_string(),
                method: method.to_string(),
                params,
            })
            .send()
            .await?;
        res.json().await
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
        self.request("evaluateTransaction", Some(params))
            .await
            .unwrap()
            .into()
    }

    pub async fn submit(&self, tx_cbor: &[u8]) -> Result<SubmitResult, SubmitError> {
        let params = SubmitRequestParams {
            transaction: TxCbor {
                cbor: hex::encode(tx_cbor),
            },
        };
        self.request("submitTransaction", Some(params))
            .await
            .unwrap()
            .into()
    }

    pub async fn utxos_by_addresses(&self, addresses: &[&str]) -> Result<Vec<Utxo>, UtxoError> {
        let params = UtxoRequestParams::ByAddress {
            addresses: addresses.iter().map(|s| s.to_string()).collect(),
        };
        self.request("queryLedgerState/utxo", Some(params))
            .await
            .unwrap()
            .into()
    }

    pub async fn utxos_by_output(
        &self,
        output_pointers: &[TxOutputPointer],
    ) -> Result<Vec<Utxo>, UtxoError> {
        let params = UtxoRequestParams::ByOutputReference {
            output_references: output_pointers.to_vec(),
        };
        self.request("queryLedgerState/utxo", Some(params))
            .await
            .unwrap()
            .into()
    }

    pub async fn protocol_params(&self) -> Result<ProtocolParams, ProtocolParamsError> {
        self.request("queryLedgerState/protocolParameters", None::<()>)
            .await
            .expect("failed to get protocol parameters")
            .into()
    }
}
