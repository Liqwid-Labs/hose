#[cfg(feature = "node")]
pub mod node;
#[cfg(feature = "ogmios")]
pub mod ogmios;

#[cfg(feature = "node")]
pub use node::NodeClient;
#[cfg(feature = "ogmios")]
pub use ogmios::OgmiosClient;

pub trait SubmitTx {
    type Error;
    fn submit_tx(
        &mut self,
        cbor: &[u8],
    ) -> impl std::future::Future<Output = std::result::Result<(), Self::Error>>;
}

pub trait EvaluateTx {
    type Error;
    fn evaluate_tx(
        &mut self,
        cbor: &[u8],
    ) -> impl std::future::Future<Output = std::result::Result<Vec<ScriptEvaluation>, Self::Error>>;
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
