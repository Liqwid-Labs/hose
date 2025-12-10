//! Coin selection algorithms, based on [CIP-2](https://cips.cardano.org/cip/CIP-2).

use hydrant::primitives::Asset;

use crate::fetcher::AddressUtxo;

mod largest_first;
mod random_improve;

pub trait Selector {
    fn select(
        &self,
        utxos: &[AddressUtxo],
        lovelace: u64,
        assets: &[Asset],
    ) -> anyhow::Result<Vec<AddressUtxo>>;
}
