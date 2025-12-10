//! Transaction script evaluation for estimating fees

// mod n2c;
use hydrant::primitives::ScriptHash;
use pallas::ledger::traverse::MultiEraTx;

pub trait Evaluator {
    async fn evaluate_tx(&self, tx: &MultiEraTx<'_>) -> anyhow::Result<Vec<Evaluation>>;
}

/// Evaluation of a single script for a transaction
pub struct Evaluation {
    validator: ScriptHash,
    cpu: u64,
    memory: u64,
}
