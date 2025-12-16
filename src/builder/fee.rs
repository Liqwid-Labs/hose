use hydrant::primitives::TxOutputPointer;
use pallas::txbuilder::{BuildConway, StagingTransaction};

use crate::ogmios::OgmiosClient;
use crate::ogmios::pparams::ProtocolParams;

/// Returns the minimum lovelace for a transaction
pub async fn calculate_min_fee(
    ogmios: &OgmiosClient,
    tx: &StagingTransaction,
    pparams: &ProtocolParams,
) -> u64 {
    let built_tx = tx.clone().build_conway_raw().unwrap();

    // Base fee + fee from size
    let mut min_fee = pparams.min_fee_constant.lovelace;
    let tx_size = built_tx.tx_bytes.0.len() as u64;
    min_fee += tx_size * pparams.min_fee_constant.lovelace;

    // Fee from scripts
    let evaluation = ogmios.evaluate(&built_tx.tx_bytes.0, vec![]).await.unwrap();
    let total_cpu = evaluation.iter().map(|e| e.budget.cpu).sum::<u64>();
    let total_mem = evaluation.iter().map(|e| e.budget.memory).sum::<u64>();
    min_fee += total_cpu * pparams.script_execution_prices.cpu;
    min_fee += total_mem * pparams.script_execution_prices.memory;

    // Fee from reference input script sizes
    // https://github.com/IntersectMBO/cardano-ledger/blob/master/docs/adr/2024-08-14_009-refscripts-fee-change.md
    if let Some(reference_inputs) = tx.reference_inputs.as_ref() {
        let reference_inputs = reference_inputs
            .iter()
            .map(|input| TxOutputPointer::new(input.tx_hash.0.into(), input.txo_index as usize))
            .map(Into::into)
            .collect::<Vec<_>>();

        // TODO: don't unwrap
        let reference_inputs = ogmios.utxos_by_output(&reference_inputs).await.unwrap();

        let total_script_size = reference_inputs
            .iter()
            .flat_map(|utxo| utxo.script.as_ref())
            .flat_map(|script| script.cbor())
            .map(|cbor| cbor.len() as u64)
            .sum::<u64>();

        // Full chunks
        let range = pparams.min_fee_reference_scripts.range as u64;
        let base = pparams.min_fee_reference_scripts.base;
        let multiplier = pparams.min_fee_reference_scripts.multiplier;
        let steps = (total_script_size / range) as i32;
        let cost_per_step = (range * base) as f64;
        for i in 0..steps {
            min_fee += (cost_per_step * multiplier.powi(i + 1)).floor() as u64;
        }

        // Partial chunk
        let partial_chunk_bytes = total_script_size % range;
        if partial_chunk_bytes > 0 {
            let base_cost = (partial_chunk_bytes * base) as f64;
            min_fee += (base_cost * multiplier.powi(steps + 1)).floor() as u64;
        }
    }

    min_fee
}
