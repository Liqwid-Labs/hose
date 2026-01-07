use std::collections::HashMap;

use pallas::codec::minicbor;
use pallas::crypto::hash::Hasher;
use pallas::ledger::primitives::conway::AuxiliaryData;

use super::{
    Address, Bytes, DatumBytes, DatumHash, ExUnits, Hash, Input, MintAssets, Output, PubKeyHash,
    RedeemerPurpose, Redeemers, Script, ScriptHash, ScriptKind,
};
use crate::builder::transaction::error::TxBuilderError;

// TODO: Don't make wrapper types public
#[derive(Default, PartialEq, Eq, Debug, Clone)]
pub struct StagingTransaction {
    pub inputs: Option<Vec<Input>>,
    pub reference_inputs: Option<Vec<Input>>,
    pub outputs: Option<Vec<Output>>,
    pub fee: Option<u64>,
    pub mint: Option<MintAssets>,
    pub valid_from_slot: Option<u64>,
    pub invalid_from_slot: Option<u64>,
    pub network_id: Option<u8>,
    pub collateral_inputs: Option<Vec<Input>>,
    pub collateral_output: Option<Output>,
    pub disclosed_signers: Option<Vec<PubKeyHash>>,
    pub scripts: Option<HashMap<ScriptHash, Script>>,
    pub datums: Option<HashMap<DatumHash, DatumBytes>>,
    pub redeemers: Option<Redeemers>,
    pub script_data_hash: Option<Hash<32>>,
    pub signature_amount_override: Option<u8>,
    pub change_address: Option<Address>,
    pub language_view: Option<pallas::ledger::primitives::conway::LanguageView>,
    pub auxiliary_data: Option<AuxiliaryData>,
    // pub certificates: TODO
    // pub withdrawals: TODO
    // pub updates: TODO
    // pub phase_2_valid: TODO
}

impl StagingTransaction {
    pub fn new() -> Self {
        Default::default()
    }

    pub fn input(mut self, input: Input) -> Self {
        let mut txins = self.inputs.unwrap_or_default();
        txins.push(input);
        self.inputs = Some(txins);
        self
    }

    pub fn remove_input(mut self, input: Input) -> Self {
        let mut txins = self.inputs.unwrap_or_default();
        txins.retain(|x| *x != input);
        self.inputs = Some(txins);
        self
    }

    pub fn reference_input(mut self, input: Input) -> Self {
        let mut ref_txins = self.reference_inputs.unwrap_or_default();
        ref_txins.push(input);
        self.reference_inputs = Some(ref_txins);
        self
    }

    pub fn remove_reference_input(mut self, input: Input) -> Self {
        let mut ref_txins = self.reference_inputs.unwrap_or_default();
        ref_txins.retain(|x| *x != input);
        self.reference_inputs = Some(ref_txins);
        self
    }

    pub fn output(mut self, output: Output) -> Self {
        let mut txouts = self.outputs.unwrap_or_default();
        txouts.push(output);
        self.outputs = Some(txouts);
        self
    }

    pub fn remove_output(mut self, index: usize) -> Self {
        let mut txouts = self.outputs.unwrap_or_default();
        txouts.remove(index);
        self.outputs = Some(txouts);
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

        let mut mint = self.mint.unwrap_or_default();

        mint.entry(Hash(*policy))
            .and_modify(|policy_map| {
                policy_map
                    .entry(name.clone().into())
                    .and_modify(|asset_map| {
                        *asset_map += amount;
                    })
                    .or_insert(amount);
            })
            .or_insert_with(|| {
                let mut map: HashMap<Bytes, i64> = HashMap::new();
                map.insert(name.clone().into(), amount);
                map
            });

        self.mint = Some(mint);

        Ok(self)
    }

    pub fn remove_mint_asset(mut self, policy: Hash<28>, name: Vec<u8>) -> Self {
        let mut mint = if let Some(mint) = self.mint {
            mint
        } else {
            return self;
        };

        if let Some(assets) = mint.get_mut(&Hash(*policy)) {
            assets.remove(&name.into());
            if assets.is_empty() {
                mint.remove(&Hash(*policy));
            }
        }

        self.mint = Some(mint);

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
        let mut coll_ins = self.collateral_inputs.unwrap_or_default();
        coll_ins.push(input);
        self.collateral_inputs = Some(coll_ins);
        self
    }

    pub fn remove_collateral_input(mut self, input: Input) -> Self {
        let mut coll_ins = self.collateral_inputs.unwrap_or_default();
        coll_ins.retain(|x| *x != input);
        self.collateral_inputs = Some(coll_ins);
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

    pub fn disclosed_signer(mut self, pub_key_hash: Hash<28>) -> Self {
        let mut disclosed_signers = self.disclosed_signers.unwrap_or_default();
        disclosed_signers.push(Hash(*pub_key_hash));
        self.disclosed_signers = Some(disclosed_signers);
        self
    }

    pub fn remove_disclosed_signer(mut self, pub_key_hash: Hash<28>) -> Self {
        let mut disclosed_signers = self.disclosed_signers.unwrap_or_default();
        disclosed_signers.retain(|x| *x != Hash(*pub_key_hash));
        self.disclosed_signers = Some(disclosed_signers);
        self
    }

    pub fn script(mut self, language: ScriptKind, bytes: Vec<u8>) -> Self {
        let mut scripts = self.scripts.unwrap_or_default();

        let hash = match language {
            ScriptKind::Native => Hasher::<224>::hash_tagged(bytes.as_ref(), 0),
            ScriptKind::PlutusV1 => Hasher::<224>::hash_tagged(bytes.as_ref(), 1),
            ScriptKind::PlutusV2 => Hasher::<224>::hash_tagged(bytes.as_ref(), 2),
            ScriptKind::PlutusV3 => Hasher::<224>::hash_tagged(bytes.as_ref(), 3),
        };

        scripts.insert(
            Hash(*hash),
            Script {
                kind: language,
                bytes: bytes.into(),
            },
        );

        self.scripts = Some(scripts);
        self
    }

    pub fn remove_script_by_hash(mut self, script_hash: Hash<28>) -> Self {
        let mut scripts = self.scripts.unwrap_or_default();

        scripts.remove(&Hash(*script_hash));

        self.scripts = Some(scripts);
        self
    }

    pub fn datum(mut self, datum: Vec<u8>) -> Self {
        let mut datums = self.datums.unwrap_or_default();

        let hash = Hasher::<256>::hash_cbor(&datum);

        datums.insert(Hash(*hash), datum.into());
        self.datums = Some(datums);
        self
    }

    pub fn remove_datum(mut self, datum: Vec<u8>) -> Self {
        let mut datums = self.datums.unwrap_or_default();

        let hash = Hasher::<256>::hash_cbor(&datum);

        datums.remove(&Hash(*hash));
        self.datums = Some(datums);
        self
    }

    pub fn remove_datum_by_hash(mut self, datum_hash: Hash<32>) -> Self {
        let mut datums = self.datums.unwrap_or_default();

        datums.remove(&Hash(*datum_hash));
        self.datums = Some(datums);
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
        rdmrs.insert(
            RedeemerPurpose::Spend(input),
            (plutus_data.into(), ex_units),
        );
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
            (plutus_data.into(), ex_units),
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
