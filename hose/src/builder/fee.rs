use std::collections::HashSet;
use std::sync::Arc;

use anyhow::Context;
use hydrant::UtxoIndexer;
use hydrant::primitives::TxOutputPointer;
use num::{BigRational, ToPrimitive as _};
use ogmios_client::OgmiosHttpClient;
use ogmios_client::method::evaluate::Evaluation;
use ogmios_client::method::pparams::ProtocolParams;
use pallas::ledger::addresses::{Address, ShelleyPaymentPart};
use tokio::sync::Mutex;

use crate::builder::tx::StagingTransaction;

/// Returns the minimum lovelace for a transaction
pub async fn calculate_min_fee(
    indexer: Arc<Mutex<UtxoIndexer>>,
    ogmios: &OgmiosHttpClient,
    tx: &StagingTransaction,
    pparams: &ProtocolParams,
    evaluation: Option<Vec<Evaluation>>,
) -> anyhow::Result<(u64, Vec<Evaluation>)> {
    // Estimate witness count
    let input_pointers = tx
        .inputs
        .iter()
        .chain(tx.collateral_inputs.iter())
        .map(|input| TxOutputPointer::new(input.hash.into(), input.index))
        .collect::<Vec<_>>();

    let inputs = {
        let indexer = indexer.lock().await;
        indexer
            .utxos(&input_pointers)
            .context("Failed to fetch input UTXOs for witness estimation")?
    };

    let mut signers = HashSet::new();
    for input in inputs {
        let address = Address::from_bytes(&input.address)
            .map_err(|e| anyhow::anyhow!("Invalid address: {:?}", e))?;

        if let Address::Shelley(shelley_addr) = address {
            if let ShelleyPaymentPart::Key(hash) = shelley_addr.payment() {
                signers.insert(*hash);
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
    if !tx.reference_inputs.is_empty() {
        let reference_inputs = tx
            .reference_inputs
            .iter()
            .map(|input| TxOutputPointer::new(input.hash.into(), input.index))
            .collect::<Vec<_>>();

        let reference_inputs = {
            let indexer = indexer.lock().await;
            indexer
                .utxos(&reference_inputs)
                .context("Failed to fetch reference input UTXOs")?
        };

        let total_script_size = reference_inputs
            .iter()
            .flat_map(|utxo| utxo.script.as_ref())
            .map(|script| script.bytes.len() as u64)
            .sum::<u64>();

        // Full chunks
        let range = pparams.min_fee_reference_scripts.range as u64;
        let base = pparams.min_fee_reference_scripts.base;
        let multiplier = pparams.min_fee_reference_scripts.multiplier;
        let steps = (total_script_size / range) as i32;
        let cost_per_step = range as f64 * base;
        for i in 0..steps {
            min_fee += BigRational::from_integer(
                ((cost_per_step * multiplier.powi(i + 1)).floor() as u64).into(),
            );
        }

        // Partial chunk
        let partial_chunk_bytes = total_script_size % range;
        if partial_chunk_bytes > 0 {
            let base_cost = partial_chunk_bytes as f64 * base;
            min_fee += BigRational::from_integer(
                ((base_cost * multiplier.powi(steps + 1)).floor() as u64).into(),
            );
        }
    }

    let fee = min_fee
        .floor()
        .to_integer()
        .to_biguint()
        .context("Failed to convert fee to BigUint")?
        .to_u64()
        .context("Failed to convert fee to u64")?;
    Ok((fee, evaluation))
}
