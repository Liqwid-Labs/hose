//! High-level transaction builder API

use pallas::ledger::addresses::Address;
use pallas::txbuilder::StagingTransaction;

use crate::providers::ogmios::OgmiosClient;
use crate::submitter::Submitter;

pub struct TxBuilder {
    ogmios: OgmiosClient,
    body: StagingTransaction,
    change_address: Address,
}
