use std::collections::HashSet;
use std::sync::Arc;

use anyhow::{Context, Result};
use hydrant::UtxoIndexer;
use hydrant::primitives::TxOutputPointer;
use num::{BigRational, ToPrimitive as _};
use ogmios_client::OgmiosHttpClient;
use ogmios_client::method::evaluate::Evaluation;
use ogmios_client::method::pparams::ProtocolParams;
use pallas::ledger::addresses::{Address, ShelleyPaymentPart};
use tokio::sync::Mutex;

use super::TxBuilder;
use crate::builder::tx::StagingTransaction;
use crate::primitives::Certificate;

pub fn ref_script_fee(
    total_script_size: u64,
    range: u64,
    base: f64,
    multiplier: f64,
) -> u64 {
    if total_script_size == 0 {
        return 0;
    }

    let steps = (total_script_size / range) as i32;
    let cost_per_step = range as f64 * base;
    let mut fee = 0u64;

    // note: first step uses multiplier^0.
    for i in 0..steps {
        fee = fee + (cost_per_step * multiplier.powi(i)).floor() as u64;
    }

    // the last step uses exponent equal to number of full chunks.
    let remaining_bytes = total_script_size % range;
    if remaining_bytes > 0 {
        let base_cost = remaining_bytes as f64 * base;
        fee = fee + (base_cost * multiplier.powi(steps)).floor() as u64;
    }

    fee
}

