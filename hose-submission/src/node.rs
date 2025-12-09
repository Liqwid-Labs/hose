use std::path::PathBuf;

use pallas::applying::{CertState, Environment, UTxOs, ValidationResult};
use pallas::ledger::traverse::{Era, MultiEraTx};
use pallas::network::facades;
use pallas::network::miniprotocols::localstate::queries_v16::{get_chain_point, get_current_era};
use pallas::network::miniprotocols::localtxsubmission::EraTx;
use pallas::network::miniprotocols::localtxsubmission::Response;
use pallas::network::miniprotocols::Point;

use super::SubmitTx;
use hose_primitives::NetworkId;

/// A client that uses NodeClient directly and validates locally.
pub struct NodeClient<'a> {
    pub network: NetworkId,
    pub socket_path: PathBuf,
    pub betterfrost_client: &'a betterfrost_client::v0::Client,
}

impl<'a> NodeClient<'a> {
    pub fn new(
        network: NetworkId,
        socket_path: PathBuf,
        betterfrost_client: &'a betterfrost_client::v0::Client,
    ) -> Self {
        Self {
            network,
            socket_path,
            betterfrost_client,
        }
    }
}

impl SubmitTx for NodeClient<'_> {
    type Error = anyhow::Error;

    async fn submit_tx(&mut self, cbor: &[u8]) -> std::result::Result<(), Self::Error> {
        let mut client =
            facades::NodeClient::connect(&self.socket_path, self.network.magic().into()).await?;

        let statequery = client.statequery();
        statequery.acquire(None).await?;
        let era = get_current_era(statequery).await?;
        let chain_tip_slot = match get_chain_point(statequery).await? {
            Point::Origin => panic!("chain tip is not known"),
            Point::Specific(slot, _) => slot,
        };
        statequery.send_release().await?;

        // HACK: Both 0 and 1 are mapped to Byron. Why +1?
        let named_era = Era::try_from(era + 1)?;

        println!("Current chain tip slot: {:?}", chain_tip_slot);
        println!("Current era: {}", named_era);

        let multi_era_tx = MultiEraTx::decode_for_era(named_era, cbor)?;

        let utxos = query_utxos(&multi_era_tx, self.betterfrost_client).await?;

        let validation_environment = Environment {
            block_slot: chain_tip_slot,
            prot_magic: self.network.magic(),
            network_id: self.network.into(),
            prot_params: self.network.into(),
            acnt: None,
        };

        let validation_result = validate_tx(validation_environment, utxos, multi_era_tx)?;
        println!("{:?}", validation_result);

        // Actually submitting the transaction
        let response = client
            .submission()
            .submit_tx(EraTx(era, cbor.to_vec()))
            .await?;

        match response {
            Response::Accepted => println!("OK."),
            Response::Rejected(reason) => println!("Rejected: {:?}", hex::encode(reason.0)),
        };

        Ok(())
    }
}

async fn query_utxos<'a>(
    tx: &MultiEraTx<'a>,
    betterfrost: &betterfrost_client::v0::Client,
) -> anyhow::Result<UTxOs<'a>> {
    let refs = tx
        .consumes()
        .iter()
        .map(|utxo| (*utxo.hash(), utxo.index() as u32))
        .collect::<Vec<_>>();

    let utxos = UTxOs::new();

    for (tx_hash, _idx) in refs {
        // TODO: populate utxos from betterfrost
        let _res = betterfrost
            .tx_inputs_outputs_by_hash(hex::encode(tx_hash))
            .await
            // FIXME!
            .unwrap();
    }

    Ok(utxos)
}

fn validate_tx(
    env: Environment,
    utxos: UTxOs,
    multi_era_tx: MultiEraTx,
) -> anyhow::Result<ValidationResult> {
    let mut cert_state = CertState::default();

    Ok(pallas::applying::validate_tx(
        &multi_era_tx,
        0,
        &env,
        &utxos,
        &mut cert_state,
    ))
}
