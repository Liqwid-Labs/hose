use pallas_applying::{self, CertState, UTxOs, ValidationResult};
use pallas_network::facades::NodeClient;
use pallas_network::miniprotocols::localstate::queries_v16::{
    get_block_epoch_number, get_chain_point, get_current_era, get_current_pparams,
};
use pallas_network::miniprotocols::localtxsubmission::{RejectReason, Response};
use pallas_network::miniprotocols::txmonitor::TxId;
use pallas_network::miniprotocols::Point;
use pallas_network::miniprotocols::{localtxsubmission::EraTx, MAINNET_MAGIC};
use pallas_primitives::NetworkId;

use crate::config::Config;

pub async fn query_utxos<'a>(
    tx: &pallas_traverse::MultiEraTx<'a>,
    betterfrost: &betterfrost_client::Client,
) -> anyhow::Result<UTxOs<'a>> {
    let refs = tx
        .consumes()
        .iter()
        .map(|utxo| (*utxo.hash(), utxo.index() as u32))
        .collect::<Vec<_>>();

    let mut utxos = UTxOs::new();

    for (tx_hash, idx) in refs {
        // TODO: populate utxos from betterfrost
        let res = betterfrost
            .tx_inputs_outputs_by_hash(hex::encode(tx_hash))
            .await
            // FIXME!
            .unwrap();
    }

    Ok(utxos)
}

pub fn validate_tx(
    env: pallas_applying::Environment,
    utxos: UTxOs,
    cbor: &[u8],
    multi_era_tx: pallas_traverse::MultiEraTx,
) -> anyhow::Result<ValidationResult> {
    let mut cert_state = CertState::default();

    Ok(pallas_applying::validate_tx(
        &multi_era_tx,
        0,
        &env,
        &utxos,
        &mut cert_state,
    ))
}

pub async fn submit_transaction(
    config: &Config,
    betterfrost_client: &betterfrost_client::Client,
    id: TxId,
    cbor: &[u8],
) -> anyhow::Result<()> {
    println!("Submitting transaction with id {}", id);

    let mut client = NodeClient::connect("/tmp/node.socket", MAINNET_MAGIC).await?;

    let statequery = client.statequery();
    statequery.acquire(None).await?;
    let era = get_current_era(statequery).await?;
    let protocol_params = get_current_pparams(statequery, era).await?;
    let chain_tip_slot = match get_chain_point(statequery).await? {
        Point::Origin => panic!("chain tip is not known"),
        Point::Specific(slot, _) => slot,
    };
    statequery.send_release().await?;

    // HACK: Both 0 and 1 are mapped to Byron. Why +1?
    let named_era = pallas_traverse::Era::try_from(era + 1)?;

    println!("Current chain tip slot: {:?}", chain_tip_slot);
    println!("Current era: {}", named_era);

    let network_magic = config.network.network_magic();

    let multi_era_tx = pallas_traverse::MultiEraTx::decode_for_era(named_era, &cbor)?;

    let utxos = query_utxos(&multi_era_tx, betterfrost_client).await?;

    let validation_environment = pallas_applying::Environment {
        block_slot: chain_tip_slot,
        prot_magic: network_magic,
        network_id: config.network.clone().into(),
        prot_params: todo!("Get protocol params"),
        acnt: None,
    };

    let validation_result = validate_tx(validation_environment, utxos, cbor, multi_era_tx)?;

    println!("{:?}", validation_result);

    // Actually submitting the transaction
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
