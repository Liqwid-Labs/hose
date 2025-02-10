use num_enum::FromPrimitive;
use serde::{Deserialize, Serialize};

pub enum RequestMethod {
    SubmitTransaction,
    EvaluateTransaction,
}

impl From<RequestMethod> for String {
    fn from(method: RequestMethod) -> Self {
        match method {
            RequestMethod::SubmitTransaction => "submitTransaction".into(),
            RequestMethod::EvaluateTransaction => "evaluateTransaction".into(),
        }
    }
}

#[derive(Serialize, Debug)]
pub struct Request {
    pub jsonrpc: String,
    pub method: String,
    pub id: Option<String>,
    pub params: Option<serde_json::Value>,
}

#[derive(Deserialize, Debug)]
pub struct ErrorResponse {
    pub code: ErrorResponseCode,
    pub message: String,
    pub data: serde_json::Value,
}

#[derive(Deserialize, Debug)]
#[serde(untagged)]
pub enum Response {
    Error {
        jsonrpc: String,
        method: String,
        id: Option<String>,
        error: ErrorResponse,
    },
    Result {
        jsonrpc: String,
        method: String,
        id: Option<String>,
        result: serde_json::Value,
    },
}

impl Response {
    pub fn id(&self) -> Option<String> {
        match self {
            Response::Error { id, .. } => id.clone(),
            Response::Result { id, .. } => id.clone(),
        }
    }
}

/// Errors that can occur during transaction validation and submission
#[derive(Deserialize, Debug, Clone, PartialEq, Eq, FromPrimitive)]
#[repr(i32)]
#[serde(from = "i32")]
pub enum ErrorResponseCode {
    /// Failed to deserialize in any of the known eras
    DeserializationError = -32602,

    /// Unable to acquire the ledger state at the request point.
    QueryAcquireFailed = 2000,

    /// An era mismatch between a client request and the era the ledger is in. This may occur when running queries on a syncing node and/or when the node is crossing an era.
    QueryEraMismatch = 2001,

    /// Some query is not available for the requested ledger era.
    QueryUnavailableInCurrentEra = 2002,

    /// Previously acquired ledger state is no longer available.
    QueryAcquireExpired = 2003,

    /// Something went wrong (e.g. misconfiguration) in reading genesis file for the latest era.
    QueryInvalidGenesis = 2004,

    /// Returned when trying to evaluate execution units of a pre-Alonzo transaction.
    /// Note that this isn't possible with Ogmios because transactions are always de-serialized as Alonzo transactions.
    IncompatibleEra = 3000,

    /// Returned when trying to evaluate execution units of an era that is now considered too old and is no longer supported.
    /// This can solved by using a more recent transaction format.
    UnsupportedEra = 3001,

    /// Happens when providing an additional UTXO set which overlaps with the UTXO on-chain.
    OverlappingAdditionalUtxo = 3002,

    /// Happens when attempting to evaluate execution units on a node that isn't enough synchronized.
    NodeTipTooOld = 3003,

    /// The transaction is malformed or missing information; making evaluation impossible.
    CannotCreateEvaluationContext = 3004,

    /// Failed to submit the transaction in the current era. This may happen when trying to submit
    /// a transaction near an era boundary (i.e. at the moment of a hard-fork).
    EraMismatch = 3005,

    /// One or more script execution terminated with an error.
    ScriptExecutionFailure = 3010,

    /// Some script witnesses are missing. Indeed, any script used in a transaction (when spending, minting, etc...)
    /// must be provided in full with the transaction. Scripts must therefore be added either to the witness set or
    /// provided as a reference input should you use plutus:v2 or higher and a format from Babbage and beyond.
    InvalidRedeemerPointers = 3011,

    /// Some of the scripts failed to evaluate to a positive outcome.
    ValidationFailure = 3012,

    /// A redeemer points to an input that isn't locked by a Plutus script.
    UnsuitableOutputReference = 3013,

    /// Some signatures are invalid. Only the serialised transaction body, without metadata or witnesses, must be signed.
    InvalidSignatories = 3100,

