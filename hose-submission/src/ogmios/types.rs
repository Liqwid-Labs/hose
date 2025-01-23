use serde::{Deserialize, Serialize};

pub enum RequestMethod {
    SubmitTransaction,
    EvaluateTransaction,
}

impl From<RequestMethod> for String {
    fn from(method: RequestMethod) -> Self {
        match method {
            RequestMethod::SubmitTransaction => "submitTransaction".into(),
            RequestMethod::EvaluateTransaction => "evaluateTransaction".into(),
        }
    }
}

#[derive(Serialize, Debug)]
pub struct Request {
    pub jsonrpc: String,
    pub method: String,
    pub params: serde_json::Value,
}

#[derive(Deserialize, Debug)]
pub struct OgmiosError {
    pub code: u32,
    pub data: serde_json::Value,
}

#[derive(Deserialize, Debug)]
#[serde(untagged)]
pub enum OgmiosResponse {
    Error { error: OgmiosError },
    Result { result: serde_json::Value },
}
