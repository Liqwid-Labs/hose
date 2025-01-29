#![allow(clippy::from_over_into)]

use std::collections::HashMap;

use super::client::*;
use super::types::{ Request, RequestMethod };
use crate::QueryUTxOs;
use hose_primitives::{TxHash, Output};
use pallas::ledger::addresses::Address;
use serde_json::json;
use serde::Deserialize;

impl QueryUTxOs for OgmiosClient {
    type Error = ClientError;
    async fn query_utxos(
            &mut self,
            addresses: &[Address],
        ) -> std::result::Result<Vec<Output>, Self::Error> {
        let response = self.request(QueryUTxOsRequest::new(addresses).into()).await?;
        let utxos: QueryUTxOsResponse = serde_json::from_value(response)?;
        Ok(utxos.into_iter().map(|u| u.into()).collect())
    }
}

pub struct QueryUTxOsRequest {
    addresses: Vec<Address>,
}

impl QueryUTxOsRequest {
    pub fn new(addresses: &[Address]) -> Self {
        Self { addresses: addresses.to_vec() }
    }
}

impl Into<Request> for QueryUTxOsRequest {
    fn into(self) -> Request {
        Request {
            jsonrpc: "2.0".into(),
            method: RequestMethod::QueryLedgerStateUTxO.into(),
            params: json!({ "addresses": self.addresses.into_iter().map(|a| a.to_string()).collect::<Vec<_>>() }),
        }
    }
}

pub type QueryUTxOsResponse = Vec<QueryUTxO>;

#[derive(Deserialize)]
pub struct QueryUTxO {
    address: String,
    transaction: QueryUTxOTransaction,
    index: u64,
    value: HashMap<String, HashMap<String, u64>>,
}

#[derive(Deserialize)]
pub struct QueryUTxOTransaction {
    id: TxHash,
}

impl Into<Output> for QueryUTxO {
    fn into(self) -> Output {
        let QueryUTxO { address, transaction, index, value } = self;
        Output {
            // TODO: don't unwrap
            address: Address::from_bech32(&address).unwrap(),
            tx_hash: transaction.id,
            txo_index: index,
            lovelace: *value.get("ada").and_then(|v| v.get("lovelace")).unwrap_or(&0),
            // TODO: parse assets
            assets: HashMap::new(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_query_utxos_request() {
        let address = "addr1q9jd3kjmlgxpccpn004kugp8k52emgrt36kty2un39pay08pxyw706r8chwpthhx6l370dv8xrmgch2u384dk9hrrcwq3tdsr7";
        let address = pallas::ledger::addresses::Address::from_bech32(address).unwrap();
        let request = QueryUTxOsRequest::new(&[address]);
        let mut client: OgmiosClient = OgmiosClient::new("ws://mainnet-ogmios:1337".into()).await.unwrap();
        let response = client.request(request.into()).await.unwrap();
        println!("{:?}", response);
    }
}
