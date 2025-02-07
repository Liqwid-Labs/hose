use std::cell::RefCell;
use std::collections::HashMap;
use std::sync::Arc;

use futures_util::future::Pending;
use futures_util::{SinkExt, StreamExt};
use serde::Deserialize;
use thiserror::Error;
use tokio::net::TcpStream;
use tokio::sync::{oneshot, Mutex};
use tokio_tungstenite::tungstenite::protocol::Message;
use tokio_tungstenite::tungstenite::Utf8Bytes;
use tokio_tungstenite::{tungstenite, MaybeTlsStream, WebSocketStream};
use uuid::Uuid;

use super::types::*;

type PendingRequests =
Arc<Mutex<HashMap<String, oneshot::Sender<Result<serde_json::Value, ClientError>>>>>;

#[derive(Debug, Clone)]
struct ClientConnection {
    ws: Arc<Mutex<WebSocketStream<MaybeTlsStream<TcpStream>>>>,
    pending_requests: PendingRequests,
}

impl ClientConnection {
    pub fn new(ws: WebSocketStream<MaybeTlsStream<TcpStream>>) -> Self {
        let pending_requests = Arc::new(Mutex::new(HashMap::new()));
        Self {
            ws: Arc::new(Mutex::new(ws)),
            pending_requests,
        }
    }

    async fn handle_response(&self, text: Utf8Bytes) {
        if let Ok(response) = serde_json::from_str::<Response>(&text) {
            if let Some(id) = response.id() {
                let mut requests = self.pending_requests.lock().await;
                if let Some(sender) = requests.remove(&id) {
                    let result = match response {
                        Response::Error { error, .. } => {
                            Err(ClientError::ErrorResponse(error))
                        }
                        Response::Result { result, .. } => Ok(result),
                    };
                    let _ = sender.send(result);
                }
            }
        }
    }

    async fn handle_responses(&self) {
        loop {
            let msg_result = {
                let mut ws = self.ws.lock().await;
                ws.next().await
            };

            match msg_result {
                Some(Ok(Message::Text(text))) => {
                    self.handle_response(text).await;
                }
                // Websocket closed
                None | Some(Ok(Message::Close(_))) => {
                    // Close all pending requests
                    for (_, sender) in self.pending_requests.lock().await.drain() {
                        let _ = sender.send(Err(ClientError::NoResponse));
                    }
                    break;
                }
                _ => {
                    continue;
                }
            }
        }
    }

    pub async fn request(&self, mut request: Request) -> Result<serde_json::Value, ClientError> {
        let id = request.id.clone().unwrap_or(Uuid::new_v4().to_string());
        request.id = Some(id.clone());

        // Create a channel for receiving the response
        let (sender, receiver) = oneshot::channel();
        self.pending_requests.lock().await.insert(id, sender);

        // Send the request
        let request_msg = Message::Text(serde_json::to_string(&request)?.into());
        self.ws.lock().await.send(request_msg).await?;

        // Wait for the response
        receiver.await.map_err(|_| ClientError::NoResponse)?
    }
}

#[derive(Debug)]
pub struct OgmiosClient {
    client_connection: Arc<ClientConnection>,
}

impl OgmiosClient {
    pub async fn new(ogmios_url: &str) -> Result<Self, ClientError> {
        let (ws, _) = tokio_tungstenite::connect_async(ogmios_url).await?;
        let client_connection = ClientConnection::new(ws);

        let client = Self {
            client_connection: Arc::new(client_connection.clone()),
        };

        tokio::spawn(async move { client_connection.handle_responses().await });

        Ok(client)
    }

    pub async fn request(&self, request: Request) -> Result<serde_json::Value, ClientError> {
        self.client_connection.request(request).await
    }

    pub async fn next_block(&self) -> Result<NextBlockResponse, ClientError> {
        let req = self
            .request(Request {
                jsonrpc: "2.0".to_string(),
                method: "nextBlock".to_string(),
                params: None,
                id: None,
            })
        .await?;

        Ok(serde_json::from_value(req)?)
    }

    pub async fn query_ledger_tip(&self) -> std::result::Result<Tip, ClientError> {
        let response = self
            .request(Request {
                jsonrpc: "2.0".to_string(),
                method: "queryLedgerState/tip".to_string(),
                id: None,
                params: None,
            })
        .await?;
        Ok(serde_json::from_value::<Tip>(response)?)
    }
}

#[derive(Debug, Deserialize, PartialEq, Eq)]
pub struct Tip {
    pub slot: u64,
    pub id: String,
}

#[derive(Debug, Deserialize, PartialEq, Eq)]
pub struct Block {
    pub id: String,
}

#[derive(Debug, Deserialize, PartialEq, Eq)]
#[serde(tag = "direction")]
pub enum NextBlockResponse {
    #[serde(rename = "forward")]
    RollForward { block: Block, tip: Tip },
    #[serde(rename = "backward")]
    RollBackward { tip: Tip },
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

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_request() {
        let mut client = OgmiosClient::new("ws://mainnet-ogmios:1337").await.unwrap();

        let response = client.query_ledger_tip().await;

        assert!(response.is_ok());
    }
}
