use futures_util::{SinkExt, StreamExt};
use tokio_tungstenite::tungstenite::protocol::Message;
use tokio_tungstenite::MaybeTlsStream;

use super::types::*;

use super::types::Response;
use thiserror::Error;

pub struct OgmiosClient {
    ws: tokio_tungstenite::WebSocketStream<MaybeTlsStream<tokio::net::TcpStream>>,
}

impl OgmiosClient {
    pub async fn new(ogmios_url: &str) -> Result<Self, ClientError> {
        let (ws, _) = tokio_tungstenite::connect_async(ogmios_url).await?;
        Ok(Self { ws })
    }

    pub async fn request(&mut self, request: Request) -> Result<serde_json::Value, ClientError> {
        let request = tokio_tungstenite::tungstenite::Message::Text(
            serde_json::to_string(&request).unwrap().into()
        );
        self.ws.send(request).await?;

        let response = self.ws.next().await.ok_or(ClientError::NoResponse)??;
        match response {
            Message::Text(text) => {
                let response: Response =
                    serde_json::from_str(&text).inspect_err(|e| println!("{e:?}: {text}"))?;

                match response {
                    Response::Error { error, .. } => {
                        Err(ClientError::ErrorResponse(error))
                    }
                   Response::Result { result, .. } => Ok(result),
                }
            }
            _ => Err(ClientError::UnexpectedResponse),
        }
    }
}

#[derive(Error, Debug)]
pub enum ClientError {
    #[error("WebSocket error")]
    WebSocketError(#[from] tokio_tungstenite::tungstenite::Error),

    #[error("JSON decode error: {0}")]
    JsonDecodeError(#[from] serde_json::Error),

    #[error("No response")]
    NoResponse,

    #[error("Unexpected response")]
    UnexpectedResponse,

    #[error("Error response from Ogmios: {0:?}")]
    ErrorResponse(ErrorResponse),
}