    /// Some signatures are missing. A signed transaction must carry signatures for all inputs locked by
    /// verification keys or a native script.
    MissingSignatories = 3101,

    /// Some script witnesses are missing.
    MissingScripts = 3102,

    /// The transaction contains failing phase-1 monetary scripts (a.k.a. native scripts).
    FailingNativeScript = 3103,

    /// Extraneous (i.e. non-required) scripts found in the transaction.
    ExtraneousScripts = 3104,

    /// Missing required metadata hash in the transaction body.
    MissingMetadataHash = 3105,

    /// No metadata corresponding to a specified metadata hash.
    MissingMetadata = 3106,

    /// There's a mismatch between the provided metadata hash digest and the one computed from the actual metadata.
    MetadataHashMismatch = 3107,

    /// Invalid metadatum found in transaction metadata.
    InvalidMetadata = 3108,

    /// Missing required redeemer(s) for Plutus scripts.
    MissingRedeemers = 3109,

    /// Extraneous (non-required) redeemers found in the transaction.
    ExtraneousRedeemers = 3110,

    /// Transaction failed because some Plutus scripts are missing their associated datums.
    MissingDatums = 3111,

    /// The transaction failed because it contains datums not associated with any script or output.
    ExtraneousDatums = 3112,

    /// The transaction failed because the provided script integrity hash doesn't match the computed one.
    ScriptIntegrityHashMismatch = 3113,

    /// Trying to spend inputs that are locked by Plutus scripts, but have no associated datums.
    OrphanScriptInputs = 3114,

    /// Transaction is using a Plutus version for which there's no available cost model.
    MissingCostModels = 3115,

    /// Some Plutus scripts in the witness set or in an output are invalid.
    MalformedScripts = 3116,

    /// The transaction contains unknown UTxO references as inputs.
    UnknownOutputReference = 3117,

    /// The transaction is outside of its validity interval.
    OutsideOfValidityInterval = 3118,

    /// The transaction exceeds the maximum size allowed by the protocol.
    TransactionTooLarge = 3119,

    /// Some output values in the transaction are too large.
    ValueTooLarge = 3120,

    /// Transaction must have at least one input, but this one has an empty input set.
    EmptyInputSet = 3121,

    /// Insufficient fee! The transaction doesn't contain enough fee to cover the minimum required by the protocol.
    FeeTooSmall = 3122,

    /// In and out value not conserved. The transaction must exactly balance.
    ValueNotConserved = 3123,

    /// Some discriminated entities in the transaction are configured for another network.
    NetworkMismatch = 3124,

    /// Some outputs have an insufficient amount of Ada attached to them.
    InsufficientlyFundedOutputs = 3125,

    /// Some output associated with legacy / bootstrap (a.k.a. Byron) addresses have attributes that are too large.
    BootstrapAttributesTooLarge = 3126,

    /// The transaction is attempting to mint or burn Ada tokens.
    MintingOrBurningAda = 3127,

    /// Insufficient collateral value for Plutus scripts in the transaction.
    InsufficientCollateral = 3128,

    /// Invalid choice of collateral: an input provided for collateral is locked by script.
    CollateralLockedByScript = 3129,

    /// One of the transaction validity bound is outside any foreseeable future.
    UnforeseeableSlot = 3130,

    /// The transaction contains too many collateral inputs.
    TooManyCollateralInputs = 3131,

    /// The transaction doesn't provide any collateral inputs but it must.
    MissingCollateralInputs = 3132,

    /// One of the input provided as collateral carries something else than Ada tokens.
    NonAdaCollateral = 3133,

    /// The transaction execution budget for scripts execution is above the allowed limit.
    ExecutionUnitsTooLarge = 3134,

    /// There's a mismatch between the declared total collateral amount, and the value computed from the inputs and outputs.
    TotalCollateralMismatch = 3135,

    /// Invalid transaction submitted as valid, or vice-versa.
    SpendsMismatch = 3136,

    /// The transaction contains votes from unauthorized voters.
    UnauthorizedVotes = 3137,

