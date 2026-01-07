use std::collections::HashMap;
use std::ops::{Deref, DerefMut};

use super::{Bytes, DatumBytes, Input, PolicyId, ScriptBytes};

#[derive(PartialEq, Eq, Debug, Clone, Copy)]
pub enum ScriptKind {
    Native,
    PlutusV1,
    PlutusV2,
    PlutusV3,
}

#[derive(PartialEq, Eq, Debug, Clone)]
pub struct Script {
    pub kind: ScriptKind,
    pub bytes: ScriptBytes,
}

impl Script {
    pub fn new(kind: ScriptKind, bytes: Vec<u8>) -> Self {
        Self {
            kind,
            bytes: bytes.into(),
        }
    }
}

#[derive(PartialEq, Eq, Debug, Clone)]
pub enum DatumKind {
    Hash,
    Inline,
}

#[derive(PartialEq, Eq, Debug, Clone)]
pub struct Datum {
    pub kind: DatumKind,
    pub bytes: DatumBytes,
}

#[derive(PartialEq, Eq, Hash, Debug, Clone)]
pub enum RedeemerPurpose {
    Spend(Input),
    Mint(PolicyId),
    // Reward TODO
    // Cert TODO
}

#[derive(PartialEq, Eq, Debug, Clone)]
pub struct ExUnits {
    pub mem: u64,
    pub steps: u64,
}

#[derive(PartialEq, Eq, Debug, Default, Clone)]
pub struct Redeemers(HashMap<RedeemerPurpose, (Bytes, Option<ExUnits>)>);

impl Deref for Redeemers {
    type Target = HashMap<RedeemerPurpose, (Bytes, Option<ExUnits>)>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for Redeemers {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl Redeemers {
    pub fn from_map(map: HashMap<RedeemerPurpose, (Bytes, Option<ExUnits>)>) -> Self {
        Self(map)
    }
}
