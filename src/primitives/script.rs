use std::collections::HashMap;
use std::ops::{Deref, DerefMut};

pub use hydrant::primitives::{Datum, DatumHash, Script, ScriptHash, ScriptKind};

use super::{Hash, Input, Policy, RewardAccount};

#[derive(PartialEq, Eq, Debug, Clone)]
pub enum DatumOption {
    Hash(DatumHash),
    Inline(Vec<u8>),
}

#[derive(PartialEq, Eq, Debug, Clone)]
pub enum RedeemerPurpose {
    Spend(Input),
    Mint(Policy),
    Cert(Hash<28>),
    Reward(RewardAccount),
}

impl std::hash::Hash for RedeemerPurpose {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        let tag_spend: u8 = 0;
        let tag_mint: u8 = 1;
        let tag_cert: u8 = 2;
        let tag_reward: u8 = 3;

        match self {
            RedeemerPurpose::Spend(input) => {
                std::hash::Hash::hash(&tag_spend, state);
                std::hash::Hash::hash(input, state);
            }
            RedeemerPurpose::Mint(policy) => {
                std::hash::Hash::hash(&tag_mint, state);
                std::hash::Hash::hash(policy, state);
            }
            RedeemerPurpose::Cert(script_hash) => {
                std::hash::Hash::hash(&tag_cert, state);
                std::hash::Hash::hash(script_hash, state);
            }
            RedeemerPurpose::Reward(account) => {
                std::hash::Hash::hash(&tag_reward, state);
                std::hash::Hash::hash(account, state);
            }
        }
    }
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