    /// Reference(s) to unknown governance proposals found in transaction.
    UnknownGovernanceProposals = 3138,

    /// The transaction contains an invalid or unauthorized protocol parameters update.
    InvalidProtocolParametersUpdate = 3139,

    /// The transaction references an unknown stake pool as a target for delegation or update.
    UnknownStakePool = 3140,

    /// The transaction contains incomplete or invalid rewards withdrawals.
    IncompleteWithdrawals = 3141,

    /// A stake pool retirement certificate is trying to retire too late in the future.
    RetirementTooLate = 3142,

    /// Stake pool cost declared in a registration or update certificate are below the allowed minimum.
    StakePoolCostTooLow = 3143,

    /// Some hash digest of (optional) stake pool metadata is too long.
    MetadataHashTooLarge = 3144,

    /// Trying to re-register some already known credentials.
    CredentialAlreadyRegistered = 3145,

    /// The transaction references an unknown stake credential.
    UnknownCredential = 3146,

    /// Trying to unregister stake credentials associated to a non empty reward account.
    NonEmptyRewardAccount = 3147,

    /// Invalid or unauthorized genesis delegation.
    InvalidGenesisDelegation = 3148,

    /// Invalid MIR transfer.
    InvalidMIRTransfer = 3149,

    /// The transaction is attempting to withdraw rewards from stake credentials that do not engage in on-chain governance.
    ForbiddenWithdrawal = 3150,

    /// The deposit specified in a stake credential registration does not match the current value set by protocol parameters.
    CredentialDepositMismatch = 3151,

    /// Trying to re-register some already known delegate representative.
    DRepAlreadyRegistered = 3152,

    /// The transaction references an unknown delegate representative.
    DRepNotRegistered = 3153,

    /// The transaction references an unknown constitutional committee member.
    UnknownConstitutionalCommitteeMember = 3154,

    /// There's a mismatch between the proposal deposit amount declared in the transaction and the one expected by the ledger.
    GovernanceProposalDepositMismatch = 3155,

    /// The transaction contains an invalid governance action: it tries to both add members to the committee and remove some of those same members.
    ConflictingCommitteeUpdate = 3156,

    /// The transaction contains an invalid governance action: it tries to add new members to the constitutional committee with a retirement epoch in the past.
    InvalidCommitteeUpdate = 3157,

    /// The transaction is trying to withdraw more funds than specified in a governance action!
    TreasuryWithdrawalMismatch = 3158,

    /// The transaction contains invalid or missing reference to previous (ratified) governance proposals.
    InvalidOrMissingPreviousProposals = 3159,

    /// The transaction contains votes for an expired proposal.
    VotingOnExpiredActions = 3160,

    /// The transaction ran out of execution budget!
    ExecutionBudgetOutOfBounds = 3161,

    /// The new proposed version for a hard-fork isn't a valid version bump.
    InvalidHardForkVersionBump = 3162,

    /// The provided constitution guardrails hash doesn't match the expected on defined in the constitution.
    ConstitutionGuardrailsHashMismatch = 3163,

    /// Identical UTxO references were found in both the transaction inputs and references.
    ConflictingInputsAndReferences = 3164,

    /// The ledger is still in a bootstrapping phase.
    UnauthorizedGovernanceAction = 3165,

    /// Reference scripts are too large
    ReferenceScriptsTooLarge = 3166,

    /// Some voters in the transaction are unknown.
    UnknownVoters = 3167,

    /// Some proposals contain empty treasury withdrawals, which is pointless and a waste of resources.
    EmptyTreasuryWithdrawal = 3168,

    /// A transaction was rejected due to custom rules that prevented it from entering the mempool.
    UnexpectedMempoolError = 3997,

    /// Unrecognized certificate type. This error is a placeholder due to how internal data-types are modeled.
    UnrecognizedCertificateType = 3998,

    /// Not in the list of known errors, contributions welcome!
    #[num_enum(catch_all)]
    UnknownError(i32) = -1,
}
