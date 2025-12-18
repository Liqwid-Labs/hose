//! High-level transaction builder API

use pallas::crypto::hash::Hash;
use pallas::ledger::addresses::Address;
use pallas::ledger::primitives::NetworkId;
use pallas::txbuilder::StagingTransaction;
pub use pallas::txbuilder::{Input, Output};

pub mod fee;
pub mod selection;

use crate::ogmios::OgmiosClient;

pub struct TxBuilder {
    body: StagingTransaction,
    network: NetworkId,
    collateral_address: Option<Address>,
}

// TODO: redeemers, auxillary data, language view, mint asset, delegation, governance
impl TxBuilder {
    pub fn new(network: NetworkId) -> Self {
        Self {
            body: StagingTransaction::new(),
            network,
            collateral_address: None,
        }
    }

    /// Manually add an input to the transaction for consumption.
    ///
    /// Note that when no inputs are specified, the balancing algorithm will automatically select
    /// inputs from change address.
    pub fn add_input(mut self, input: Input) -> Self {
        self.body = self.body.input(input);
        self
    }

    /// Manually add a collateral input to the transaction for consumption by the chain, if our
    /// scripts fail to execute after submission. The input must contain only ADA (no assets).
    ///
    /// Note that when no collateral inputs are specified, the balancing algorithm will automatically
    /// select inputs from change address.
    pub fn add_collateral_input(mut self, input: Input) -> Self {
        self.body = self.body.collateral_input(input);
        self
    }

    /// Add a read-only input to the transaction which won't be consumed, but can be inspected by
    /// scripts. Perfect for oracles, shared state, etc.
    pub fn add_reference_input(mut self, input: Input) -> Self {
        self.body = self.body.reference_input(input);
        self
    }

    /// Add an output to the transaction, optionally including assets, datum and/or script.
    pub fn add_output(mut self, output: Output) -> Self {
        self.body = self.body.output(output);
        self
    }

    /// Sets the address to which the collateral change will be sent when script validation fails.
    ///
    /// Note that by default, no collateral output is added to save on transaction size.
    pub fn collateral_output_address(mut self, address: Address) -> Self {
        self.collateral_address = Some(address);
        self
    }

    pub fn valid_from(self, _timestamp: u64) -> Self {
        todo!();
    }
    pub fn valid_to(self, _timestamp: u64) -> Self {
        todo!();
    }

    // Witnesses
    pub fn add_script(mut self, language: pallas::txbuilder::ScriptKind, bytes: Vec<u8>) -> Self {
        self.body = self.body.script(language, bytes);
        self
    }
    pub fn add_datum(mut self, datum: Vec<u8>) -> Self {
        self.body = self.body.datum(datum);
        self
    }
    pub fn add_signer(mut self, pub_key_hash: Hash<28>) -> Self {
        self.body = self.body.disclosed_signer(pub_key_hash);
        self
    }
}
