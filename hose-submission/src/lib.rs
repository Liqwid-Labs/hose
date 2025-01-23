use pallas::network::miniprotocols::txmonitor::TxId;

mod ogmios;
mod direct_to_node;

pub use ogmios::OgmiosClient;
pub use direct_to_node::DirectToNode;

pub trait SubmitTx {
    type Error;

    async fn submit_tx(
        // FIXME: I don't know how to make it not mut
        &mut self,
        tx_id: TxId,
        cbor: &[u8],
    ) -> std::result::Result<TxId, Self::Error>;
}
