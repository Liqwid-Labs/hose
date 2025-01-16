use pallas::applying::{CertState, Environment, UTxOs, ValidationResult};
use pallas::ledger::primitives::NetworkId;
use pallas::ledger::traverse::{Era, MultiEraTx};
use pallas::network::facades::NodeClient;
use pallas::network::miniprotocols::localstate::queries_v16::{
    get_chain_point, get_current_era, get_current_pparams,
};
use pallas::network::miniprotocols::localtxsubmission::{RejectReason, Response};
use pallas::network::miniprotocols::txmonitor::TxId;
use pallas::network::miniprotocols::Point;
use pallas::network::miniprotocols::{self, localtxsubmission::EraTx, MAINNET_MAGIC};

use crate::config::Config;
use crate::params;

use super::SubmitTx;

/// A client that uses NodeClient directly and validates locally.
pub struct DirectToNode<'a> {
    pub config: &'a Config,
    pub betterfrost_client: &'a betterfrost_client::Client,
}

impl<'a> DirectToNode<'a> {
    pub fn new(config: &'a Config, betterfrost_client: &'a betterfrost_client::Client) -> Self {
        Self {
            config,
            betterfrost_client,
        }
    }
}

impl SubmitTx for DirectToNode<'_> {
    type Error = anyhow::Error;

    async fn submit_tx(
        &mut self,
        tx_id: TxId,
        cbor: &[u8],
    ) -> std::result::Result<TxId, Self::Error> {
        println!("Submitting transaction with id {}", &tx_id);

        let mut client = NodeClient::connect("/tmp/node.socket", MAINNET_MAGIC).await?;

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

        let network_magic = self.config.network.network_magic();

        let multi_era_tx = MultiEraTx::decode_for_era(named_era, cbor)?;

        let utxos = query_utxos(&multi_era_tx, self.betterfrost_client).await?;

        let validation_environment = Environment {
            block_slot: chain_tip_slot,
            prot_magic: network_magic,
            network_id: self.config.network.clone().into(),
            prot_params: params::get_protocol_parameters(self.config.network)?,
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
        }

        let monitor = client.monitor();
        monitor.acquire().await?;
        let res = monitor.query_has_tx(tx_id.clone()).await?;
        println!("has_tx: {:?}", res);
        monitor.release().await?;

        Ok(tx_id)
    }
}

async fn query_utxos<'a>(
    tx: &MultiEraTx<'a>,
    betterfrost: &betterfrost_client::Client,
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
