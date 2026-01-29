use crate::primitives::Hash;

#[derive(PartialEq, Eq, Debug, Clone)]
pub enum Certificate {
    // TODO: key credential registrations
    StakeRegistrationScript { script_hash: Hash<28>, deposit: u64 },
}

impl Certificate {
    pub fn script_hash(&self) -> Hash<28> {
        match self {
            Certificate::StakeRegistrationScript { script_hash, .. } => *script_hash,
        }
    }

    pub fn deposit(&self) -> u64 {
        match self {
            Certificate::StakeRegistrationScript { deposit, .. } => *deposit,
        }
    }
}
