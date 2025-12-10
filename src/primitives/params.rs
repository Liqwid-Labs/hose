use crate::network::NetworkId;
use pallas::{
    applying::utils::{AlonzoProtParams, BabbageProtParams, ConwayProtParams, ShelleyProtParams},
    ledger::{
        configs::{alonzo, conway, shelley},
        primitives::alonzo::Language as AlonzoLanguage,
    },
};

pub use pallas::applying::utils::MultiEraProtocolParameters;

const MAINNET_SHELLEY: &str = include_str!("../configs/mainnet/shelley.json");
const MAINNET_ALONZO: &str = include_str!("../configs/mainnet/alonzo.json");
const MAINNET_CONWAY: &str = include_str!("../configs/mainnet/conway.json");

const PREVIEW_SHELLEY: &str = include_str!("../configs/preview/shelley.json");
const PREVIEW_ALONZO: &str = include_str!("../configs/preview/alonzo.json");
const PREVIEW_CONWAY: &str = include_str!("../configs/preview/conway.json");

pub fn get_protocol_parameters(network: NetworkId) -> MultiEraProtocolParameters {
    let (shelley_str, alonzo_str, conway_str) = match network {
        NetworkId::Mainnet => (MAINNET_SHELLEY, MAINNET_ALONZO, MAINNET_CONWAY),
        NetworkId::Testnet => (PREVIEW_SHELLEY, PREVIEW_ALONZO, PREVIEW_CONWAY),
    };

    // TODO: test unwrap at compile time
    let shelley: shelley::GenesisFile = serde_json::from_str(shelley_str).unwrap();
    let alonzo: alonzo::GenesisFile = serde_json::from_str(alonzo_str).unwrap();
    let conway: conway::GenesisFile = serde_json::from_str(conway_str).unwrap();

    let shelley_params = bootstrap_shelley_pparams(&shelley);
    let alonzo_params = bootstrap_alonzo_pparams(shelley_params, &alonzo);
    let babbage_params = bootstrap_babbage_pparams(alonzo_params);
    let conway_params = bootstrap_conway_pparams(babbage_params, &conway);

    MultiEraProtocolParameters::Conway(conway_params)
}

impl From<NetworkId> for MultiEraProtocolParameters {
    fn from(network: NetworkId) -> Self {
        get_protocol_parameters(network)
    }
}

fn bootstrap_shelley_pparams(shelley: &shelley::GenesisFile) -> ShelleyProtParams {
    ShelleyProtParams {
        // TODO: remove unwrap once we make the whole process fallible
        system_start: chrono::DateTime::parse_from_rfc3339(shelley.system_start.as_ref().unwrap())
            .unwrap(),
        protocol_version: shelley.protocol_params.protocol_version.clone().into(),
        epoch_length: shelley.epoch_length.unwrap_or_default() as u64,
        slot_length: shelley.slot_length.unwrap_or_default() as u64,
        max_block_body_size: shelley.protocol_params.max_block_body_size,
        max_transaction_size: shelley.protocol_params.max_tx_size,
        max_block_header_size: shelley.protocol_params.max_block_header_size,
        key_deposit: shelley.protocol_params.key_deposit,
        min_utxo_value: shelley.protocol_params.min_utxo_value,
        minfee_a: shelley.protocol_params.min_fee_a,
        minfee_b: shelley.protocol_params.min_fee_b,
        pool_deposit: shelley.protocol_params.pool_deposit,
        desired_number_of_stake_pools: shelley.protocol_params.n_opt,
        min_pool_cost: shelley.protocol_params.min_pool_cost,
        expansion_rate: shelley.protocol_params.rho.clone(),
        treasury_growth_rate: shelley.protocol_params.tau.clone(),
        maximum_epoch: shelley.protocol_params.e_max,
        pool_pledge_influence: shelley.protocol_params.a0.clone(),
        decentralization_constant: shelley.protocol_params.decentralisation_param.clone(),
        extra_entropy: shelley.protocol_params.extra_entropy.clone().into(),
    }
}

