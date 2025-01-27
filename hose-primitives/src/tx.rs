use pallas::{crypto::hash::Hash, ledger::addresses::Address};

pub type TxHash = Hash<32>;

#[derive(Debug, Clone)]
pub struct Asset {
    // TODO: better types possible?
    pub policy_id: String,
    pub name: String,
    pub quantity: u64,
}

#[derive(Debug, Clone)]
pub struct TxO {
    pub address: Address,
    pub tx_hash: TxHash,
    pub txo_index: u64,
    pub lovelace: u64,
    pub assets: Vec<Asset>,
    // datum
    // scripts
}

pub type UTxO = TxO;

impl Into<pallas::txbuilder::Input> for TxO {
    fn into(self) -> pallas::txbuilder::Input {
        pallas::txbuilder::Input::new(self.tx_hash, self.txo_index)
    }
}

impl Into<pallas::txbuilder::Output> for TxO {
    fn into(self) -> pallas::txbuilder::Output {
        pallas::txbuilder::Output {
            address: self.address.into(),
            lovelace: self.lovelace,
            // TODO:
            assets: None,
            datum: None,
            script: None,
        }
    }
}
