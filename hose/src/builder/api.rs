//! Public API for building transactions

use std::collections::HashSet;

use hydrant::primitives::{Asset, AssetId};
use pallas::ledger::addresses::Address;
use pallas::ledger::primitives::NetworkId;

use super::TxBuilder;
use super::tx::StagingTransaction;
use crate::builder::tx::TxBuilderError;
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
            fee_padding: 0,
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
        script_kind: ScriptKind,
    ) -> Self {
        self.body = self.body.input(input.clone());
        self.body = self.body.add_spend_redeemer(input, plutus_data, None);
        self.script_kinds.insert(script_kind);
        self
    }

    pub fn mint_asset(
        self,
        asset: Asset,
        policy_script_kind: ScriptKind,
        redeemer: Vec<u8>,
    ) -> Result<Self, TxBuilderError> {
        if asset.quantity == 0 {
            return Ok(self);
        }
        let amount =
            i64::try_from(asset.quantity).map_err(|_| TxBuilderError::InvalidMintAmount)?;
        self.mint_or_burn_asset(asset.into(), policy_script_kind, amount, redeemer)
    }

    pub fn burn_asset(
        self,
        asset: Asset,
        policy_script_kind: ScriptKind,
        redeemer: Vec<u8>,
    ) -> Result<Self, TxBuilderError> {
        if asset.quantity == 0 {
            return Ok(self);
        }
        let amount =
            -i64::try_from(asset.quantity).map_err(|_| TxBuilderError::InvalidMintAmount)?;
        self.mint_or_burn_asset(asset.into(), policy_script_kind, amount, redeemer)
    }

    fn mint_or_burn_asset(
        mut self,
        asset: AssetId,
        policy_script_kind: ScriptKind,
        amount: i64,
        redeemer: Vec<u8>,
    ) -> Result<Self, TxBuilderError> {
        let asset_id = asset.clone();
        self.body = self.body.mint_asset(asset.policy, asset.name, amount)?;
        if let Some(quantity) = self.body.mint.get(&asset_id).copied() {
            if quantity == 0 {
                self.body.mint.remove(&asset_id);
            }
        }

        let has_policy_mint = self
            .body
            .mint
            .iter()
            .any(|(id, quantity)| id.policy == asset_id.policy && *quantity != 0);
        if has_policy_mint {
            self.body = self.body.add_mint_redeemer(asset_id.policy, redeemer, None);
        } else {
            self.body = self.body.remove_mint_redeemer(asset_id.policy);
        }

        self.script_kinds.insert(policy_script_kind);
        Ok(self)
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
    ) -> Self {
        self.body = self
            .body
            .add_certificate(Certificate::StakeRegistrationScript {
                script_hash,
                deposit: None,
            });
        if let Some(redeemer) = redeemer {
            // if a redeemer was provided, we attach the script and its ex_units as well
            self.body = self.body.add_cert_redeemer(script_hash, redeemer, None);
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
    ) -> Self {
        self.body = self
            .body
            .add_certificate(Certificate::StakeDeregistrationScript {
                script_hash,
                deposit: None,
            });
        self.body = self.body.add_cert_redeemer(script_hash, redeemer, None);
        self.script_kinds.insert(script_kind);
        self
    }

    /// Delegate a script's stake to a stake pool.
    pub fn delegate_script_stake(
        mut self,
        script_hash: Hash<28>,
        pool_id: Hash<28>,
        script_kind: ScriptKind,
        redeemer: Option<Vec<u8>>,
        ex_units: Option<ExUnits>,
    ) -> Self {
        self.body = self
            .body
            .add_certificate(Certificate::StakeDelegationScript {
                script_hash,
                pool_id,
            });

        if let Some(redeemer) = redeemer {
            self.body = self.body.add_cert_redeemer(script_hash, redeemer, ex_units);
            self.script_kinds.insert(script_kind);
        }
        self
    }

    /// Register a key's reward account and lock some lovelace as a deposit.
    pub fn register_stake(mut self, pub_key_hash: Hash<28>) -> Self {
        self.body = self.body.add_certificate(Certificate::StakeRegistration {
            pub_key_hash,
            deposit: None,
        });
        self
    }

    /// Deregister a key's reward account and refund the deposit.
    pub fn deregister_stake(mut self, pub_key_hash: Hash<28>) -> Self {
        self.body = self.body.add_certificate(Certificate::StakeDeregistration {
            pub_key_hash,
            deposit: None,
        });
        self
    }

    /// Delegate a key's stake to a stake pool.
    pub fn delegate_stake(mut self, pub_key_hash: Hash<28>, pool_id: Hash<28>) -> Self {
        self.body = self.body.add_certificate(Certificate::StakeDelegation {
            pub_key_hash,
            pool_id,
        });
        self
    }

    /// Withdraw rewards from a key's reward account.
    ///
    /// The account must have been registered beforehand.
    pub fn withdraw_rewards(mut self, pub_key_hash: Hash<28>, amount: u64) -> Self {
        let network_id = self.body.network_id.unwrap_or(0);
        let reward_account = RewardAccount::from_key_hash_with_network_id(network_id, pub_key_hash);
        self.body = self.body.withdrawal(reward_account, amount);
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
    ) -> Self {
        let network_id = self.body.network_id.unwrap_or(0);
        let reward_account =
            RewardAccount::from_script_hash_with_network_id(network_id, script_hash);
        self.body = self.body.withdrawal(reward_account.clone(), amount);
        self.body = self
            .body
            .add_reward_redeemer(reward_account, redeemer, None);
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

    // Used to avoid fee oscillation/underestimation in devnet admin-upgrade flows.
    /// Adds a fixed lovelace buffer to the computed minimum fee.
    pub fn fee_padding(mut self, padding: u64) -> Self {
        self.fee_padding = padding;
        self
    }

    pub fn change_datum(mut self, datum: DatumOption) -> Self {
        self.change_datum = Some(datum);
        self
    }
}
