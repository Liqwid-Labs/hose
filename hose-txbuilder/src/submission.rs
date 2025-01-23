use pallas::network::miniprotocols::txmonitor::TxId;

pub mod direct_to_node;
pub mod ogmios;

pub trait SubmitTx {
    type Error;

    async fn submit_tx(
        // FIXME: I don't know how to make it not mut
        &mut self,
        tx_id: TxId,
        cbor: &[u8],
    ) -> std::result::Result<TxId, Self::Error>;
}
