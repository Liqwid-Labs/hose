use std::sync::Arc;

use anyhow::{Context, Result};
use hydrant::UtxoIndexer;
use hydrant::primitives::{TxOutput, TxOutputPointer};
use ogmios_client::method::pparams::ProtocolParams;
use pallas::ledger::addresses::Address;
use tokio::sync::Mutex;

use super::TxBuilder;
use crate::primitives::Input;

impl TxBuilder {
    fn non_collateral_inputs(&self) -> Vec<TxOutputPointer> {
        self.body
            .inputs
            .iter()
            .chain(self.body.reference_inputs.iter())
            .map(Into::into)
            .collect::<Vec<_>>()
    }

    async fn requires_collateral(&self, indexer: &Arc<Mutex<UtxoIndexer>>) -> Result<bool> {
        // any mints (minting policy) or scripts (inline)
        if !self.body.mint.is_empty() || !self.body.scripts.is_empty() {
            return Ok(true);
        }

        // any input comes from a script or contains a script (validator)
        let input_utxos = {
            let indexer = indexer.lock().await;
            indexer.utxos(&self.non_collateral_inputs())?
        };
        if input_utxos.iter().any(|input| {
            Address::from_bytes(&input.address).unwrap().has_script() || input.script.is_some()
        }) {
            return Ok(true);
        }

        Ok(false)
    }

    pub(crate) async fn collateral_input(
        &self,
        indexer: &Arc<Mutex<UtxoIndexer>>,
        possible_utxos: &[TxOutput],
        pparams: &ProtocolParams,
        fee: u64,
    ) -> Result<Option<Input>> {
        if !self.body.collateral_inputs.is_empty() || !self.requires_collateral(indexer).await? {
            return Ok(None);
        }

        let required_lovelace = ((fee as f64) * pparams.collateral_percentage).ceil() as u64;

        // TODO: support multiple collateral inputs
        let mut collateral_utxos = possible_utxos
            .iter()
            .filter(|utxo| {
                utxo.lovelace > required_lovelace && utxo.assets.is_empty() && utxo.script.is_none()
            })
            .collect::<Vec<_>>();
        collateral_utxos.sort_unstable_by_key(|utxo| utxo.lovelace);
        let collateral_utxo = *collateral_utxos
            .first()
            .context("no utxos large enough for collateral")?;
        let collateral_utxo_pointer: TxOutputPointer = collateral_utxo.into();
        Ok(Some(collateral_utxo_pointer.into()))
    }
}
