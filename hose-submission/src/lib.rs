mod node;
mod ogmios;

use hose_primitives::UTxO;
pub use node::NodeClient;
pub use ogmios::OgmiosClient;
use pallas::ledger::{addresses::Address, traverse::MultiEraOutput};

pub trait SubmitTx {
    type Error;
    fn submit_tx(
        &mut self,
        cbor: &[u8],
    ) -> impl std::future::Future<Output = Result<(), Self::Error>>;
}

pub trait EvaluateTx {
    type Error;
    fn evaluate_tx(
        &mut self,
        cbor: &[u8],
    ) -> impl std::future::Future<Output = Result<Vec<ScriptEvaluation>, Self::Error>>;
}

pub trait QueryUTxOs {
    type Error;
    fn query_utxos(
        &mut self,
        addresses: &[Address],
    ) -> impl std::future::Future<Output = Result<Vec<UTxO>, Self::Error>>;
}

pub enum ScriptType {
    /// Transaction inputs
    Spend,
    /// Transaction certificate
    Certificate,
    /// Transaction monetary policies
    Mint,
    /// Transaction rewards withdrawal
    Withdrawal,
}

pub struct ScriptEvaluation {
    pub script_type: ScriptType,
    pub script_index: usize,
    pub memory_budget: u64,
    pub cpu_budget: u64,
}
