use std::cmp::Reverse;
use std::sync::Arc;

use anyhow::{Context, Result, ensure};
use hydrant::UtxoIndexer;
use hydrant::primitives::{Assets, AssetsDelta, TxOutput};
use ogmios_client::method::pparams::ProtocolParams;
use tokio::sync::Mutex;

use super::{Output, TxBuilder};
use crate::primitives::Certificate;

impl TxBuilder {
    pub(crate) async fn select_coins(
        &self,
        indexer: &Arc<Mutex<UtxoIndexer>>,
        possible_utxos: &[TxOutput],
        fee: u64,
        pparams: &ProtocolParams,
    ) -> Result<Vec<TxOutput>> {
        let mut selected_utxos = vec![];

        let input_lovelace = self.get_input_lovelace(indexer).await?;
        let input_assets = self.get_input_assets(indexer).await?;

        // Filter utxos already used as inputs
        // TODO: should also filter out utxos with scripts? utxos with datums?
        let mut possible_utxos = possible_utxos
            .iter()
            .filter(|utxo| !self.body.inputs.iter().any(|input| input == *utxo))
            .collect::<Vec<_>>();

        // TODO: consider minted assets
        // TODO: for simplicity, we assume that all assets are included in the change output
        let mut change_output =
            Output::new(self.change_address.clone(), 0).add_assets(input_assets.clone())?;
        change_output.datum = self.change_datum.clone();
        let min_change_lovelace = change_output.min_deposit(pparams)?;

        let registration_deposit = self.get_registration_deposit();
        let deregistration_refund = self.get_deregistration_refund();
        let withdrawal_lovelace = self.get_withdrawal_lovelace();
        let mut required_lovelace =
            (self.get_output_lovelace() + fee + min_change_lovelace + registration_deposit)
                .saturating_sub(input_lovelace + withdrawal_lovelace + deregistration_refund);

        let output_assets: AssetsDelta = self.get_output_assets().into();
        let input_assets: AssetsDelta = input_assets.into();
        let mut required_assets: AssetsDelta =
            output_assets - input_assets - self.body.mint.clone();

        // Select for assets
        while !possible_utxos.is_empty()
            && let Some(asset) = required_assets.only_positive().keys().next()
        {
            // Largest-first by asset ammount
            possible_utxos.sort_by_key(|utxo| Reverse(*utxo.assets.get(asset).unwrap_or(&0)));

            let utxo = possible_utxos.remove(0);
            if utxo.assets.get(asset).unwrap_or(&0) == &0 {
                break;
            }

            required_assets = required_assets - utxo.assets.clone().into();
            selected_utxos.push(utxo.clone());
        }

        // Select for lovelace
        possible_utxos.sort_by_key(|utxo| Reverse(utxo.lovelace)); // Largest-first
        while !possible_utxos.is_empty()
            && (required_lovelace > 0 || (self.body.inputs.is_empty() && selected_utxos.is_empty()))
        {
            let utxo = possible_utxos.remove(0);
            required_lovelace = required_lovelace.saturating_sub(utxo.lovelace);
            selected_utxos.push(utxo.clone());
        }

        ensure!(
            required_lovelace == 0,
            "failed to select coins, wallet doesn't contain enough lovelace (needs {} more)",
            required_lovelace
        );
        ensure!(
            required_assets.only_positive().is_empty(),
            "failed to select coins, wallet doesn't contain enough assets: {:?}",
            required_assets.only_positive()
        );

        Ok(selected_utxos)
    }

    /// Create change output if needed because transaction is not balanced.
    pub(crate) async fn change_output(
        &self,
        indexer: &Arc<Mutex<UtxoIndexer>>,
        fee: u64,
        pparams: &ProtocolParams,
    ) -> Result<Option<Output>> {
        // TODO: consider minted assets
        let input_lovelace = self.get_input_lovelace(indexer).await?;
        let registration_deposit = self.get_registration_deposit();
        let deregistration_refund = self.get_deregistration_refund();
        let withdrawal_lovelace = self.get_withdrawal_lovelace();
        let output_lovelace = self.get_output_lovelace();
        let change_lovelace = (input_lovelace + withdrawal_lovelace + deregistration_refund)
            .saturating_sub(output_lovelace + fee + registration_deposit);

        let input_assets: AssetsDelta = self.get_input_assets(indexer).await?.into();
        let output_assets: AssetsDelta = self.get_output_assets().into();
        let change_assets = input_assets + self.body.mint.clone() - output_assets;
        if !change_assets.only_negative().is_empty() {
            tracing::error!(
                "Negative change assets: {:#?}",
                change_assets.only_negative()
            );
            return Err(anyhow::anyhow!("change cannot be negative"));
        }
        let change_assets = change_assets.only_positive();

        if change_lovelace == 0 && change_assets.only_positive().is_empty() {
            return Ok(None);
        }

        let mut change_output = Output::new(self.change_address.clone(), change_lovelace)
            .add_assets(change_assets.into())
            .context("failed to create change output")?;
        change_output.datum = self.change_datum.clone();

        if change_output.min_deposit(pparams)? > change_output.lovelace {
            return Ok(None);
        }
        Ok(Some(change_output))
    }

    pub(crate) async fn get_input_lovelace(
        &self,
        indexer: &Arc<Mutex<UtxoIndexer>>,
    ) -> Result<u64> {
        let indexer = indexer.lock().await;
        Ok(indexer
            .utxos(&self.body.inputs)?
            .iter()
            .map(|utxo| utxo.lovelace)
            .sum())
    }

    async fn get_input_assets(&self, indexer: &Arc<Mutex<UtxoIndexer>>) -> Result<Assets> {
        let indexer = indexer.lock().await;
        Ok(indexer
            .utxos(&self.body.inputs)?
            .iter()
            .map(|utxo| utxo.assets.clone())
            .sum())
    }

    pub(crate) fn get_output_lovelace(&self) -> u64 {
        self.body.outputs.iter().map(|output| output.lovelace).sum()
    }

    fn get_output_assets(&self) -> Assets {
        self.body
            .outputs
            .iter()
            .flat_map(|output| output.assets.clone())
            .sum()
    }

    /// Registration certificates consume a deposit from the inputs, while deregistration
    /// certificates refund them.
    fn get_registration_deposit(&self) -> u64 {
        self.body
            .certificates
            .iter()
            .filter_map(|cert| match cert {
                Certificate::StakeRegistration { deposit, .. } => *deposit,
                Certificate::StakeRegistrationScript { deposit, .. } => *deposit,
                _ => None,
            })
            .sum()
    }

    fn get_deregistration_refund(&self) -> u64 {
        self.body
            .certificates
            .iter()
            .filter_map(|cert| match cert {
                Certificate::StakeDeregistration { deposit, .. } => *deposit,
                Certificate::StakeDeregistrationScript { deposit, .. } => *deposit,
                _ => None,
            })
            .sum()
    }

    fn get_withdrawal_lovelace(&self) -> u64 {
        self.body.withdrawals.values().copied().sum()
    }
}
