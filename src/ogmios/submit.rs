use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::codec::{
    AdaBalance, AdaBalanceDelta, Balance, CredentialOrigin, ExecutionUnits, Language,
    RedeemerPointer, StakePoolId, TxCbor, TxId, TxOutput, TxOutputPointer, ValidityInterval,
};
use super::script::ScriptPurpose;
use super::utxo::Utxo;
use crate::define_ogmios_error;
use crate::ogmios::codec::{Era, InputSource, NumberOfBytes, ProtocolVersion};

// -----------
// Request
// -----------

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SubmitRequestParams {
    pub transaction: TxCbor,
}

// -----------
// Response
// -----------

#[derive(Debug, Clone, Deserialize)]
#[doc = "hi"]
struct MetadataHash {
    /// Hex-encoded 32-byte blake2b hash digest
    hash: String,
}

define_ogmios_error! {
    #[derive(Debug, Clone)]
    pub enum SubmitError {
        3005 => EraMismatch {
            query_era: Era,
            ledger_era: Era,
        },
        3100 => InvalidSignatories {
            #[doc="Hex-encoded 32-byte verification key hashes"]
            invalid_signatories: Vec<String>,
        },
        3101 => MissingSignatories {
            /// Hex-encoded 28-byte blake2b hashes
            missing_signatories: Vec<String>,
        },
        3102 => FailingNativeScripts {
            /// Hex-encoded 28-byte blake2b hashes
            failing_native_scripts: Vec<String>,
        },
        3104 => ExtraneousScripts {
            /// Hex-encoded 28-byte blake2b hashes
            extraneous_scripts: Vec<String>,
        },
        3105 => MissingMetadataHash {
            metadata: MetadataHash,
        },
        3106 => MissingMetadata {
            metadata: MetadataHash,
        },
        3107 => MetadataHashMismatch {
            provided: MetadataHash,
            computed: MetadataHash,
        },
        3108 => InvalidMetadata,
        3109 => MissingRedeemers {
            missing_redeemers: Vec<ScriptPurpose>,
        },
        3110 => ExtraneousRedeemers {
            extraneous_redeemers: Vec<RedeemerPointer>,
        },
        3111 => MissingDatums {
            /// Hex-encoded 32-byte blake2b hashes
            missing_datums: Vec<String>,
        },
        3112 => ExtraneousDatums {
            /// Hex-encoded 32-byte blake2b hashes
            extraneous_datums: Vec<String>,
        },
        3113 => ScriptIntegrityHashMismatch {
            /// Hex-encoded 32-byte blake2b hash
            provided_script_integrity: Option<String>,
            /// Hex-encoded 32-byte blake2b hash
            computed_script_integrity: Option<String>,
        },
        3114 => OrphanScriptInputs {
            orphan_script_inputs: Vec<TxOutputPointer>,
        },
        3115 => MissingCostModels {
            missing_cost_models: Vec<Language>,
        },
        3116 => MalformedScripts {
            /// Hex-encoded 28-byte blake2b hashes
            malformed_scripts: Vec<String>,
        },
        3117 => UnknownOutputReferences {
            unknown_output_references: Vec<TxOutputPointer>,
        },
        3118 => OutsideOfValidityInterval {
            validity_interval: ValidityInterval,
            current_slot: u32,
        },
        3119 => TransactionTooLarge {
            measured_transaction_size: u64,
            maximum_transaction_size: u64,
        },
        3120 => ValueToolarge {
            excessively_large_outputs: Vec<Utxo>
        },
        3121 => EmptyInputSet,
        3122 => TransactionFeeTooSmall {
            minimum_required_fee: AdaBalance,
            provided_fee: AdaBalance,
        },
        3123 => ValueNotConserved {
            value_consumed: Balance,
            value_produced: Balance,
        },
        3124 => NetworkMismatch {
            expected_network: Network,
            discriminated_type: NetworkMismatchDiscriminatedType,
            /// Array of addresses (addr1), reward accounts (stake1) or stake pools (pool1)
            invalid_entities: Option<Vec<String>>,
        },
        3125 => InsufficientlyFundedOutputs {
            insufficiently_funded_outputs: Vec<InsufficientlyFundedOutput>
        },
        3126 => BootstrapAttributesTooLarge {
            bootstrap_outputs: Vec<TxOutput>
        },
        3127 => MintingOrBurningAda,
        3128 => InsufficientCollateral {
            provided_collateral: AdaBalanceDelta,
            minimum_required_collateral: AdaBalance,
        },
        3129 => CollateralLockedByScript {
            unsuitable_collateral_inputs: Vec<TxOutputPointer>,
        },
        3130 => UnforeseeableSlot {
            unforeseeable_slot: u32,
        },
        3131 => TooManyCollateralInputs {
            maximum_collateral_inputs: u32,
            counted_collateral_inputs: u32,
        },
        3132 => MissingCollateralInputs,
        3133 => NonAdaCollateral {
            unsuitable_collateral_inputs: Balance
        },
        3134 => ExecutionUnitsTooLarge {
            provided_execution_units: ExecutionUnits,
            maximum_execution_units: ExecutionUnits,
        },
        3135 => TotalCollateralMismatch {
            declared_total_collateral: AdaBalance,
            computed_total_collateral: AdaBalanceDelta,
        },
        3136 => SpendsMismatch {
            declared_spending: InputSource,
            mismatch_reason: String
        },
        3137 => UnauthorizedVotes {
            unauthorized_votes: Vec<Value>, // TODO:
        },
        3138 => UnknownGovernanceProposals {
            unknown_proposals: Vec<Value>, // TODO:
        },
        3139 => InvalidProtocolParametersUpdate,
        3140 => UnknownStakePool {
            /// Hex-encoded 28-byte blake2b hash digest (pool1...)
            unknown_stake_pool: String,
        },
        3141 => IncompleteWithdrawals {
            incomplete_withdrawals: HashMap<String, AdaBalance>
        },
        3142 => RetirementTooLate {
            current_epoch: u64,
            declared_epoch: u64,
            first_invalid_epoch: u64,
        },
        3143 => StakePoolCostTooLow {
            minimum_stake_pool_cost: AdaBalance,
            declared_stake_pool_cost: AdaBalance,
        },
        3144 => MetadataHashTooLarge {
            infringing_stake_pool: StakePoolId,
            computed_metadata_hash_size: NumberOfBytes
        },
        3145 => CredentialAlreadyRegistered {
            /// Hex-encoded 28-byte blake2b hash digest
            known_credential: String,
            from: CredentialOrigin,
        },
        3146 => UnknownCredential {
            /// Hex-encoded 28-byte blake2b hash digest
            unknown_credential: String,
            from: CredentialOrigin,
        },
        3147 => NonEmptyRewardAccount {
            non_empty_reward_account_balance: AdaBalance,
        },
        3148 => InvalidGenesisDelegation,
        3149 => InvalidMIRTransfer,
        3150 => ForbiddenWithdrawal {
            /// Hex-encoded 28-byte blake2b hash digest
            marginalized_credentials: Vec<String>,
        },
        3151 => CredentialDepositMismatch {
            provided_deposit: AdaBalance,
            expected_deposit: AdaBalance,
        },
        3152 => DRepAlreadyRegistered {
            known_delegate_representative: Value // TODO:
        },
        3153 => DRepNotRegistered {
            unknown_delegate_representative: Value // TODO:
        },
        3154 => UnknownConsitutionalCommitteeMember {
            unknown_consistency_committee_member: CommitteeMember
        },
        3155 => GovernanceProposalDepositMismatch {
            provided_deposit: AdaBalance,
            expected_deposit: AdaBalance,
        },
        3156 => ConflictingCommitteeUpdate {
            conflicting_members: Vec<CommitteeMember>
        },
        3157 => InvalidCommitteeUpdate {
            already_retired_members: Vec<CommitteeMember>
        },
        3158 => TreasureWithdrawalMismatch {
            provided_withdrawal: AdaBalance,
            computed_withdrawal: AdaBalance,
        },
        3159 => InvalidOrMissingPreviousProposals {
            invalid_or_missing_previous_proposals: Vec<Value>, // TODO:
        },
        3160 => VotingOnExpiredActions {
            inivalid_votes: Vec<Value>, // TODO:
        },
        3161 => ExecutionBudgetOutOfBounds {
            budget_used: ExecutionUnits,
        },
        3162 => InvalidHardForkVersionBump {
            proposed_version: ProtocolVersion,
            current_version: ProtocolVersion,
        },
        3163 => ConstitutionGuardrailsHashMismatch {
            provided_hash: Option<String>,
            expected_hash: Option<String>,
        },
        3164 => ConflictingInputsAndReferences {
            conflicting_references: Vec<TxOutputPointer>,
        },
        3165 => UnauthorizedGovernanceAction,
        3166 => ReferenceScriptsTooLarge {
            measured_reference_scripts: NumberOfBytes,
            maximum_reference_scripts: NumberOfBytes,
        },
        3167 => UnknownVoters {
            unknown_voters: Vec<Value>, // TODO:
        },
        3168 => EmptyTreasuryWithdrawal,
        3997 => UnexpectedMempoolError(Value),
        3998 => UnrecognizedCertificateType,
        -32602 => Deserialization {
            byron: String,
            shelley: String,
            allegra: String,
            mary: String,
            alonzo: String,
            babbage: String,
            conway: String,
        },
        _ => Unknown { error: Value }
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct CommitteeMember {
    /// Hex-encoded 28-byte blake2b hash digest
    pub id: String,
    pub from: CredentialOrigin,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Network {
    Mainnet,
    Testnet,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum NetworkMismatchDiscriminatedType {
    Address,
    RewardAccount,
    StakePoolCertificate,
    Transaction,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InsufficientlyFundedOutput {
    pub output: Utxo,
    pub minimum_required_value: AdaBalance,
}

#[derive(Debug, Clone, Deserialize)]
pub struct SubmitResult {
    pub transaction: TxId,
}

// pub type SubmitResponse = RpcResponse<SubmitResult, EvaluationError>;
