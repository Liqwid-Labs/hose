use serde::{Deserialize, Serialize};

use super::codec::AdaBalance;
use crate::ogmios::codec::ExecutionUnits;

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProtocolParams {
    /// Multiplied by the size of the transaction
    pub min_fee_coefficient: u64,
    /// Base cost for all transactions
    pub min_fee_constant: AdaBalance,
    /// Multiplied by the size of the reference script
    /// This number gets multiplied every `range` bytes by the `multiplier`
    /// such that the cost scales exponentially, via a recursive function
    ///
    /// range: 1024 (1KiB)
    /// base: 10
    /// multiplier: 1.2
    ///
    /// 1KB: 10 * 1024 = 10240
    /// 2KB: 10 * 1024 + (10 * 1.2) * 1024 = 22528
    /// 2.5KB: 10 * 1024 + (10 * 1.2) * 1024 + (10 * 1.2^2) * 512 = 29900.8
    /// ...
    pub min_fee_reference_scripts: MinFeeReferenceScripts,
    /// Multiplied by the size of the UTxO (not the whole transaction) to get the minimum UTxO
    /// deposit
    pub min_utxo_deposit_coefficient: u64,
    /// Price per unit of CPU and memory
    pub script_execution_prices: ExecutionUnits,
}

/// Multiplied by the size of the reference script
/// This number gets multiplied every `range` bytes by the `multiplier`
/// such that the cost scales exponentially, via a recursive function
///
/// range: 1024 (1KiB)
/// base: 10
/// multiplier: 1.2
///
/// 1KB: 10 * 1024 = 10240
/// 2KB: 10 * 1024 + (10 * 1.2) * 1024 = 22528
/// 2.5KB: 10 * 1024 + (10 * 1.2) * 1024 + (10 * 1.2^2) * 512 = 29900.8
/// ...
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MinFeeReferenceScripts {
    /// Range (in bytes) at which the cost scales by the multiplier
    pub range: u32,
    /// Cost per byte, multiplied by `multiplier ^ range_index`
    pub base: f64,
    pub multiplier: f64,
}
