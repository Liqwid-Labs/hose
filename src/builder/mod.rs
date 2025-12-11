//! High-level transaction builder API

use pallas::ledger::addresses::Address;
use pallas::txbuilder::StagingTransaction;

use crate::ogmios::OgmiosClient;

pub struct TxBuilder {
    ogmios: OgmiosClient,
    body: StagingTransaction,
    change_address: Address,
}
