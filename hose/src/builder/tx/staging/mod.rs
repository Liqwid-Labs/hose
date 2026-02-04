use std::collections::{BTreeMap, HashMap};

use hydrant::primitives::AssetId;
use pallas::codec::minicbor;
use pallas::ledger::primitives::conway::AuxiliaryData;

use super::TxBuilderError;
use crate::primitives::{
    Address, AssetsDelta, Certificate, Datum, DatumHash, ExUnits, Hash, Input, Output, PubKeyHash,
    RedeemerPurpose, Redeemers, RewardAccount, Script, ScriptHash, ScriptKind,
};

mod build;

#[derive(Default, PartialEq, Eq, Debug, Clone)]
pub struct StagingTransaction {
    pub inputs: Vec<Input>,
    pub reference_inputs: Vec<Input>,
    pub outputs: Vec<Output>,
    pub fee: Option<u64>,
    pub mint: AssetsDelta,
    pub valid_from_slot: Option<u64>,
    pub invalid_from_slot: Option<u64>,
    pub network_id: Option<u8>,
    pub collateral_inputs: Vec<Input>,
    pub collateral_output: Option<Output>,
    pub disclosed_signers: Option<Vec<PubKeyHash>>,
    pub scripts: HashMap<ScriptHash, Script>,
    pub datums: HashMap<DatumHash, Datum>,
    pub redeemers: Option<Redeemers>,
    pub script_data_hash: Option<Hash<32>>,
    pub signature_amount_override: Option<u8>,
    pub change_address: Option<Address>,
    pub language_view: Option<pallas::ledger::primitives::conway::LanguageView>,
    pub auxiliary_data: Option<AuxiliaryData>,
    pub certificates: Vec<Certificate>,
    pub withdrawals: BTreeMap<RewardAccount, u64>,
    // pub updates: TODO
    // pub phase_2_valid: TODO
}

impl StagingTransaction {
    pub fn new() -> Self {
        Default::default()
    }

    pub fn input(mut self, input: Input) -> Self {
        self.inputs.push(input);
        self
    }

    pub fn remove_input(mut self, input: Input) -> Self {
        self.inputs.retain(|x| *x != input);
        self
    }

    pub fn reference_input(mut self, input: Input) -> Self {
        self.reference_inputs.push(input);
        self
    }

    pub fn remove_reference_input(mut self, input: Input) -> Self {
        self.reference_inputs.retain(|x| *x != input);
        self
    }

    pub fn output(mut self, output: Output) -> Self {
        self.outputs.push(output);
        self
    }

    pub fn remove_output(mut self, index: usize) -> Self {
        self.outputs.remove(index);
        self
    }

    pub fn fee(mut self, fee: u64) -> Self {
        self.fee = Some(fee);
        self
    }

    pub fn clear_fee(mut self) -> Self {
        self.fee = None;
        self
    }

    pub fn mint_asset(
        mut self,
        policy: Hash<28>,
        name: Vec<u8>,
        amount: i64,
    ) -> Result<Self, TxBuilderError> {
        if name.len() > 32 {
            return Err(TxBuilderError::AssetNameTooLong);
        }

        self.mint
            .entry(AssetId::new(policy, name.clone()))
            .and_modify(|asset_amount| *asset_amount += amount)
            .or_insert(amount);

        Ok(self)
    }

    pub fn remove_mint_asset(mut self, policy: Hash<28>, name: Vec<u8>) -> Self {
        self.mint.remove(&AssetId::new(policy, name));
        self
    }

    pub fn valid_from_slot(mut self, slot: u64) -> Self {
        self.valid_from_slot = Some(slot);
        self
    }

    pub fn clear_valid_from_slot(mut self) -> Self {
        self.valid_from_slot = None;
        self
    }

    pub fn invalid_from_slot(mut self, slot: u64) -> Self {
        self.invalid_from_slot = Some(slot);
        self
    }

    pub fn clear_invalid_from_slot(mut self) -> Self {
        self.invalid_from_slot = None;
        self
    }

    pub fn network_id(mut self, id: u8) -> Self {
        self.network_id = Some(id);
        self
    }

