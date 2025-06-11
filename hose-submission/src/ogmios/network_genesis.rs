use crate::ogmios::types::{Request, RequestMethod};
use serde::{Deserialize, Serialize};
use serde_json::json;

#[derive(Debug, Clone, Copy, Deserialize, Serialize)]
pub enum Era {
    #[serde(rename = "byron")]
    Byron,
    #[serde(rename = "shelley")]
    Shelley,
    #[serde(rename = "alonzo")]
    Alonzo,
    #[serde(rename = "conway")]
    Conway,
}

pub struct NetworkGenesisConfigurationRequest {
    pub era: Era,
}

impl NetworkGenesisConfigurationRequest {
    pub fn new(era: Era) -> Self {
        Self { era }
    }
}

impl From<NetworkGenesisConfigurationRequest> for Request {
    fn from(val: NetworkGenesisConfigurationRequest) -> Self {
        Request {
            jsonrpc: "2.0".into(),
            method: RequestMethod::NetworkGenesisConfiguration.into(),
            id: None,
            params: Some(json!({ "era": val.era })),
        }
    }
}
