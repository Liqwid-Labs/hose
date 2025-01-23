use futures_util::{SinkExt, StreamExt};
use tokio_tungstenite::tungstenite::protocol::Message;
use tokio_tungstenite::MaybeTlsStream;

use super::types::*;

use super::types::OgmiosError;
use thiserror::Error;

pub struct OgmiosClient {
    ws: tokio_tungstenite::WebSocketStream<MaybeTlsStream<tokio::net::TcpStream>>,
}

impl OgmiosClient {
    pub async fn new(ogmios_url: &str) -> Result<Self, OgmiosClientError> {
        let (ws, _) = tokio_tungstenite::connect_async(ogmios_url).await?;
        Ok(Self { ws })
    }

    pub async fn request(&mut self, request: Request) -> Result<serde_json::Value, OgmiosClientError> {
        let request = tokio_tungstenite::tungstenite::Message::Text(
            serde_json::to_string(&request).unwrap().into()
        );
        self.ws.send(request).await?;

        let response = self.ws.next().await.ok_or(OgmiosClientError::NoResponse)??;
        match response {
            Message::Text(text) => {
                let response: OgmiosResponse =
                    serde_json::from_str(&text).inspect_err(|e| println!("{e:?}: {text}"))?;

                match response {
                    OgmiosResponse::Error { error, .. } => {
                        Err(OgmiosClientError::OgmiosError(error))
                    }
                   OgmiosResponse::Result { result, .. } => Ok(result),
                }
            }
            _ => Err(OgmiosClientError::UnexpectedResponse),
        }
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
