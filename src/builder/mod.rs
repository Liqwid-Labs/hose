//! High-level transaction builder API

use anyhow::Context;
use hydrant::UtxoIndexer;
use hydrant::primitives::TxOutputPointer;
use pallas::ledger::addresses::Address;
use pallas::ledger::primitives::NetworkId;

use crate::ogmios::OgmiosClient;
use crate::ogmios::pparams::ProtocolParams;
use crate::primitives::{Hash, Input, Output, ScriptKind, TxHash};
use crate::wallet::Wallet;

pub mod coin_selection;
pub mod fee;
pub mod tx;

use coin_selection::{handle_change, select_coins};
use fee::calculate_min_fee;
use tx::{BuiltTransaction, StagingTransaction};

pub struct TxBuilder {
    body: StagingTransaction,
    collateral_output: bool,
    collateral_address: Option<Address>,
    change_address: Option<Address>,
}

// TODO: redeemers, auxillary data, language view, mint asset, delegation, governance
impl TxBuilder {
    pub fn new(network: NetworkId) -> Self {
        Self {
            body: StagingTransaction::new().network_id(network.into()),
            collateral_output: false,
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

    pub fn use_collateral_output(mut self) -> Self {
        self.collateral_output = true;
        self
    }

    fn non_collateral_inputs(&self) -> Vec<TxOutputPointer> {
        self.body
            .inputs
            .iter()
            .chain(self.body.reference_inputs.iter())
            .map(Into::into)
            .collect::<Vec<_>>()
    }

    fn requires_collateral(&self, indexer: &UtxoIndexer) -> anyhow::Result<bool> {
        // any mints (minting policy) or scripts (inline)
        if !self.body.mint.is_empty() || !self.body.scripts.is_empty() {
            return Ok(true);
        }

        // any input comes from a script or contains a script (validator)
        let input_utxos = indexer.utxos(&self.non_collateral_inputs())?;
        if input_utxos.iter().any(|input| {
            Address::from_bytes(&input.address).unwrap().has_script() || input.script.is_some()
        }) {
            return Ok(true);
        }

        Ok(false)
    }

    /// 1. Balance inputs/outputs with fee (estimated on first run, actual on future runs)
    /// 2. Evaluate transaction and get the actual fee
    /// 3. Apply fee to the transaction
    /// 4. Check if balanced (true -> continue, false -> back to step 1, max of X tries)
    /// 5. BUILD
    pub async fn build(
        mut self,
        indexer: &UtxoIndexer,
        ogmios: &OgmiosClient,
        pparams: &ProtocolParams,
    ) -> anyhow::Result<BuiltTx> {
        let change_address = self
            .change_address
            .clone()
            .context("change address not set")?;

        let address_utxos = indexer.address_utxos(&change_address.to_vec())?;

        // 1. balance inputs/outputs with fee
        let mut fee = calculate_min_fee(indexer, ogmios, &self.body, pparams).await;
        loop {
            let additional_inputs = select_coins(pparams, &address_utxos, &self.body, fee).await;
            if additional_inputs.is_empty() {
                break;
            }
            for input in additional_inputs {
                self.body = self.body.input(input.into());
            }

            fee = calculate_min_fee(indexer, ogmios, &self.body, pparams).await;
            self.body = self.body.fee(fee);
        }

        // 2. add change output
        // TODO: minimum output
        if let Some(change_output) = handle_change(
            indexer,
            self.change_address.as_ref().unwrap(),
            &self.body,
            fee,
        )? {
            self.body = self.body.output(change_output);
        }

        // 3. pick collateral input
        if self.requires_collateral(indexer)? && self.body.collateral_inputs.is_empty() {
            let required_lovelace = ((fee as f64) * pparams.collateral_percentage).ceil() as u64;

            // TODO: support multiple collateral inputs
            let mut collateral_utxos = address_utxos
                .iter()
                .filter(|utxo| {
                    utxo.lovelace > required_lovelace
                        && utxo.script.is_none()
                        && utxo.datum_hash.is_none()
                })
                .collect::<Vec<_>>();
            collateral_utxos.sort_unstable_by_key(|utxo| utxo.lovelace);
            let collateral_utxo = *collateral_utxos
                .first()
                .context("no utxos large enough for collateral")?;
            let collateral_utxo_pointer: TxOutputPointer = collateral_utxo.into();
            self.body = self.body.collateral_input(collateral_utxo_pointer.into());

            if self.collateral_output {
                // assume a change output of maximum 500 bytes
                // TODO: technically we should use the actual size of the change output
                let excess_lovelace = collateral_utxo.lovelace - required_lovelace;
                let min_lovelace = pparams.min_utxo_deposit_coefficient * 500;
                if excess_lovelace > min_lovelace {
                    let change_output = Output::new(change_address.clone(), excess_lovelace)
                        .add_assets(collateral_utxo.assets.clone());
                    self.body = self
                        .body
                        .output(change_output.expect("failed to create change output"));
                }
            }
        }
        // TODO: update fee

        // 4. serialize to CBOR
        match self.body.clone().build_conway() {
            Ok(tx) => Ok(BuiltTx::new(self.body, tx)),
            Err(e) => Err(anyhow::anyhow!("Failed to build transaction: {}", e)),
        }
    }
}

pub struct BuiltTx {
    staging: StagingTransaction,
    tx: BuiltTransaction,
}

impl BuiltTx {
    pub fn new(staging: StagingTransaction, tx: BuiltTransaction) -> Self {
        Self { staging, tx }
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
        self.tx.bytes.clone()
    }

    pub fn cbor_hex(&self) -> String {
        hex::encode(self.cbor())
    }

    pub fn hash(&self) -> anyhow::Result<TxHash> {
        Ok(self.tx.hash.0.into())
    }
}
