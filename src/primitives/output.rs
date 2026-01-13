use std::collections::BTreeMap;

use pallas::codec::utils::{Bytes, CborWrap};
use pallas::crypto::hash::Hash as PallasHash;
use pallas::ledger::primitives::conway::{
    self, NativeScript, PlutusData, PlutusScript, PostAlonzoTransactionOutput,
    ScriptRef as PallasScript, TransactionOutput, Value,
};
use pallas::ledger::primitives::{Fragment, PositiveCoin};

use super::{Address, Asset, AssetId, Assets, DatumOption, Hash, Script, ScriptKind};
use crate::builder::tx::TxBuilderError;

#[derive(PartialEq, Eq, Debug, Clone)]
pub struct Output {
    pub address: Address,
    pub lovelace: u64,
    pub assets: Option<Assets>,
    pub datum: Option<DatumOption>,
    pub script: Option<Script>,
}

impl Output {
    pub fn new(address: Address, lovelace: u64) -> Self {
        Self {
            address,
            lovelace,
            assets: None,
            datum: None,
            script: None,
        }
    }

    pub fn add_asset(
        mut self,
        policy: Hash<28>,
        name: Vec<u8>,
        amount: u64,
    ) -> Result<Self, TxBuilderError> {
        if name.len() > 32 {
            return Err(TxBuilderError::AssetNameTooLong);
        }

        let mut assets = self.assets.unwrap_or_default();
        assets.add_asset(Asset::new(policy, name, amount));
        self.assets = Some(assets);

        Ok(self)
    }

    pub fn add_assets(mut self, assets: Assets) -> Result<Self, TxBuilderError> {
        self.assets = Some(self.assets.unwrap_or_default() + assets);
        Ok(self)
    }

    pub fn remove_asset(mut self, policy: Hash<28>, name: Vec<u8>) -> Self {
        let mut assets = self.assets.unwrap_or_default();
        assets.remove(&AssetId::new(policy, name));
        self.assets = Some(assets);
        self
    }

    pub fn remove_assets(mut self, assets_to_remove: Assets) -> Self {
        let mut assets = self.assets.unwrap_or_default();
        for key in assets_to_remove.keys() {
            assets.remove(&key);
        }
        self.assets = Some(assets);
        self
    }

    pub fn set_datum(mut self, bytes: Vec<u8>) -> Self {
        self.datum = Some(DatumOption::Inline(bytes));
        self
    }

    pub fn set_datum_hash(mut self, hash: Hash<32>) -> Self {
        self.datum = Some(DatumOption::Hash(hash));
        self
    }

    pub fn clear_datum(mut self) -> Self {
        self.datum = None;
        self
    }

    pub fn set_script(mut self, language: ScriptKind, bytes: Vec<u8>) -> Self {
        self.script = Some(Script::new(language, bytes));
        self
    }

    pub fn clear_script(mut self) -> Self {
        self.script = None;
        self
    }

    pub fn build_babbage(&self) -> Result<TransactionOutput<'_>, TxBuilderError> {
        let mut assets: BTreeMap<PallasHash<28>, BTreeMap<Bytes, PositiveCoin>> = BTreeMap::new();

        for (asset_id, amount) in self.assets.clone().unwrap_or_default().iter() {
            let Ok(amount) = PositiveCoin::try_from(*amount) else {
                continue;
            };
            assets
                .entry(asset_id.policy.0.into())
                .or_default()
                .insert(asset_id.name.clone().into(), amount);
        }

        let assets = (!assets.is_empty()).then(|| assets.into_iter().collect());

        let value = match assets {
            Some(assets) => Value::Multiasset(self.lovelace, assets),
            None => Value::Coin(self.lovelace),
        };

        let datum_option = match self.datum.clone() {
            Some(DatumOption::Hash(dh)) => Some(conway::DatumOption::Hash(dh.0.into())),
            Some(DatumOption::Inline(pd)) => {
                let pd = PlutusData::decode_fragment(pd.as_ref())
                    .map_err(|_| TxBuilderError::MalformedDatum)?;
                Some(conway::DatumOption::Data(CborWrap(pd.into())))
            }
            None => None,
        };

        let script_ref = if let Some(ref s) = self.script {
            let script = match s.kind {
                ScriptKind::Native => PallasScript::NativeScript(
                    NativeScript::decode_fragment(s.bytes.as_ref())
                        .map_err(|_| TxBuilderError::MalformedScript)?
                        .into(),
                ),
                ScriptKind::PlutusV1 => {
                    PallasScript::PlutusV1Script(PlutusScript::<1>(s.bytes.clone().into()))
                }
                ScriptKind::PlutusV2 => {
                    PallasScript::PlutusV2Script(PlutusScript::<2>(s.bytes.clone().into()))
                }
                ScriptKind::PlutusV3 => {
                    PallasScript::PlutusV3Script(PlutusScript::<3>(s.bytes.clone().into()))
                }
            };

            Some(CborWrap(script))
        } else {
            None
        };

        Ok(TransactionOutput::PostAlonzo(
            PostAlonzoTransactionOutput {
                address: self.address.to_vec().into(),
                value,
                datum_option: datum_option.map(|x| x.into()),
                script_ref,
            }
            .into(),
        ))
    }
}
