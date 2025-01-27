use pallas::{applying::MultiEraProtocolParameters, ledger::primitives};
use std::str::FromStr;

use crate::get_protocol_parameters;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NetworkId {
    Mainnet,
    Testnet,
}

impl NetworkId {
    pub fn magic(&self) -> u32 {
        match self {
            NetworkId::Mainnet => 764824073,
            NetworkId::Testnet => 2,
        }
    }

    pub fn parameters(self) -> MultiEraProtocolParameters {
        get_protocol_parameters(self)
    }
}

impl From<NetworkId> for u8 {
    fn from(val: NetworkId) -> Self {
        match val {
            NetworkId::Mainnet => 1,
            NetworkId::Testnet => 0,
        }
    }
}

impl From<u8> for NetworkId {
    fn from(val: u8) -> Self {
        match val {
            1 => NetworkId::Mainnet,
            0 => NetworkId::Testnet,
            _ => panic!("unknown network id {val}"),
        }
    }
}

impl From<NetworkId> for primitives::NetworkId {
    fn from(val: NetworkId) -> Self {
        match val {
            NetworkId::Mainnet => primitives::NetworkId::Mainnet,
            NetworkId::Testnet => primitives::NetworkId::Testnet,
        }
    }
}

impl FromStr for NetworkId {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "Mainnet" => Ok(NetworkId::Mainnet),
            "Testnet" => Ok(NetworkId::Testnet),
            _ => Err(format!("unknown network {}", s)),
        }
    }
}
