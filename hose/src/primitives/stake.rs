use crate::primitives::Hash;

#[derive(PartialEq, Eq, Debug, Clone)]
pub enum Certificate {
    StakeRegistration {
        pub_key_hash: Hash<28>,
        // Note: a deposit is always required. A value of None here just means that the value of
        // the deposit is to be retrieved from the protocol params.
        deposit: Option<u64>,
    },
    StakeDeregistration {
        pub_key_hash: Hash<28>,
        // Note: a deposit is always required. A value of None here just means that the value of
        // the deposit is to be retrieved from the protocol params.
        deposit: Option<u64>,
    },
    StakeDelegation {
        pub_key_hash: Hash<28>,
        pool_id: Hash<28>,
    },
    StakeRegistrationScript {
        script_hash: Hash<28>,
        // Note: a deposit is always required. A value of None here just means that the value of
        // the deposit is to be retrieved from the protocol params.
        deposit: Option<u64>,
    },
    StakeDeregistrationScript {
        script_hash: Hash<28>,
        // Note: a deposit is always required. A value of None here just means that the value of
        // the deposit is to be retrieved from the protocol params.
        deposit: Option<u64>,
    },
    StakeDelegationScript {
        script_hash: Hash<28>,
        pool_id: Hash<28>,
    },
}

impl Certificate {
    pub fn script_hash(&self) -> Option<Hash<28>> {
        match self {
            Certificate::StakeRegistrationScript { script_hash, .. } => Some(*script_hash),
            Certificate::StakeDeregistrationScript { script_hash, .. } => Some(*script_hash),
            Certificate::StakeDelegationScript { script_hash, .. } => Some(*script_hash),
            _ => None,
        }
    }

    pub fn credential_hash(&self) -> Hash<28> {
        match self {
            Certificate::StakeRegistration { pub_key_hash, .. } => *pub_key_hash,
            Certificate::StakeDeregistration { pub_key_hash, .. } => *pub_key_hash,
            Certificate::StakeDelegation { pub_key_hash, .. } => *pub_key_hash,
            Certificate::StakeRegistrationScript { script_hash, .. } => *script_hash,
            Certificate::StakeDeregistrationScript { script_hash, .. } => *script_hash,
            Certificate::StakeDelegationScript { script_hash, .. } => *script_hash,
        }
    }

    pub fn deposit(&self) -> Option<u64> {
        match self {
            Certificate::StakeRegistration { deposit, .. } => *deposit,
            Certificate::StakeDeregistration { deposit, .. } => *deposit,
            Certificate::StakeRegistrationScript { deposit, .. } => *deposit,
            Certificate::StakeDeregistrationScript { deposit, .. } => *deposit,
            _ => None,
        }
    }

    pub fn deposit_delta(&self) -> i64 {
        match self {
            // TODO: Should we error if this is called before the deposit has been populated?
            Certificate::StakeRegistration { deposit, .. } => deposit.unwrap_or(0) as i64,
            Certificate::StakeDeregistration { deposit, .. } => -(deposit.unwrap_or(0) as i64),
            Certificate::StakeRegistrationScript { deposit, .. } => deposit.unwrap_or(0) as i64,
            Certificate::StakeDeregistrationScript { deposit, .. } => {
                -(deposit.unwrap_or(0) as i64)
            }
            _ => 0,
        }
    }
}
