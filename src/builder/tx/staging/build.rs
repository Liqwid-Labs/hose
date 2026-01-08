use std::collections::BTreeMap;
use std::ops::Deref;

use pallas::codec::utils::Bytes;
use pallas::crypto::hash::Hash as PallasHash;
use pallas::ledger::primitives::conway::{
    ExUnits as PallasExUnits, Multiasset, NativeScript, NetworkId, NonZeroInt, PlutusData,
    PlutusScript, Redeemer, RedeemerTag, TransactionBody, TransactionInput, Tx, WitnessSet,
};
use pallas::ledger::primitives::{Fragment, KeepRaw, NonEmptySet};
use pallas::ledger::traverse::ComputeHash;

use crate::builder::tx::{BuiltTransaction, StagingTransaction, TxBuilderError};
use crate::primitives::{ExUnits, Hash, Output, RedeemerPurpose, ScriptKind};

impl StagingTransaction {
    pub fn build_conway(self) -> Result<BuiltTransaction, TxBuilderError> {
        let mut inputs = self
            .inputs
            .iter()
            .map(|x| TransactionInput {
                transaction_id: x.hash.0.into(),
                index: x.index,
            })
            .collect::<Vec<_>>();

        inputs.sort_unstable_by_key(|x| (x.transaction_id, x.index));

        let outputs = self
            .outputs
            .iter()
            .map(Output::build_babbage)
            .collect::<Result<Vec<_>, _>>()?;

        let mut mint: BTreeMap<PallasHash<28>, BTreeMap<Bytes, NonZeroInt>> = BTreeMap::new();

        for (asset_id, amount) in self.mint.iter() {
            let Ok(amount) = NonZeroInt::try_from(*amount) else {
                continue;
            };
            mint.entry(asset_id.policy.0.into())
                .or_default()
                .insert(asset_id.name.clone().into(), amount);
        }

        let mint: Option<Multiasset<NonZeroInt>> =
            (!mint.is_empty()).then(|| mint.into_iter().collect());

        let collateral = NonEmptySet::from_vec(
            self.collateral_inputs
                .iter()
                .map(|x| TransactionInput {
                    transaction_id: x.hash.0.into(),
                    index: x.index,
                })
                .collect(),
        );

        let required_signers = NonEmptySet::from_vec(
            self.disclosed_signers
                .unwrap_or_default()
                .iter()
                .map(|x| x.0.into())
                .collect(),
        );

        let network_id = if let Some(nid) = self.network_id {
            match NetworkId::try_from(nid) {
                Err(()) => return Err(TxBuilderError::InvalidNetworkId),
                Ok(network_id) => Some(network_id),
            }
        } else {
            None
        };

        let collateral_return = self
            .collateral_output
            .as_ref()
            .map(Output::build_babbage)
            .transpose()?;

        let reference_inputs = NonEmptySet::from_vec(
            self.reference_inputs
                .iter()
                .map(|x| TransactionInput {
                    transaction_id: x.hash.0.into(),
                    index: x.index,
                })
                .collect(),
        );

        let (mut native_script, mut plutus_v1_script, mut plutus_v2_script, mut plutus_v3_script) =
            (vec![], vec![], vec![], vec![]);

        for (_, script) in self.scripts {
            match script.kind {
                ScriptKind::Native => {
                    let script = NativeScript::decode_fragment(&script.bytes)
                        .map_err(|_| TxBuilderError::MalformedScript)?;

                    native_script.push(script)
                }
                ScriptKind::PlutusV1 => {
                    let script = PlutusScript::<1>(script.bytes.into());

                    plutus_v1_script.push(script)
                }
                ScriptKind::PlutusV2 => {
                    let script = PlutusScript::<2>(script.bytes.into());

                    plutus_v2_script.push(script)
                }
                ScriptKind::PlutusV3 => {
                    let script = PlutusScript::<3>(script.bytes.into());

                    plutus_v3_script.push(script)
                }
            }
        }

        let plutus_data = self
            .datums
            .iter()
            .map(|(_, datum)| {
                PlutusData::decode_fragment(&datum.bytes)
                    .map_err(|_| TxBuilderError::MalformedDatum)
            })
            .collect::<Result<Vec<_>, _>>()?;

        let mut mint_policies = mint
            .iter()
            .flat_map(|x: &Multiasset<NonZeroInt>| x.iter())
            .map(|(p, _)| *p)
            .collect::<Vec<_>>();

        mint_policies.sort_unstable_by_key(|x| *x);

        let mut redeemers = vec![];

        if let Some(rdmrs) = self.redeemers {
            for (purpose, (pd, ex_units)) in rdmrs.deref().iter() {
                let ex_units = if let Some(ExUnits { mem, steps }) = ex_units {
                    PallasExUnits {
                        mem: *mem,
                        steps: *steps,
                    }
                } else {
                    todo!("ExUnits budget calculation not yet implement") // TODO
                };

                let data = PlutusData::decode_fragment(pd.as_ref())
                    .map_err(|_| TxBuilderError::MalformedDatum)?;

                match purpose {
                    RedeemerPurpose::Spend(txin) => {
                        let index = inputs
                            .iter()
                            .position(|x| (*x.transaction_id, x.index) == (txin.hash.0, txin.index))
                            .ok_or(TxBuilderError::RedeemerTargetMissing)?
                            as u32;

                        redeemers.push(Redeemer {
                            tag: RedeemerTag::Spend,
                            index,
                            data,
                            ex_units,
                        })
                    }
                    RedeemerPurpose::Mint(pid) => {
                        let index = mint_policies
                            .iter()
                            .position(|x| x.as_slice() == pid.0)
                            .ok_or(TxBuilderError::RedeemerTargetMissing)?
                            as u32;

                        redeemers.push(Redeemer {
                            tag: RedeemerTag::Mint,
                            index,
                            data,
                            ex_units,
                        })
                    } // todo!("reward and cert redeemers not yet supported"), // TODO
                }
            }
        };

        let witness_set_redeemers =
            pallas::ledger::primitives::conway::Redeemers::List(redeemers.clone());
        let witness_set_datums = if !plutus_data.is_empty() {
            Some(KeepRaw::from(
                NonEmptySet::from_vec(plutus_data.clone().into_iter().map(KeepRaw::from).collect())
                    .unwrap(),
            ))
        } else {
            None
        };

        let script_data_hash = self.language_view.map(|language_view| {
            let dta = pallas::ledger::primitives::conway::ScriptData {
                redeemers: Some(witness_set_redeemers.clone()),
                datums: witness_set_datums.clone(),
                language_view: Some(language_view),
            };

            dta.hash()
        });

        let mut pallas_tx: Tx = Tx {
            transaction_body: TransactionBody {
                inputs: pallas::ledger::primitives::Set::from(inputs),
                outputs,
                ttl: self.invalid_from_slot,
                validity_interval_start: self.valid_from_slot,
                fee: self.fee.unwrap_or_default(),
                certificates: None,        // TODO
                withdrawals: None,         // TODO
                auxiliary_data_hash: None, // TODO (accept user input)
                mint,
                script_data_hash,
                collateral,
                required_signers,
                network_id,
                collateral_return,
                reference_inputs,
                total_collateral: None,    // TODO
                voting_procedures: None,   // TODO
                proposal_procedures: None, // TODO
                treasury_value: None,      // TODO
                donation: None,            // TODO
            }
            .into(),
            transaction_witness_set: WitnessSet {
                vkeywitness: None,
                native_script: NonEmptySet::from_vec(
                    native_script.into_iter().map(|x| x.into()).collect(),
                ),
                bootstrap_witness: None,
                plutus_v1_script: NonEmptySet::from_vec(plutus_v1_script),
                plutus_v2_script: NonEmptySet::from_vec(plutus_v2_script),
                plutus_v3_script: NonEmptySet::from_vec(plutus_v3_script),
                plutus_data: witness_set_datums,
                redeemer: if redeemers.is_empty() {
                    None
                } else {
                    Some(witness_set_redeemers.into())
                },
            }
            .into(),
            success: true, // TODO
            auxiliary_data: self.auxiliary_data.map(KeepRaw::from).into(),
        };

        // TODO: pallas auxiliary_data_hash should be Hash<32> not Bytes
        pallas_tx.transaction_body.auxiliary_data_hash = pallas_tx
            .auxiliary_data
            .clone()
            .map(|ad| ad.compute_hash())
            .into();

        Ok(BuiltTransaction {
            hash: Hash(*pallas_tx.transaction_body.compute_hash()),
            bytes: pallas_tx.encode_fragment().unwrap(),
            signatures: None,
        })
    }
}
