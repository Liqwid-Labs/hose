use std::collections::HashMap;
use std::sync::Arc;

use futures_util::{SinkExt, StreamExt};
use thiserror::Error;
use tokio::net::TcpStream;
use tokio::sync::{oneshot, Mutex};
use tokio_tungstenite::tungstenite::protocol::Message;
use tokio_tungstenite::{MaybeTlsStream, WebSocketStream};
use uuid::Uuid;

use super::types::*;

#[derive(Debug)]
pub struct OgmiosClient {
    ws: Arc<Mutex<WebSocketStream<MaybeTlsStream<TcpStream>>>>,
    pending_requests:
        Arc<Mutex<HashMap<String, oneshot::Sender<Result<serde_json::Value, ClientError>>>>>,
}

impl OgmiosClient {
    pub async fn new(ogmios_url: &str) -> Result<Self, ClientError> {
        let (ws, _) = tokio_tungstenite::connect_async(ogmios_url).await?;
        let ws = Arc::new(Mutex::new(ws));
        let pending_requests = Arc::new(Mutex::new(HashMap::new()));
        let client = Self {
            ws,
            pending_requests,
        };

        // Spawn a task to handle incoming messages
        let ws = client.ws.clone();
        let pending_requests = client.pending_requests.clone();
        tokio::spawn(async move { OgmiosClient::handle_responses(ws, pending_requests).await });

        Ok(client)
    }

    async fn handle_responses(
        ws: Arc<Mutex<WebSocketStream<MaybeTlsStream<TcpStream>>>>,
        pending_requests: Arc<
            Mutex<HashMap<String, oneshot::Sender<Result<serde_json::Value, ClientError>>>>,
        >,
    ) {
        loop {
            let msg_result = {
                let mut ws = ws.lock().await;
                ws.next().await
            };

            match msg_result {
                Some(Ok(Message::Text(text))) => {
                    if let Ok(response) = serde_json::from_str::<Response>(&text) {
                        if let Some(id) = response.id() {
                            let mut requests = pending_requests.lock().await;
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
                // Websocket closed
                None | Some(Ok(Message::Close(_))) => {
                    // Close all pending requests
                    for (_, sender) in pending_requests.lock().await.drain() {
                        let _ = sender.send(Err(ClientError::NoResponse));
                    }
                    break;
                },
                _ => continue,
            }
        }
    }

    pub async fn request(
        &mut self,
        mut request: Request,
    ) -> Result<serde_json::Value, ClientError> {
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

        let response = client.request(Request {
            jsonrpc: "2.0".to_string(),
            method: "queryLedgerState/tip".to_string(),
            id: Some("testing".to_string()),
            params: None,
        }).await.unwrap();

        assert!(response.get("slot").is_some())
    }
}
