//! UTxO fetching for a given address

use hydrant::primitives::{TxOutput, TxOutputPointer};

#[derive(Debug, Clone)]
pub struct AddressUtxo {
    pointer: TxOutputPointer,
    output: TxOutput,
}

pub trait Fetcher {
    fn address_utxos(&self, address: String) -> anyhow::Result<Vec<AddressUtxo>>;
}
