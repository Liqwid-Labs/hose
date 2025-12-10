use std::collections::HashSet;

use super::{Evaluation, Evaluator};
use pallas::{
    ledger::{
        primitives::NetworkId,
        traverse::MultiEraTx,
        validate::{
            phase2::{evaluate_tx, script_context::SlotConfig},
            utils::{MultiEraProtocolParameters, UtxoMap},
        },
    },
    network::{
        facades::NodeClient,
        miniprotocols::localstate::queries_v16::{
            get_current_era, get_current_pparams, get_utxo_by_txin,
        },
    },
};

pub struct N2CEvaluator {
    client: NodeClient,
    pparams: MultiEraProtocolParameters,
    slot_config: SlotConfig,
}

impl N2CEvaluator {
    pub async fn new(client: NodeClient, network: NetworkId) -> anyhow::Result<Self> {
        let state_query_client = client.statequery();

        let era = get_current_era(client).await?;
        let pparams = get_current_pparams(client, era).await?;

        Self {
            client,
            pparams,
            slot_config: Default::default(),
        }
    }
}

impl Evaluator for N2CEvaluator {
    async fn evaluate_tx(&self, tx: &MultiEraTx<'_>) -> anyhow::Result<Vec<Evaluation>> {
        let inputs = HashSet::from_iter(
            tx.inputs_sorted_set()
                .iter()
                .chain(tx.reference_inputs().iter()),
        );

        let state_query_client = self.client.statequery();
        let utxos = get_utxo_by_txin(
            state_query_client,
            tx.era().into(),
            inputs.iter().map(Into::into).collect(),
        )
        .await?;
        let utxo_map = UtxoMap::new();

        // TODO: get slot config from MultiEraProtocolParameters
        evaluate_tx(tx, &self.pparams, &utxo_map, &self.slot_config)
    }
}
