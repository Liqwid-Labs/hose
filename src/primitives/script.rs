use std::collections::HashMap;
use std::ops::{Deref, DerefMut};

pub use hydrant::primitives::{Datum, DatumHash, Script, ScriptHash, ScriptKind};

use super::{Input, Policy};

#[derive(PartialEq, Eq, Debug, Clone)]
pub enum DatumOption {
    Hash(DatumHash),
    Inline(Vec<u8>),
}

#[derive(PartialEq, Eq, Hash, Debug, Clone)]
pub enum RedeemerPurpose {
    Spend(Input),
    Mint(Policy),
    // Reward TODO
    // Cert TODO
}

#[derive(PartialEq, Eq, Debug, Clone)]
pub struct ExUnits {
    pub mem: u64,
    pub steps: u64,
}

#[derive(PartialEq, Eq, Debug, Default, Clone)]
pub struct Redeemers(HashMap<RedeemerPurpose, (Vec<u8>, Option<ExUnits>)>);

impl Deref for Redeemers {
    type Target = HashMap<RedeemerPurpose, (Vec<u8>, Option<ExUnits>)>;

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
    pub fn from_map(map: HashMap<RedeemerPurpose, (Vec<u8>, Option<ExUnits>)>) -> Self {
        Self(map)
    }
}
