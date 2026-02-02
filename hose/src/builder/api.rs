//! Public API for building transactions

use std::collections::HashSet;

use pallas::ledger::addresses::Address;
use pallas::ledger::primitives::NetworkId;

use super::TxBuilder;
use super::tx::StagingTransaction;
use crate::primitives::{
    Certificate, DatumOption, ExUnits, Hash, Input, Output, RewardAccount, ScriptKind,
};

impl TxBuilder {
    pub fn new(network: NetworkId, change_address: Address) -> Self {
        Self {
            body: StagingTransaction::new().network_id(network.into()),
            collateral_address: None,
            change_address,
            change_datum: None,
            script_kinds: HashSet::new(),
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

    // TODO: Use a `Script` type
    pub fn add_script_input(
        mut self,
        input: Input,
        plutus_data: Vec<u8>,
        ex_units: Option<ExUnits>,
        script_kind: ScriptKind,
    ) -> Self {
        self.body = self.body.input(input.clone());
        self.body = self.body.add_spend_redeemer(input, plutus_data, ex_units);
        self.script_kinds.insert(script_kind);
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

    /// Register a script's reward account and lock some lovelace as a deposit, so it can be
    /// withdrawn from in later transactions.
    ///
    /// Note that as of Jan 2026, script's aren't evaluated when they're registered (and so a
    /// redeemer is optional), but they will be in the future.
    ///
    /// The deposit amount can be retrieved from the protocol parameters.
    pub fn register_script_stake(
        mut self,
        script_hash: Hash<28>,
        script_kind: ScriptKind,
        // NOTE: Right now, redeemers and script execution aren't required by the ledger, but the
        // Conway CDDL mandates them and they'll become necessary after the next hard fork.
        redeemer: Option<Vec<u8>>,
        ex_units: Option<ExUnits>,
    ) -> Self {
        self.body = self
            .body
            .add_certificate(Certificate::StakeRegistrationScript {
                script_hash,
                deposit: None,
            });
        if let Some(redeemer) = redeemer {
            // if a redeemer was provided, we attach the script and its ex_units as well
            self.body = self.body.add_cert_redeemer(script_hash, redeemer, ex_units);
            self.script_kinds.insert(script_kind);
        }
        self
    }

    /// Deregister a script's reward account and refund the deposit.
    ///
    /// Note that, unlike registration, deregistration always requires a redeemer.
    pub fn deregister_script_stake(
        mut self,
        script_hash: Hash<28>,
        script_kind: ScriptKind,
        redeemer: Vec<u8>,
        ex_units: Option<ExUnits>,
    ) -> Self {
        self.body = self
            .body
            .add_certificate(Certificate::StakeDeregistrationScript {
                script_hash,
                deposit: None,
            });
        self.body = self.body.add_cert_redeemer(script_hash, redeemer, ex_units);
        self.script_kinds.insert(script_kind);
        self
    }

    /// Withdraw rewards from a script's reward account. Note that the account must have been
    /// registered beforehand with `register_script_stake`.
    ///
    /// FIXME: according to the ledger rules, it's only possible to withdraw the entire amount of
    /// rewards accrued in the account. We should probably query for this balance and fill the
    /// amount automatically.
    pub fn withdraw_from_script(
        mut self,
        script_hash: Hash<28>,
        script_kind: ScriptKind,
        amount: u64,
        redeemer: Vec<u8>,
        ex_units: Option<ExUnits>,
    ) -> Self {
        let network_id = self.body.network_id.unwrap_or(0);
        let reward_account =
            RewardAccount::from_script_hash_with_network_id(network_id, script_hash);
        self.body = self.body.withdrawal(reward_account.clone(), amount);
        self.body = self
            .body
            .add_reward_redeemer(reward_account, redeemer, ex_units);
        self.script_kinds.insert(script_kind);
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
    pub fn add_script(mut self, language: ScriptKind, bytes: Vec<u8>) -> Self {
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

    pub fn change_datum(mut self, datum: DatumOption) -> Self {
        self.change_datum = Some(datum);
        self
    }
}
