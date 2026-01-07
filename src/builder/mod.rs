//! High-level transaction builder API

use anyhow::Context;
use hydrant::primitives::{TxHash, TxOutputPointer};
use pallas::crypto::hash::Hash;
use pallas::ledger::addresses::Address;
use pallas::ledger::primitives::NetworkId;
use pallas::network::miniprotocols::localstate::queries_v16::NextEpochChange;

use crate::builder::coin_selection::handle_change;
use crate::builder::transaction::build_conway::BuildConway as _;
use crate::builder::transaction::model::{BuiltTransaction, StagingTransaction};
pub use crate::builder::transaction::model::{Input, Output};

pub mod coin_selection;
pub mod fee;
pub mod transaction;

use coin_selection::{get_input_coins, get_output_coins, select_coins};
use fee::calculate_min_fee;

use crate::builder::transaction::model::ScriptKind;
use crate::ogmios::OgmiosClient;
use crate::ogmios::pparams::ProtocolParams;
use crate::wallet::Wallet;

pub struct TxBuilder {
    body: StagingTransaction,
    network: NetworkId,
    collateral_address: Option<Address>,
    change_address: Option<Address>,
}

// TODO: redeemers, auxillary data, language view, mint asset, delegation, governance
impl TxBuilder {
    pub fn new(network: NetworkId) -> Self {
        Self {
            body: StagingTransaction::new(),
            network,
            collateral_address: None,
            change_address: None,
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

    pub fn change_address(mut self, address: Address) -> Self {
        self.change_address = Some(address);
        self
    }

    fn all_inputs(&self) -> Vec<TxOutputPointer> {
        self.body
            .inputs
            .iter()
            .flatten()
            .chain(self.body.reference_inputs.iter().flatten())
            .chain(self.body.collateral_inputs.iter().flatten())
            .cloned()
            .map(Into::into)
            .collect::<Vec<_>>()
    }

    fn requires_collateral(&self) -> bool {
        // any input comes from a script
        // any input (including reference) contains a script
        // any mints
        // plutus script in witnesses
        true
    }

    /// 1. Balance inputs/outputs with fee (estimated on first run, actual on future runs)
    /// 2. Evaluate transaction and get the actual fee
    /// 3. Apply fee to the transaction
    /// 4. Check if balanced (true -> continue, false -> back to step 1, max of X tries)
    /// 5. BUILD
    pub async fn build(
        mut self,
        ogmios: &OgmiosClient,
        pparams: &ProtocolParams,
    ) -> anyhow::Result<BuiltTx> {
        let mut body = self.body.clone();

        let change_address = self
            .change_address
            .clone()
            .context("change address not set")?
            .to_bech32()
            .context("invalid change address")?;

        // TODO: pick collateral input
        // TODO: minimum output
        let address_utxos = ogmios
            .utxos_by_addresses(&[change_address.as_str()])
            .await?;

        let mut fee = calculate_min_fee(ogmios, &self.body, pparams).await;
        loop {
            let input_coins = get_input_coins(ogmios, &self.all_inputs()).await;
            let output_coins = get_output_coins(&self.body).await;

            let additional_inputs = select_coins(
                &address_utxos,
                &self.all_inputs(),
                &input_coins,
                &output_coins,
                fee,
            )
            .await;
            if additional_inputs.is_empty() {
                break;
            }
            for input in additional_inputs {
                self.body = self.body.input(input.into());
            }

            fee = calculate_min_fee(ogmios, &self.body, pparams).await;
            self.body.fee = Some(fee);
        }

        let input_coins = get_input_coins(ogmios, &self.all_inputs()).await;
        let output_coins = get_output_coins(&self.body).await;

        let change = handle_change(
            self.change_address.as_ref().unwrap(),
            &input_coins,
            &output_coins,
            fee,
        );
        for output in change {
            self.body = self.body.output(output);
        }

        // TODO: handle change address
        match self.body.clone().build_conway_raw() {
            Ok(tx) => Ok(BuiltTx::new(self.body, tx, self.network)),
            Err(e) => Err(anyhow::anyhow!("Failed to build transaction: {}", e)),
        }
    }
}

pub struct BuiltTx {
    staging: StagingTransaction,
    tx: BuiltTransaction,
    network: NetworkId,
}

impl BuiltTx {
    pub fn new(staging: StagingTransaction, tx: BuiltTransaction, network: NetworkId) -> Self {
        Self {
            staging,
            tx,
            network,
        }
    }

    pub fn body(&self) -> &StagingTransaction {
        &self.staging
    }

    pub fn sign(mut self, wallet: &Wallet) -> anyhow::Result<Self> {
        let tx = wallet.sign(&self.tx)?;
        self.tx = tx;
        Ok(self)
    }

    pub fn cbor(&self) -> Vec<u8> {
        self.tx.tx_bytes.0.clone()
    }

    pub fn cbor_hex(&self) -> String {
        hex::encode(self.cbor())
    }

    pub fn hash(&self) -> anyhow::Result<TxHash> {
        Ok(self.tx.tx_hash.0.into())
    }
}
