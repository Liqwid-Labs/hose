use pallas::codec::utils::Bytes;
use pallas::ledger::addresses::Network;

use crate::primitives::Hash;

// Conway CDDL: "reward addresses: bits 7-5: 111; bit 4: credential is keyhash/scripthash; bits 3-0: network id"
const REWARD_ADDRESS_PREFIX: u8 = 0b1110_0000;
const REWARD_ADDRESS_CREDENTIAL_SCRIPT: u8 = 0b0001_0000;
const REWARD_ADDRESS_NETWORK_MASK: u8 = 0b0000_1111;

fn network_id_from_network(network: Network) -> u8 {
    match network {
        Network::Testnet => 0,
        Network::Mainnet => 1,
        Network::Other(n) => n,
    }
}

fn network_from_network_id(network_id: u8) -> Network {
    match network_id {
        0 => Network::Testnet,
        1 => Network::Mainnet,
        n => Network::Other(n),
    }
}

#[derive(PartialEq, Eq, PartialOrd, Ord, Hash, Debug, Clone)]
pub struct RewardAccount(Bytes);

impl RewardAccount {
    pub fn from_script_hash(network: Network, script_hash: Hash<28>) -> Self {
        // TODO: helper for key credential accounts (note that the header is different).
        let network_id = network_id_from_network(network);

        let header = REWARD_ADDRESS_PREFIX
            | REWARD_ADDRESS_CREDENTIAL_SCRIPT
            | (network_id & REWARD_ADDRESS_NETWORK_MASK);
        let mut bytes = Vec::with_capacity(1 + 28);
        bytes.push(header);
        bytes.extend_from_slice(&script_hash.0);

        RewardAccount(Bytes::from(bytes))
    }

    pub fn from_script_hash_with_network_id(network_id: u8, script_hash: Hash<28>) -> Self {
        let network = network_from_network_id(network_id);
        Self::from_script_hash(network, script_hash)
    }

    pub fn from_key_hash(network: Network, pub_key_hash: Hash<28>) -> Self {
        let network_id = network_id_from_network(network);

        let header = REWARD_ADDRESS_PREFIX | (network_id & REWARD_ADDRESS_NETWORK_MASK);
        let mut bytes = Vec::with_capacity(1 + 28);
        bytes.push(header);
        bytes.extend_from_slice(&pub_key_hash.0);

        RewardAccount(Bytes::from(bytes))
    }

    pub fn from_key_hash_with_network_id(network_id: u8, pub_key_hash: Hash<28>) -> Self {
        let network = network_from_network_id(network_id);
        Self::from_key_hash(network, pub_key_hash)
    }

    pub fn as_bytes(&self) -> &Bytes {
        &self.0
    }
}

impl From<Bytes> for RewardAccount {
    fn from(value: Bytes) -> Self {
        RewardAccount(value)
    }
}

impl From<Vec<u8>> for RewardAccount {
    fn from(value: Vec<u8>) -> Self {
        RewardAccount(Bytes::from(value))
    }
}

impl From<RewardAccount> for Bytes {
    fn from(value: RewardAccount) -> Self {
        value.0
    }
}

impl AsRef<[u8]> for RewardAccount {
    fn as_ref(&self) -> &[u8] {
        self.0.as_ref()
    }
}
