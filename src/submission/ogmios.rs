use crate::config::Config;
use futures_util::{SinkExt, StreamExt};
use pallas::network::miniprotocols::txmonitor::TxId;
use serde::Deserialize;
use serde_json::json;
use thiserror::Error;
use tokio_tungstenite::tungstenite::protocol::Message;
use tokio_tungstenite::MaybeTlsStream;

use super::SubmitTx;

pub struct OgmiosClient<'a> {
    config: &'a Config,

    ws: tokio_tungstenite::WebSocketStream<MaybeTlsStream<tokio::net::TcpStream>>,
}

impl<'a> OgmiosClient<'a> {
    pub async fn new(config: &'a Config, ogmios_url: &str) -> anyhow::Result<Self> {
        let (ws, _) = tokio_tungstenite::connect_async(ogmios_url)
            .await
            .expect("to connect");

        Ok(Self { ws, config })
    }
}

#[derive(Error, Debug)]
pub enum OgmiosClientError {
    #[error("WebSocket error")]
    WebSocketError(#[from] tokio_tungstenite::tungstenite::Error),

    #[error("No response")]
    NoResponse,

    #[error("JSON decode error: {0}")]
    JsonDecodeError(#[from] serde_json::Error),

    #[error("Unexpected response")]
    UnexpectedResponse,

    #[error("Error response from Ogmios: {0:?}")]
    OgmiosError(OgmiosError),
}

#[derive(Deserialize, Debug)]
pub struct OgmiosError {
    pub code: u32,
    pub data: serde_json::Value,
}

#[derive(Deserialize, Debug)]
#[serde(untagged)]
pub enum OgmiosResponse {
    Error { jsonrpc: String, error: OgmiosError },
}

impl SubmitTx for OgmiosClient<'_> {
    type Error = OgmiosClientError;

    async fn submit_tx(
        &mut self,
        tx_id: TxId,
        cbor: &[u8],
    ) -> std::result::Result<TxId, Self::Error> {
        let request = tokio_tungstenite::tungstenite::Message::Text(
            serde_json::to_string(&json!({
                "jsonrpc": "2.0",
                "method": "submitTransaction",
                "params": {
                    "transaction": {
                        "cbor": hex::encode(cbor),
                    },
                },
            }))
            .expect("To be a valid JSON")
            .into(),
        );

        self.ws.send(request).await?;

        let response = self.ws.next().await.ok_or(OgmiosClientError::NoResponse)?;

        match response {
            Ok(Message::Text(text)) => {
                let response: OgmiosResponse =
                    serde_json::from_str(&text).inspect_err(|e| println!("{e:?}: {text}"))?;

                match response {
                    OgmiosResponse::Error { error, .. } => {
                        Err(OgmiosClientError::OgmiosError(error))
                    }
                    _ => Ok(tx_id),
                }
            }
            Ok(_) => Err(OgmiosClientError::UnexpectedResponse),
            Err(e) => Err(e.into()),
        }
    }
}