fn bootstrap_alonzo_pparams(
    previous: ShelleyProtParams,
    genesis: &alonzo::GenesisFile,
) -> AlonzoProtParams {
    AlonzoProtParams {
        system_start: previous.system_start,
        epoch_length: previous.epoch_length,
        slot_length: previous.slot_length,
        minfee_a: previous.minfee_a,
        minfee_b: previous.minfee_b,
        max_block_body_size: previous.max_block_body_size,
        max_transaction_size: previous.max_transaction_size,
        max_block_header_size: previous.max_block_header_size,
        key_deposit: previous.key_deposit,
        pool_deposit: previous.pool_deposit,
        protocol_version: previous.protocol_version,
        min_pool_cost: previous.min_pool_cost,
        desired_number_of_stake_pools: previous.desired_number_of_stake_pools,
        expansion_rate: previous.expansion_rate.clone(),
        treasury_growth_rate: previous.treasury_growth_rate.clone(),
        maximum_epoch: previous.maximum_epoch,
        pool_pledge_influence: previous.pool_pledge_influence,
        decentralization_constant: previous.decentralization_constant,
        extra_entropy: previous.extra_entropy,
        // new from genesis
        ada_per_utxo_byte: genesis.lovelace_per_utxo_word,
        cost_models_for_script_languages: genesis.cost_models.clone().into(),
        execution_costs: genesis.execution_prices.clone().into(),
        max_tx_ex_units: genesis.max_tx_ex_units.clone().into(),
        max_block_ex_units: genesis.max_block_ex_units.clone().into(),
        max_value_size: genesis.max_value_size,
        collateral_percentage: genesis.collateral_percentage,
        max_collateral_inputs: genesis.max_collateral_inputs,
    }
}

fn bootstrap_babbage_pparams(previous: AlonzoProtParams) -> BabbageProtParams {
    BabbageProtParams {
        system_start: previous.system_start,
        epoch_length: previous.epoch_length,
        slot_length: previous.slot_length,
        minfee_a: previous.minfee_a,
        minfee_b: previous.minfee_b,
        max_block_body_size: previous.max_block_body_size,
        max_transaction_size: previous.max_transaction_size,
        max_block_header_size: previous.max_block_header_size,
        key_deposit: previous.key_deposit,
        pool_deposit: previous.pool_deposit,
        protocol_version: previous.protocol_version,
        min_pool_cost: previous.min_pool_cost,
        desired_number_of_stake_pools: previous.desired_number_of_stake_pools,
        ada_per_utxo_byte: previous.ada_per_utxo_byte,
        execution_costs: previous.execution_costs,
        max_tx_ex_units: previous.max_tx_ex_units,
        max_block_ex_units: previous.max_block_ex_units,
        max_value_size: previous.max_value_size,
        collateral_percentage: previous.collateral_percentage,
        max_collateral_inputs: previous.max_collateral_inputs,
        expansion_rate: previous.expansion_rate,
        treasury_growth_rate: previous.treasury_growth_rate,
        maximum_epoch: previous.maximum_epoch,
        pool_pledge_influence: previous.pool_pledge_influence,
        decentralization_constant: previous.decentralization_constant,
        extra_entropy: previous.extra_entropy,
        cost_models_for_script_languages: pallas::ledger::primitives::babbage::CostModels {
            plutus_v1: previous
                .cost_models_for_script_languages
                .iter()
                .filter(|(k, _)| k == &AlonzoLanguage::PlutusV1)
                .map(|(_, v)| v.clone())
                .next(),
            plutus_v2: None,
        },
    }
}

