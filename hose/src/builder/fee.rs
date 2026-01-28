use std::collections::HashSet;
use std::sync::Arc;

use anyhow::Context;
use hydrant::UtxoIndexer;
use hydrant::primitives::TxOutputPointer;
use num::{BigRational, ToPrimitive as _};
use ogmios_client::OgmiosHttpClient;
use ogmios_client::method::evaluate::Evaluation;
use ogmios_client::method::pparams::ProtocolParams;
use pallas::ledger::addresses::Address;
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

        // Manual extraction of Payment Key Hash from Shelley address
        // Header (1 byte): Type (4 bits) | Network (4 bits)
        // Types 0, 2, 4, 6 have Payment Key Hash at bytes 1..29
        let bytes = address.to_vec();
        if !bytes.is_empty() {
            let header = bytes[0];
            let type_id = (header & 0xF0) >> 4;
            // Ensure it's a Shelley address (Type <= 7) and has a key hash (Even types)
            if type_id <= 7 && type_id % 2 == 0 && bytes.len() >= 29 {
                let mut hash = [0u8; 28];
                hash.copy_from_slice(&bytes[1..29]);
                signers.insert(hash);
            }
        }
    }

    if let Some(disclosed) = &tx.disclosed_signers {
        for signer in disclosed {
            signers.insert(signer.0);
        }
    }

    let witness_count = signers.len().max(1);

    let built_tx = tx
        .clone()
        .build_conway(evaluation.clone(), witness_count)
        .context("Failed to build transaction for fee calculation")?;
    let evaluation = ogmios
        .evaluate(&built_tx.bytes)
        .await
        .context("Failed to evaluate transaction")?;
    let built_tx = tx
        .clone()
        .build_conway(Some(evaluation.clone()), witness_count)
        .context("Failed to build transaction with evaluation")?;

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
