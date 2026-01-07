use std::collections::HashMap;
use std::ops::{Deref, DerefMut};

use super::{AssetName, Bytes, PolicyId};

#[derive(PartialEq, Eq, Debug, Clone, Default)]
pub struct OutputAssets(HashMap<PolicyId, HashMap<AssetName, u64>>);

impl Deref for OutputAssets {
    type Target = HashMap<PolicyId, HashMap<Bytes, u64>>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for OutputAssets {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl OutputAssets {
    pub fn from_map(map: HashMap<PolicyId, HashMap<Bytes, u64>>) -> Self {
        Self(map)
    }
}

#[derive(PartialEq, Eq, Debug, Clone, Default)]
pub struct MintAssets(pub HashMap<PolicyId, HashMap<AssetName, i64>>);

impl Deref for MintAssets {
    type Target = HashMap<PolicyId, HashMap<Bytes, i64>>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for MintAssets {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl MintAssets {
    pub fn new() -> Self {
        MintAssets(HashMap::new())
    }

    pub fn from_map(map: HashMap<PolicyId, HashMap<Bytes, i64>>) -> Self {
        Self(map)
    }
}