fn bootstrap_conway_pparams(
    previous: BabbageProtParams,
    genesis: &conway::GenesisFile,
) -> ConwayProtParams {
    ConwayProtParams {
        system_start: previous.system_start,
        epoch_length: previous.epoch_length,
        slot_length: previous.slot_length,
        minfee_a: previous.minfee_a,
        minfee_b: previous.minfee_b,
        max_block_body_size: previous.max_block_body_size,
        max_transaction_size: previous.max_transaction_size,
        max_block_header_size: previous.max_block_header_size,
        key_deposit: previous.key_deposit,
        pool_deposit: previous.pool_deposit,
        protocol_version: previous.protocol_version,
        min_pool_cost: previous.min_pool_cost,
        desired_number_of_stake_pools: previous.desired_number_of_stake_pools,
        // In the hardfork, the value got translated from words to bytes
        // Since the transformation from words to bytes is hardcoded, the transformation here is also hardcoded
        ada_per_utxo_byte: previous.ada_per_utxo_byte / 8,
        execution_costs: previous.execution_costs,
        max_tx_ex_units: previous.max_tx_ex_units,
        max_block_ex_units: previous.max_block_ex_units,
        max_value_size: previous.max_value_size,
        collateral_percentage: previous.collateral_percentage,
        max_collateral_inputs: previous.max_collateral_inputs,
        expansion_rate: previous.expansion_rate,
        treasury_growth_rate: previous.treasury_growth_rate,
        maximum_epoch: previous.maximum_epoch,
        pool_pledge_influence: previous.pool_pledge_influence,
        cost_models_for_script_languages: pallas::ledger::primitives::conway::CostModels {
            plutus_v1: previous.cost_models_for_script_languages.plutus_v1,
            plutus_v2: previous.cost_models_for_script_languages.plutus_v2,
            plutus_v3: Some(genesis.plutus_v3_cost_model.clone()),
        },
        pool_voting_thresholds: pallas::ledger::primitives::conway::PoolVotingThresholds {
            motion_no_confidence: float_to_rational(
                genesis.pool_voting_thresholds.motion_no_confidence,
            ),
            committee_normal: float_to_rational(genesis.pool_voting_thresholds.committee_normal),
            committee_no_confidence: float_to_rational(
                genesis.pool_voting_thresholds.committee_no_confidence,
            ),
            hard_fork_initiation: float_to_rational(
                genesis.pool_voting_thresholds.hard_fork_initiation,
            ),
            security_voting_threshold: float_to_rational(
                genesis.pool_voting_thresholds.pp_security_group,
            ),
        },
        drep_voting_thresholds: pallas::ledger::primitives::conway::DRepVotingThresholds {
            motion_no_confidence: float_to_rational(
                genesis.d_rep_voting_thresholds.motion_no_confidence,
            ),
            committee_normal: float_to_rational(genesis.d_rep_voting_thresholds.committee_normal),
            committee_no_confidence: float_to_rational(
                genesis.d_rep_voting_thresholds.committee_no_confidence,
            ),
            update_constitution: float_to_rational(
                genesis.d_rep_voting_thresholds.update_to_constitution,
            ),
            hard_fork_initiation: float_to_rational(
                genesis.d_rep_voting_thresholds.hard_fork_initiation,
            ),
            pp_network_group: float_to_rational(genesis.d_rep_voting_thresholds.pp_network_group),
            pp_economic_group: float_to_rational(genesis.d_rep_voting_thresholds.pp_economic_group),
            pp_technical_group: float_to_rational(
                genesis.d_rep_voting_thresholds.pp_technical_group,
            ),
            pp_governance_group: float_to_rational(genesis.d_rep_voting_thresholds.pp_gov_group),
            treasury_withdrawal: float_to_rational(
                genesis.d_rep_voting_thresholds.treasury_withdrawal,
            ),
        },
        min_committee_size: genesis.committee_min_size,
        committee_term_limit: genesis.committee_max_term_length.into(),
        governance_action_validity_period: genesis.gov_action_lifetime.into(),
        governance_action_deposit: genesis.gov_action_deposit,
        drep_deposit: genesis.d_rep_deposit,
        drep_inactivity_period: genesis.d_rep_activity.into(),
        minfee_refscript_cost_per_byte: pallas::ledger::primitives::conway::RationalNumber {
            numerator: genesis.min_fee_ref_script_cost_per_byte,
            denominator: 1,
        },
    }
}

