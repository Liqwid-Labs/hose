use super::client::{ OgmiosClient, OgmiosClientError };
use super::types::{ Request, RequestMethod };
use crate::SubmitTx;
use serde_json::json;

impl SubmitTx for OgmiosClient {
    type Error = OgmiosClientError;

    async fn submit_tx(&mut self, cbor: &[u8]) -> std::result::Result<(), Self::Error> {
        self.request(SubmitRequest::new(cbor).into()).await.map(|_| ())
    }
}

pub struct SubmitRequest<'a> {
    transaction: &'a [u8],
}

impl<'a> SubmitRequest<'a> {
    pub fn new(transaction: &'a [u8]) -> Self {
        Self { transaction }
    }
}

impl Into<Request> for SubmitRequest<'_> {
    fn into(self) -> Request {
        Request {
            jsonrpc: "2.0".into(),
            method: RequestMethod::SubmitTransaction.into(),
            params: json!({ "transaction": { "cbor": hex::encode(self.transaction) } }),
        }
    }
}
