use crate::primitives::Hash;

#[derive(PartialEq, Eq, Debug, Clone)]
pub enum Certificate {
    // TODO: key credential registrations
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
}

impl Certificate {
    pub fn script_hash(&self) -> Hash<28> {
        match self {
            Certificate::StakeRegistrationScript { script_hash, .. } => *script_hash,
            Certificate::StakeDeregistrationScript { script_hash, .. } => *script_hash,
        }
    }

    pub fn deposit(&self) -> Option<u64> {
        match self {
            Certificate::StakeRegistrationScript { deposit, .. } => *deposit,
            Certificate::StakeDeregistrationScript { deposit, .. } => *deposit,
        }
    }

    pub fn deposit_delta(&self) -> i64 {
        match self {
            // TODO: Should we error if this is called before the deposit has been populated?
            Certificate::StakeRegistrationScript { deposit, .. } => deposit.unwrap_or(0) as i64,
            Certificate::StakeDeregistrationScript { deposit, .. } => {
                -(deposit.unwrap_or(0) as i64)
            }
        }
    }
}
