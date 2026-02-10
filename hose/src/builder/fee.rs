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

        // Fee from reference input script sizes
        // https://github.com/IntersectMBO/cardano-ledger/blob/master/docs/adr/2024-08-14_009-refscripts-fee-change.md
        let inputs_and_ref_input_pointers = tx
            .inputs
            .iter()
            .chain(tx.reference_inputs.iter())
            .map(|input| TxOutputPointer::new(input.hash, input.index))
            .collect::<Vec<_>>();

        let resolved_inputs_and_ref_inputs = {
            let indexer = indexer.lock().await;
            indexer.utxos(&inputs_and_ref_input_pointers).context(
                "Failed to fetch inputs or reference inputs for reference script fee calculation",
            )?
        };

        let total_ref_script_size = resolved_inputs_and_ref_inputs
            .iter()
            .flat_map(|utxo| utxo.script.as_ref())
            .map(|script| script.bytes.len() as u64)
            .sum::<u64>();

        if total_ref_script_size > 0 {
            // Full chunks
            let range = pparams.min_fee_reference_scripts.range as u64;
            let base = pparams.min_fee_reference_scripts.base;
            let multiplier = pparams.min_fee_reference_scripts.multiplier;

            // to match the ledger's behavior, all tier contributions need to be summed first,
            // then floored only at the very end. See `tierRefScriptFee`:
            // https://github.com/IntersectMBO/cardano-ledger/blob/6ef1bf9fa1ca589e706e781fa8c9b4ad8df1e919/eras/conway/impl/src/Cardano/Ledger/Conway/Tx.hs#L122-L130
            let steps = (total_ref_script_size / range) as i32;
            let cost_per_step = range as f64 * base;
            let mut ref_script_fee = 0.0;

            for i in 0..steps {
                ref_script_fee += cost_per_step * multiplier.powi(i);
            }

            // Partial chunk
            let partial_chunk_bytes = total_ref_script_size % range;
            if partial_chunk_bytes > 0 {
                let base_cost = partial_chunk_bytes as f64 * base;
                ref_script_fee += base_cost * multiplier.powi(steps);
            }

            min_fee += BigRational::from_integer((ref_script_fee.floor() as u64).into());
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