    pub fn clear_network_id(mut self) -> Self {
        self.network_id = None;
        self
    }

    pub fn collateral_input(mut self, input: Input) -> Self {
        self.collateral_inputs.push(input);
        self
    }

    pub fn remove_collateral_input(mut self, input: Input) -> Self {
        self.collateral_inputs.retain(|x| *x != input);
        self
    }

    pub fn collateral_output(mut self, output: Output) -> Self {
        self.collateral_output = Some(output);
        self
    }

    pub fn clear_collateral_output(mut self) -> Self {
        self.collateral_output = None;
        self
    }

    pub fn disclosed_signer(mut self, pub_key_hash: PubKeyHash) -> Self {
        let mut disclosed_signers = self.disclosed_signers.unwrap_or_default();
        disclosed_signers.push(Hash(*pub_key_hash));
        self.disclosed_signers = Some(disclosed_signers);
        self
    }

    pub fn remove_disclosed_signer(mut self, pub_key_hash: PubKeyHash) -> Self {
        let mut disclosed_signers = self.disclosed_signers.unwrap_or_default();
        disclosed_signers.retain(|x| *x != Hash(*pub_key_hash));
        self.disclosed_signers = Some(disclosed_signers);
        self
    }

    pub fn script(mut self, language: ScriptKind, bytes: Vec<u8>) -> Self {
        let hash = language.hash(&bytes);
        self.scripts.insert(
            hash,
            Script {
                kind: language,
                hash,
                bytes,
            },
        );
        self
    }

    pub fn remove_script_by_hash(mut self, script_hash: Hash<28>) -> Self {
        self.scripts.remove(&script_hash);
        self
    }

    pub fn datum(mut self, datum: Vec<u8>) -> Self {
        let datum = Datum::new(datum);
        self.datums.insert(datum.hash, datum);
        self
    }

    pub fn remove_datum(mut self, datum: Vec<u8>) -> Self {
        self.datums.remove(&Datum::new(datum).hash);
        self
    }

    pub fn remove_datum_by_hash(mut self, datum_hash: DatumHash) -> Self {
        self.datums.remove(&Hash(*datum_hash));
        self
    }

    pub fn language_view(mut self, plutus_version: ScriptKind, cost_model: Vec<i64>) -> Self {
        self.language_view = match plutus_version {
            ScriptKind::PlutusV1 => Some(pallas::ledger::primitives::conway::LanguageView(
                0, cost_model,
            )),
            ScriptKind::PlutusV2 => Some(pallas::ledger::primitives::conway::LanguageView(
                1, cost_model,
            )),
            ScriptKind::PlutusV3 => Some(pallas::ledger::primitives::conway::LanguageView(
                2, cost_model,
            )),
            ScriptKind::Native => None,
        };

        self
    }

    pub fn add_spend_redeemer(
        mut self,
        input: Input,
        plutus_data: Vec<u8>,
        ex_units: Option<ExUnits>,
    ) -> Self {
        let mut rdmrs = self.redeemers.unwrap_or_default();
        rdmrs.insert(RedeemerPurpose::Spend(input), (plutus_data, ex_units));
        self.redeemers = Some(rdmrs);

        self
    }

    pub fn remove_spend_redeemer(mut self, input: Input) -> Self {
        let mut rdmrs = self.redeemers.unwrap_or_default();
        rdmrs.remove(&RedeemerPurpose::Spend(input));
        self.redeemers = Some(rdmrs);

        self
    }

    pub fn add_mint_redeemer(
        mut self,
        policy: Hash<28>,
        plutus_data: Vec<u8>,
        ex_units: Option<ExUnits>,
    ) -> Self {
        let mut rdmrs = self.redeemers.unwrap_or_default();
        rdmrs.insert(
            RedeemerPurpose::Mint(Hash(*policy)),
            (plutus_data, ex_units),
        );
        self.redeemers = Some(rdmrs);

        self
    }

