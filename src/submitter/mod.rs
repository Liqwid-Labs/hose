//! Transaction submission with concrete error types

pub trait Submitter {
    fn submit_tx(&self, tx: &[u8]) -> anyhow::Result<String>;
    // fn protocol_params(&self) -> anyhow::Result<MultiEraProtocolParameters>;
}