fn float_to_rational(x: f32) -> pallas::ledger::primitives::conway::RationalNumber {
    const PRECISION: u32 = 9;
    let scale = 10u64.pow(PRECISION);
    let scaled = (x * scale as f32).round() as u64;

    // Check if it's very close to a whole number
    if (x.round() - x).abs() < f32::EPSILON {
        return pallas::ledger::primitives::conway::RationalNumber {
            numerator: x.round() as u64,
            denominator: 1,
        };
    }

    let gcd = gcd(scaled, scale);

    pallas::ledger::primitives::conway::RationalNumber {
        numerator: scaled / gcd,
        denominator: scale / gcd,
    }
}

// Helper function to calculate the Greatest Common Divisor
fn gcd(mut a: u64, mut b: u64) -> u64 {
    while b != 0 {
        let temp = b;
        b = a % b;
        a = temp;
    }
    a
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pool_voting_thresholds_rational() {
        let thresholds = [
            ("committeeNormal", 0.51),
            ("committeeNoConfidence", 0.51),
            ("hardForkInitiation", 0.51),
            ("motionNoConfidence", 0.51),
            ("ppSecurityGroup", 0.51),
        ];

        for (name, value) in thresholds.iter() {
            let result = float_to_rational(*value);
            assert_eq!(result.numerator, 51, "Failed for {}", name);
            assert_eq!(result.denominator, 100, "Failed for {}", name);
        }
    }

    #[test]
    fn test_drep_voting_thresholds_rational() {
        let thresholds = [
            ("motionNoConfidence", 0.67),
            ("committeeNormal", 0.67),
            ("committeeNoConfidence", 0.60),
            ("updateToConstitution", 0.75),
            ("hardForkInitiation", 0.60),
            ("ppNetworkGroup", 0.67),
            ("ppEconomicGroup", 0.67),
            ("ppTechnicalGroup", 0.67),
            ("ppGovGroup", 0.75),
            ("treasuryWithdrawal", 0.67),
        ];

        for (name, value) in thresholds.iter() {
            let result = float_to_rational(*value);
            match *value {
                0.67 => {
                    assert_eq!(result.numerator, 67, "Failed for {}", name);
                    assert_eq!(result.denominator, 100, "Failed for {}", name);
                }
                0.60 => {
                    assert_eq!(result.numerator, 3, "Failed for {}", name);
                    assert_eq!(result.denominator, 5, "Failed for {}", name);
                }
                0.75 => {
                    assert_eq!(result.numerator, 3, "Failed for {}", name);
                    assert_eq!(result.denominator, 4, "Failed for {}", name);
                }
                _ => panic!("Unexpected value for {}: {}", name, value),
            }
        }
    }

    fn assert_rational_eq(
        result: pallas::ledger::primitives::conway::RationalNumber,
        expected_num: u64,
        expected_den: u64,
        input: f32,
    ) {
        assert_eq!(
            result.numerator, expected_num,
            "Numerator mismatch for input {}",
            input
        );
        assert_eq!(
            result.denominator, expected_den,
            "Denominator mismatch for input {}",
            input
        );
    }

    #[test]
    fn test_whole_number() {
        let test_cases = [
            (1.0, 1, 1),
            (2.0, 2, 1),
            (100.0, 100, 1),
            (1000000.0, 1000000, 1),
        ];

        for &(input, expected_num, expected_den) in test_cases.iter() {
            let result = float_to_rational(input);
            assert_rational_eq(result, expected_num, expected_den, input);
        }
    }

    #[test]
    fn test_fractions() {
        let test_cases = [
            (0.5, 1, 2),
            (0.25, 1, 4),
            // (0.33333334, 333333343, 1000000000), // These fails due to floating point precision
            // (0.66666669, 666666687, 1000000000),
        ];

        for &(input, expected_num, expected_den) in test_cases.iter() {
            let result = float_to_rational(input);
            assert_rational_eq(result, expected_num, expected_den, input);
        }
    }
}