impl TxBuilder {
    /// Returns the minimum lovelace for a transaction
    pub async fn min_fee(
        tx: &StagingTransaction,
        indexer: &Arc<Mutex<UtxoIndexer>>,
        ogmios: &OgmiosHttpClient,
        pparams: &ProtocolParams,
        evaluation: Option<Vec<Evaluation>>,
    ) -> Result<(u64, Vec<Evaluation>)> {
        // Estimate witness count
        let input_pointers = tx
            .inputs
            .iter()
            .chain(tx.collateral_inputs.iter())
            .map(|input| TxOutputPointer::new(input.hash, input.index))
            .collect::<Vec<_>>();

        let inputs = {
            let indexer = indexer.lock().await;
            indexer
                .utxos(&input_pointers)
                .context("Failed to fetch input UTXOs for witness estimation")?
        };

        let mut signers = HashSet::new();
        for input in inputs {
            let address = Address::from_bytes(&input.address).context("Invalid address")?;

            if let Address::Shelley(shelley_addr) = address
                && let ShelleyPaymentPart::Key(hash) = shelley_addr.payment()
            {
                signers.insert(*hash);
            }
        }

        for cert in &tx.certificates {
            match cert {
                Certificate::StakeRegistration { pub_key_hash, .. }
                | Certificate::StakeDeregistration { pub_key_hash, .. }
                | Certificate::StakeDelegation { pub_key_hash, .. } => {
                    signers.insert(pub_key_hash.0.into());
                }
                _ => {}
            }
        }

        for account in tx.withdrawals.keys() {
            let bytes = account.as_ref();
            if !bytes.is_empty() && (bytes[0] & 0x10) == 0 {
                // Key-based reward account
                if bytes.len() >= 29 {
                    let mut hash = [0u8; 28];
                    hash.copy_from_slice(&bytes[1..29]);
                    signers.insert(hash.into());
                }
            }
        }

        if let Some(disclosed) = &tx.disclosed_signers {
            for signer in disclosed {
                signers.insert(signer.0.into());
            }
        }

        let witness_count = signers.len().max(1);

        let mut built_tx = tx
            .clone()
            .build_conway(evaluation.clone())
            .context("Failed to build transaction for fee calculation")?;

        for i in 0..witness_count {
            let mut vkey = [0u8; 32];
            vkey[0] = (i % 256) as u8;
            vkey[1] = (i / 256) as u8;
            let signature = [0u8; 64];
            built_tx = built_tx
                .add_signature(vkey.into(), signature)
                .context("Failed to add dummy witness")?;
        }

        let evaluation = ogmios
            .evaluate(&built_tx.bytes)
            .await
            .context("Failed to evaluate transaction")?;
        let mut built_tx = tx
            .clone()
            .build_conway(Some(evaluation.clone()))
            .context("Failed to build transaction with evaluation")?;

        for i in 0..witness_count {
            let mut vkey = [0u8; 32];
            vkey[0] = (i % 256) as u8;
            vkey[1] = (i / 256) as u8;
            let signature = [0u8; 64];
            built_tx = built_tx
                .add_signature(vkey.into(), signature)
                .context("Failed to add dummy witness")?;
        }

        // Base fee + fee from size
        let mut min_fee = BigRational::from_integer(pparams.min_fee_constant.lovelace.into());
        let tx_size = built_tx.bytes.len() as u64;
        // TODO: for some reason this is off by 1 byte, not that it matters since it's a difference
        // of 0.000044 ADA...
        min_fee += BigRational::from_integer(tx_size.into())
            * BigRational::from_integer(pparams.min_fee_coefficient.into());
        // Fee from scripts
        let total_cpu = evaluation
            .iter()
            .map(|e| e.budget.cpu.0.clone())
            .sum::<BigRational>();
        let total_mem = evaluation
            .iter()
            .map(|e| e.budget.memory.0.clone())
            .sum::<BigRational>();
        min_fee += total_cpu * pparams.script_execution_prices.cpu.0.clone();
        min_fee += total_mem * pparams.script_execution_prices.memory.0.clone();

        // Fee from reference script sizes (in tx outputs/collateral return and reference inputs)
        // https://github.com/IntersectMBO/cardano-ledger/blob/master/docs/adr/2024-08-14_009-refscripts-fee-change.md
        let mut total_script_size: u64 = tx
            .outputs
            .iter()
            .flat_map(|output| output.script.as_ref())
            .map(|script| script.bytes.len() as u64)
            .sum();
        total_script_size += tx.collateral_output
            .iter()
            .flat_map(|output| output.script.as_ref())
            .map(|script| script.bytes.len() as u64)
            .sum::<u64>();

        if !tx.reference_inputs.is_empty() {
            let reference_inputs = tx
                .reference_inputs
                .iter()
                .map(|input| TxOutputPointer::new(input.hash, input.index))
                .collect::<Vec<_>>();

            let reference_inputs = {
                let indexer = indexer.lock().await;
                indexer
                    .utxos(&reference_inputs)
                    .context("Failed to fetch reference input UTXOs")?
            };

            total_script_size += reference_inputs
                .iter()
                .flat_map(|utxo| utxo.script.as_ref())
                .map(|script| script.bytes.len() as u64)
                .sum::<u64>();
        }

        if total_script_size > 0 {
            let range = pparams.min_fee_reference_scripts.range as u64;
            let base = pparams.min_fee_reference_scripts.base;
            let multiplier = pparams.min_fee_reference_scripts.multiplier;
            let ref_script_fee =
                ref_script_fee(total_script_size, range, base, multiplier);
            min_fee += BigRational::from_integer(ref_script_fee.into());
        }

        let fee = min_fee
            .ceil()
            .to_integer()
            .to_biguint()
            .context("Failed to convert fee to BigUint")?
            .to_u64()
            .context("Failed to convert fee to u64")?;
        Ok((fee, evaluation))
    }
}

#[cfg(test)]
mod tests {
    use super::ref_script_fee;

    #[test]
    fn no_fee_if_no_ref_scripts() {
        assert_eq!(ref_script_fee(0, 25_600, 15.0, 1.2), 0);
    }

    #[test]
    fn ref_script_fee_full_plus_partial_steps() {
        // complete steps: 25600 * 15 * 1.0 = 384_000
        // last, partial step: 4400 * 15 * 1.2 = 79_200
        // total: 463_200
        assert_eq!(ref_script_fee(30_000, 25_600, 15.0, 1.2), 463_200);
    }
}