    pub fn add_reward_redeemer(
        mut self,
        reward_account: RewardAccount,
        plutus_data: Vec<u8>,
        ex_units: Option<ExUnits>,
    ) -> Self {
        let mut rdmrs = self.redeemers.unwrap_or_default();
        rdmrs.insert(
            RedeemerPurpose::Reward(reward_account),
            (plutus_data, ex_units),
        );
        self.redeemers = Some(rdmrs);

        self
    }

    pub fn remove_mint_redeemer(mut self, policy: Hash<28>) -> Self {
        let mut rdmrs = self.redeemers.unwrap_or_default();
        rdmrs.remove(&RedeemerPurpose::Mint(Hash(*policy)));
        self.redeemers = Some(rdmrs);

        self
    }

    pub fn add_cert_redeemer(
        mut self,
        script_hash: Hash<28>,
        plutus_data: Vec<u8>,
        ex_units: Option<ExUnits>,
    ) -> Self {
        let mut rdmrs = self.redeemers.unwrap_or_default();
        rdmrs.insert(RedeemerPurpose::Cert(script_hash), (plutus_data, ex_units));
        self.redeemers = Some(rdmrs);

        self
    }

    pub fn add_certificate(mut self, certificate: Certificate) -> Self {
        let credential_hash = certificate.credential_hash();
        self.certificates
            .retain(|c| c.credential_hash() != credential_hash);
        self.certificates.push(certificate);
        self
    }

    pub fn apply_stake_credential_deposit(mut self, deposit: u64) -> Self {
        for cert in &mut self.certificates {
            match cert {
                Certificate::StakeRegistrationScript {
                    deposit: cert_deposit,
                    ..
                } => {
                    *cert_deposit = Some(deposit);
                }
                Certificate::StakeDeregistrationScript {
                    deposit: cert_deposit,
                    ..
                } => {
                    *cert_deposit = Some(deposit);
                }
                Certificate::StakeRegistration {
                    deposit: cert_deposit,
                    ..
                } => {
                    *cert_deposit = Some(deposit);
                }
                Certificate::StakeDeregistration {
                    deposit: cert_deposit,
                    ..
                } => {
                    *cert_deposit = Some(deposit);
                }
                _ => {}
            }
        }
        self
    }

    pub fn remove_certificate_by_script_hash(mut self, script_hash: Hash<28>) -> Self {
        self.certificates
            .retain(|c| c.script_hash() != Some(script_hash));
        self
    }

    pub fn remove_certificate_by_pub_key_hash(mut self, pub_key_hash: Hash<28>) -> Self {
        self.certificates.retain(|c| match c {
            Certificate::StakeRegistration {
                pub_key_hash: hash, ..
            } => *hash != pub_key_hash,
            Certificate::StakeDeregistration {
                pub_key_hash: hash, ..
            } => *hash != pub_key_hash,
            Certificate::StakeDelegation {
                pub_key_hash: hash, ..
            } => *hash != pub_key_hash,
            _ => true,
        });
        self
    }

    pub fn withdrawal(mut self, reward_account: RewardAccount, amount: u64) -> Self {
        self.withdrawals.insert(reward_account, amount);
        self
    }

    pub fn remove_withdrawal(mut self, reward_account: &RewardAccount) -> Self {
        self.withdrawals.remove(reward_account);
        self
    }
    pub fn signature_amount_override(mut self, amount: u8) -> Self {
        self.signature_amount_override = Some(amount);
        self
    }

    pub fn clear_signature_amount_override(mut self) -> Self {
        self.signature_amount_override = None;
        self
    }

    pub fn change_address(mut self, address: Address) -> Self {
        self.change_address = Some(address);
        self
    }

    pub fn clear_change_address(mut self) -> Self {
        self.change_address = None;
        self
    }

    pub fn add_auxiliary_data(mut self, data: Vec<u8>) -> Self {
        if let Ok(aux) = minicbor::decode::<AuxiliaryData>(data.as_ref()) {
            self.auxiliary_data = Some(aux);
        }
        self
    }

    pub fn clear_auxiliary_data(mut self) -> Self {
        self.auxiliary_data = None;
        self
    }
}

#[cfg(test)]
mod tests;
