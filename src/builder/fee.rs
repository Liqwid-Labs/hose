use hydrant::UtxoIndexer;
use hydrant::primitives::TxOutputPointer;
use num::{BigRational, ToPrimitive};

use crate::builder::tx::StagingTransaction;
use crate::ogmios::OgmiosClient;
use crate::ogmios::pparams::ProtocolParams;

/// Returns the minimum lovelace for a transaction
pub async fn calculate_min_fee(
    indexer: &UtxoIndexer,
    ogmios: &OgmiosClient,
    tx: &StagingTransaction,
    pparams: &ProtocolParams,
) -> u64 {
    let built_tx = tx.clone().build_conway().unwrap();

    // Base fee + fee from size
    let mut min_fee = BigRational::from_integer(pparams.min_fee_constant.lovelace.into());
    let tx_size = built_tx.bytes.len() as u64;
    min_fee += BigRational::from_integer(tx_size.into())
        * BigRational::from_integer(pparams.min_fee_constant.lovelace.into());

    // Fee from scripts
    // TODO: don't unwrap
    let evaluation = ogmios.evaluate(&built_tx.bytes).await.unwrap();
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

        // TODO: don't unwrap
        let reference_inputs = indexer.utxos(&reference_inputs).unwrap();

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

    min_fee
        .floor()
        .to_integer()
        .to_biguint()
        .expect("failed to convert to biguint")
        .to_u64()
        .expect("failed to convert to u64")
}
