//! High-level transaction builder API

use hydrant::primitives::Address;
use pallas::txbuilder::StagingTransaction;

use crate::{evaluator::Evaluator, fetcher::Fetcher, selector::Selector, submitter::Submitter};

pub struct TxBuilder<Eval: Evaluator> {
    pub fetcher: Box<dyn Fetcher>,
    pub selector: Box<dyn Selector>,
    pub evaluator: Box<Eval>,
    pub submitter: Box<dyn Submitter>,
    body: StagingTransaction,
    change_address: Address,
}
