use std::collections::HashMap;

use super::{Address, Bytes, Datum, DatumKind, Hash, OutputAssets, Script, ScriptKind, TxHash};
use crate::builder::transaction::error::TxBuilderError;

#[derive(PartialEq, Eq, Debug, Hash, Clone)]
pub struct Input {
    pub tx_hash: TxHash,
    pub txo_index: u64,
}

impl Input {
    pub fn new(tx_hash: Hash<32>, txo_index: u64) -> Self {
        Self { tx_hash, txo_index }
    }
}

impl Into<hydrant::primitives::TxOutputPointer> for Input {
    fn into(self) -> hydrant::primitives::TxOutputPointer {
        hydrant::primitives::TxOutputPointer::new(self.tx_hash, self.txo_index)
    }
}
impl Into<hydrant::primitives::TxOutputPointer> for &Input {
    fn into(self) -> hydrant::primitives::TxOutputPointer {
        hydrant::primitives::TxOutputPointer::new(self.tx_hash, self.txo_index)
    }
}
impl From<hydrant::primitives::TxOutputPointer> for Input {
    fn from(value: hydrant::primitives::TxOutputPointer) -> Self {
        Input::new(value.hash, value.index as u64)
    }
}

#[derive(PartialEq, Eq, Debug, Clone)]
pub struct Output {
    pub address: Address,
    pub lovelace: u64,
    pub assets: Option<OutputAssets>,
    pub datum: Option<Datum>,
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

        assets
            .entry(Hash(*policy))
            .and_modify(|policy_map| {
                policy_map
                    .entry(name.clone().into())
                    .and_modify(|asset_map| {
                        *asset_map += amount;
                    })
                    .or_insert(amount);
            })
            .or_insert_with(|| {
                let mut map: HashMap<Bytes, u64> = HashMap::new();
                map.insert(name.clone().into(), amount);
                map
            });

        self.assets = Some(assets);

        Ok(self)
    }

    pub fn add_assets(mut self, assets: OutputAssets) -> Result<Self, TxBuilderError> {
        for (policy, asset_map) in assets.iter() {
            for (asset, amount) in asset_map.iter() {
                self = self.add_asset(Hash::from(policy.0), asset.0.clone(), *amount)?;
            }
        }
        Ok(self)
    }

    pub fn set_inline_datum(mut self, plutus_data: Vec<u8>) -> Self {
        self.datum = Some(Datum {
            kind: DatumKind::Inline,
            bytes: plutus_data.into(),
        });

        self
    }

    pub fn set_datum_hash(mut self, datum_hash: Hash<32>) -> Self {
        self.datum = Some(Datum {
            kind: DatumKind::Hash,
            bytes: datum_hash.to_vec().into(),
        });

        self
    }

    pub fn set_inline_script(mut self, language: ScriptKind, bytes: Vec<u8>) -> Self {
        self.script = Some(Script {
            kind: language,
            bytes: bytes.into(),
        });

        self
    }
}
