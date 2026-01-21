use serde::{Deserialize, Serialize};

use super::codec::AdaBalance;
use crate::define_ogmios_error;
use crate::ogmios::codec::{Era, ExecutionUnits};

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProtocolParams {
    /// Multiplied by the size of the transaction
    pub min_fee_coefficient: u64,
    /// Base cost for all transactions
    pub min_fee_constant: AdaBalance,

    pub plutus_cost_models: CostModels,
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

    /// Percentage of the transaction fee that must be provided as collateral
    pub collateral_percentage: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CostModels {
    #[serde(rename = "plutus:v1", skip_serializing_if = "Option::is_none")]
    pub plutus_v1: Option<CostModel>,
    #[serde(rename = "plutus:v2", skip_serializing_if = "Option::is_none")]
    pub plutus_v2: Option<CostModel>,
    #[serde(rename = "plutus:v3", skip_serializing_if = "Option::is_none")]
    pub plutus_v3: Option<CostModel>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(transparent)]
pub struct CostModel(pub Vec<i64>);

impl Into<Vec<i64>> for CostModel {
    fn into(self) -> Vec<i64> {
        self.0
    }
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

define_ogmios_error! {
    #[derive(Debug, Clone)]
    pub enum ProtocolParamsError {
        2001 => EraMismatch {
            query_era: Era,
            ledger_era: Era,
        },
        2002 => UnavailableInCurrentEra,
        2003 => StateAcquiredExpired(String)
        _ => Unknown { error: Value }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cost_model_serialization() {
        let example = r#"{"plutus:v1":[197209,0,1,1,396231,621,0,1,150000,1000,0,1,150000,32,2477736,29175,4,29773,100,29773,100,29773,100,29773,100,29773,100,29773,100,100,100,29773,100,150000,32,150000,32,150000,32,150000,1000,0,1,150000,32,150000,1000,0,8,148000,425507,118,0,1,1,150000,1000,0,8,150000,112536,247,1,150000,10000,1,136542,1326,1,1000,150000,1000,1,150000,32,150000,32,150000,32,1,1,150000,1,150000,4,103599,248,1,103599,248,1,145276,1366,1,179690,497,1,150000,32,150000,32,150000,32,150000,32,150000,32,150000,32,148000,425507,118,0,1,1,61516,11218,0,1,150000,32,148000,425507,118,0,1,1,148000,425507,118,0,1,1,2477736,29175,4,0,82363,4,150000,5000,0,1,150000,32,197209,0,1,1,150000,32,150000,32,150000,32,150000,32,150000,32,150000,32,150000,32,3345831,1,1],"plutus:v2":[205665,812,1,1,1000,571,0,1,1000,24177,4,1,1000,32,117366,10475,4,23000,100,23000,100,23000,100,23000,100,23000,100,23000,100,100,100,23000,100,19537,32,175354,32,46417,4,221973,511,0,1,89141,32,497525,14068,4,2,196500,453240,220,0,1,1,1000,28662,4,2,245000,216773,62,1,1060367,12586,1,208512,421,1,187000,1000,52998,1,80436,32,43249,32,1000,32,80556,1,57667,4,1000,10,197145,156,1,197145,156,1,204924,473,1,208896,511,1,52467,32,64832,32,65493,32,22558,32,16563,32,76511,32,196500,453240,220,0,1,1,69522,11687,0,1,60091,32,196500,453240,220,0,1,1,196500,453240,220,0,1,1,1159724,392670,0,2,806990,30482,4,1927926,82523,4,265318,0,4,0,85931,32,205665,812,1,1,41182,32,212342,32,31220,32,32696,32,43357,32,32247,32,38314,32,35892428,10,57996947,18975,10,38887044,32947,10],"plutus:v3":[100788,420,1,1,1000,173,0,1,1000,59957,4,1,11183,32,201305,8356,4,16000,100,16000,100,16000,100,16000,100,16000,100,16000,100,100,100,16000,100,94375,32,132994,32,61462,4,72010,178,0,1,22151,32,91189,769,4,2,85848,123203,7305,-900,1716,549,57,85848,0,1,1,1000,42921,4,2,24548,29498,38,1,898148,27279,1,51775,558,1,39184,1000,60594,1,141895,32,83150,32,15299,32,76049,1,13169,4,22100,10,28999,74,1,28999,74,1,43285,552,1,44749,541,1,33852,32,68246,32,72362,32,7243,32,7391,32,11546,32,85848,123203,7305,-900,1716,549,57,85848,0,1,90434,519,0,1,74433,32,85848,123203,7305,-900,1716,549,57,85848,0,1,1,85848,123203,7305,-900,1716,549,57,85848,0,1,955506,213312,0,2,270652,22588,4,1457325,64566,4,20467,1,4,0,141992,32,100788,420,1,1,81663,32,59498,32,20142,32,24588,32,20744,32,25933,32,24623,32,43053543,10,53384111,14333,10,43574283,26308,10,16000,100,16000,100,962335,18,2780678,6,442008,1,52538055,3756,18,267929,18,76433006,8868,18,52948122,18,1995836,36,3227919,12,901022,1,166917843,4307,36,284546,36,158221314,26549,36,74698472,36,333849714,1,254006273,72,2174038,72,2261318,64571,4,207616,8310,4,1293828,28716,63,0,1,1006041,43623,251,0,1]}"#;
        let parsed = serde_json::de::from_str::<CostModels>(example);
        assert!(parsed.is_ok());
        let cost_models = parsed.unwrap();
        assert!(cost_models.plutus_v1.is_some());
        assert!(cost_models.plutus_v2.is_some());
        assert!(cost_models.plutus_v3.is_some());
    }
}
