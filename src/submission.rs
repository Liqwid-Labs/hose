use pallas_network::facades::NodeClient;
use pallas_network::miniprotocols::localstate::queries_v16::{
    get_current_era, get_current_pparams,
};
use pallas_network::miniprotocols::localtxsubmission::{RejectReason, Response};
use pallas_network::miniprotocols::txmonitor::TxId;
use pallas_network::miniprotocols::{localtxsubmission::EraTx, MAINNET_MAGIC};

// pub fn validate_tx(cbor: &[u8], era: pallas_traverse::Era) -> anyhow::Result<()> {
//     let multi_era_tx = pallas_traverse::MultiEraTx::decode_for_era(era, &cbor)?;
//
//     let context = ValidationContext {
//         block_slot: args.block_slot,
//         prot_magic: config.upstream.network_magic as u32,
//         network_id: args.network_id,
//         prot_params: pparams,
//         acnt: None,
//     };
//
//     pallas_applying::validate_tx(multi_era_tx, 0, &context);
//     Ok(())
// }

pub async fn submit_transaction(id: TxId, cbor: &[u8]) -> anyhow::Result<()> {
    println!("Submitting transaction with id {}", id);

    let mut client = NodeClient::connect("/tmp/node.socket", MAINNET_MAGIC).await?;

    let statequery = client.statequery();
    statequery.acquire(None).await?;
    let era = get_current_era(statequery).await?;
    let protocol_params = get_current_pparams(statequery, era).await?;
    statequery.send_release().await?;

    println!("Protocol params: {:?}", protocol_params);

    // HACK: Both 0 and 1 are mapped to Byron. Why +1?
    let named_era = pallas_traverse::Era::try_from(era + 1)?;

    println!("Current era: {}", named_era);

    let response = client
        .submission()
        .submit_tx(EraTx(era, cbor.to_vec()))
        .await?;

    match response {
        Response::Accepted => println!("OK."),
        Response::Rejected(reason) => println!("Rejected: {:?}", hex::encode(reason.0)),
    }

    let monitor = client.monitor();
    monitor.acquire().await?;
    let res = monitor.query_has_tx(id).await?;
    println!("has_tx: {:?}", res);
    monitor.release().await?;

    Ok(())
}
