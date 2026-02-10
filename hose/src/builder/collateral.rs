use std::cmp::Reverse;
use std::sync::Arc;

use anyhow::{Result, ensure};
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

    pub(crate) async fn collateral_inputs(
        &self,
        indexer: &Arc<Mutex<UtxoIndexer>>,
        possible_utxos: &[TxOutput],
        pparams: &ProtocolParams,
        fee: u64,
    ) -> Result<Vec<Input>> {
        if !self.body.collateral_inputs.is_empty() || !self.requires_collateral(indexer).await? {
            return Ok(vec![]);
        }

        // note: collateral_percentage is a percent (e.g., 150), so divide by 100 to get the multiplier
        let required_lovelace =
            ((fee as f64) * pparams.collateral_percentage / 100.0).ceil() as u64;

        let max_collateral_inputs = if pparams.max_collateral_inputs > 0 {
            pparams.max_collateral_inputs as usize
        } else {
            3 // NOTE: Current Cardano protocol limits this to 3 (Feb 9, 2026)
        };

        select_collateral(possible_utxos, required_lovelace, max_collateral_inputs)
    }
}

fn select_collateral(
    possible_utxos: &[TxOutput],
    required_lovelace: u64,
    max_collateral_inputs: usize,
) -> Result<Vec<Input>> {
    // Filter for UTXOs that are ADA-only and have no scripts
    let mut collateral_utxos = possible_utxos
        .iter()
        .filter(|utxo| utxo.assets.is_empty() && utxo.script.is_none())
        .collect::<Vec<_>>();

    // Try to find a single UTXO that is large enough (smallest-is-enough strategy)
    let mut single_utxos = collateral_utxos
        .iter()
        .filter(|utxo| utxo.lovelace > required_lovelace)
        .collect::<Vec<_>>();
    single_utxos.sort_unstable_by_key(|utxo| utxo.lovelace);

    if let Some(utxo) = single_utxos.first() {
        let pointer: TxOutputPointer = (**utxo).clone().into();
        return Ok(vec![pointer.into()]);
    }

    // If no single UTXO is enough, accumulate multiple (largest-first strategy)
    collateral_utxos.sort_unstable_by_key(|utxo| Reverse(utxo.lovelace));

    let mut selected_inputs = vec![];
    let mut accumulated_lovelace = 0;

    for utxo in collateral_utxos {
        accumulated_lovelace += utxo.lovelace;
        let pointer: TxOutputPointer = (*utxo).clone().into();
        selected_inputs.push(pointer.into());

        if accumulated_lovelace > required_lovelace {
            break;
        }

        if selected_inputs.len() >= max_collateral_inputs {
            break;
        }
    }

    ensure!(
        accumulated_lovelace > required_lovelace,
        "no utxos large enough for collateral (needs {}, found {})",
        required_lovelace,
        accumulated_lovelace
    );

    Ok(selected_inputs)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::primitives::Hash;

    #[test]
    fn test_select_collateral_single() {
        let utxos = vec![
            TxOutput {
                hash: Hash([0u8; 32]),
                index: 0,
                address: vec![0; 29],
                lovelace: 100,
                assets: Default::default(),
                script: None,
                datum_hash: None,
            },
            TxOutput {
                hash: Hash([0u8; 32]),
                index: 1,
                address: vec![0; 29],
                lovelace: 200,
                assets: Default::default(),
                script: None,
                datum_hash: None,
            },
        ];

        let selected = select_collateral(&utxos, 150, 3).unwrap();
        assert_eq!(selected.len(), 1);
    }

    #[test]
    fn test_select_collateral_multiple() {
        let utxos = vec![
            TxOutput {
                hash: Hash([0u8; 32]),
                index: 0,
                address: vec![0; 29],
                lovelace: 100,
                assets: Default::default(),
                script: None,
                datum_hash: None,
            },
            TxOutput {
                hash: Hash([0u8; 32]),
                index: 1,
                address: vec![0; 29],
                lovelace: 100,
                assets: Default::default(),
                script: None,
                datum_hash: None,
            },
            TxOutput {
                hash: Hash([0u8; 32]),
                index: 2,
                address: vec![0; 29],
                lovelace: 100,
                assets: Default::default(),
                script: None,
                datum_hash: None,
            },
        ];

        let selected = select_collateral(&utxos, 250, 3).unwrap();
        assert_eq!(selected.len(), 3);
    }

    #[test]
    fn test_select_collateral_insufficient() {
        let utxos = vec![TxOutput {
            hash: Hash([0u8; 32]),
            index: 0,
            address: vec![0; 29],
            lovelace: 100,
            assets: Default::default(),
            script: None,
            datum_hash: None,
        }];

        let res = select_collateral(&utxos, 150, 3);
        assert!(res.is_err());
    }
}
