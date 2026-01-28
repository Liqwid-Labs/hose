//! High-level transaction builder API

use std::collections::HashSet;
use std::sync::Arc;

use anyhow::Context;
use hydrant::UtxoIndexer;
use hydrant::primitives::TxOutputPointer;
use ogmios_client::OgmiosHttpClient;
use ogmios_client::method::pparams::ProtocolParams;
use pallas::ledger::addresses::Address;
use pallas::ledger::primitives::NetworkId;
use pallas::ledger::primitives::conway::LanguageView;
use tokio::sync::Mutex;

use crate::builder::coin_selection::{get_input_assets, get_input_lovelace};
use crate::primitives::{Certificate, ExUnits, Hash, Input, Output, ScriptKind, TxHash};
use crate::wallet::Wallet;

pub mod coin_selection;
pub mod fee;
pub mod tx;

use coin_selection::{handle_change, select_coins};
use fee::calculate_min_fee;
use tx::{BuiltTransaction, StagingTransaction};

pub struct TxBuilder {
    body: StagingTransaction,
    collateral_address: Option<Address>,
    change_address: Option<Address>,
    script_kinds: HashSet<ScriptKind>,
}

// TODO: redeemers, auxillary data, language view, mint asset, delegation, governance
impl TxBuilder {
    pub fn new(network: NetworkId) -> Self {
        Self {
            body: StagingTransaction::new().network_id(network.into()),
            collateral_address: None,
            change_address: None,
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

    pub fn register_script_stake(
        mut self,
        script_kind: ScriptKind,
        script_bytes: Vec<u8>,
        // NOTE: Right now, redeemers and script execution aren't required by the ledger, but the
        // Conway CDDL mandates them and they'll become necessary after the next hard fork.
        redeemer: Option<Vec<u8>>,
        ex_units: Option<ExUnits>,
        // TODO: we don't really need to pass the deposit, it's a protocol parameter. We should get
        // it from ogmios-client.
        deposit: u64,
    ) -> Self {
        let script_hash = script_kind.hash(&script_bytes);
        self.body = self.body.add_certificate(Certificate::StakeRegistrationScript {
            script_hash,
            deposit,
        });
        if let Some(redeemer) = redeemer {
            // if a redeemer was provided, we attach the script and its ex_units as well
            self.body = self.body.add_cert_redeemer(script_hash, redeemer, ex_units);
            self.body = self.body.script(script_kind, script_bytes);
            self.script_kinds.insert(script_kind);
        }
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

    fn non_collateral_inputs(&self) -> Vec<TxOutputPointer> {
        self.body
            .inputs
            .iter()
            .chain(self.body.reference_inputs.iter())
            .map(Into::into)
            .collect::<Vec<_>>()
    }

    async fn requires_collateral(&self, indexer: Arc<Mutex<UtxoIndexer>>) -> anyhow::Result<bool> {
        // any mints (minting policy) or scripts (inline)
        if !self.body.mint.is_empty() || !self.body.scripts.is_empty() {
            return Ok(true);
        }

        // any input comes from a script or contains a script (validator)
        let input_utxos = {
            let indexer = indexer.lock().await;
            indexer.utxos(&self.non_collateral_inputs())?
        };
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
        indexer: Arc<Mutex<UtxoIndexer>>,
        ogmios: &OgmiosHttpClient,
        pparams: &ProtocolParams,
    ) -> anyhow::Result<BuiltTx> {
        for script_kind in self.script_kinds.iter() {
            if let Some(language_view) = language_view_for_script_kind(script_kind.clone(), pparams)
            {
                self.body = self
                    .body
                    .language_view(script_kind.clone(), language_view.1);
            }
        }

        let change_address = self
            .change_address
            .clone()
            .context("change address not set")?;

        let address_utxos = {
            let indexer = indexer.lock().await;
            indexer.address_utxos(&change_address.to_vec())?
        };

        // 1. balance inputs/outputs with fee
        let (mut fee, mut evaluation) =
            calculate_min_fee(indexer.clone(), ogmios, &self.body, pparams, None).await;
        loop {
            let input_lovelace = get_input_lovelace(indexer.clone(), &self.body).await?;
            let input_assets = get_input_assets(indexer.clone(), &self.body).await?;
            let additional_inputs = select_coins(
                input_lovelace,
                input_assets,
                pparams,
                &address_utxos,
                &self.body,
                fee,
            )
            .await;
            if additional_inputs.is_empty() {
                // No need to add more inputs, but we still need to recalculate the fee
                (fee, evaluation) = calculate_min_fee(
                    indexer.clone(),
                    ogmios,
                    &self.body,
                    pparams,
                    Some(evaluation),
                )
                .await;
                self.body.fee = Some(fee);
                break;
            }
            for input in additional_inputs {
                self.body = self.body.input(input.into());
            }

            (fee, evaluation) = calculate_min_fee(
                indexer.clone(),
                ogmios,
                &self.body,
                pparams,
                Some(evaluation),
            )
            .await;
            self.body = self.body.fee(fee);
        }

        // 2. add change output
        // TODO: minimum output
        if let Some(change_output) = handle_change(
            indexer.clone(),
            self.change_address.as_ref().unwrap(),
            &self.body,
            fee,
        )
        .await?
        {
            self.body = self.body.output(change_output);
        }

        // 3. pick collateral input
        if self.requires_collateral(indexer.clone()).await?
            && self.body.collateral_inputs.is_empty()
        {
            let required_lovelace = ((fee as f64) * pparams.collateral_percentage).ceil() as u64;

            // TODO: support multiple collateral inputs
            let mut collateral_utxos = address_utxos
                .iter()
                .filter(|utxo| utxo.lovelace > required_lovelace)
                .collect::<Vec<_>>();
            collateral_utxos.sort_unstable_by_key(|utxo| utxo.lovelace);
            let collateral_utxo = *collateral_utxos
                .first()
                .context("no utxos large enough for collateral")?;
            let collateral_utxo_pointer: TxOutputPointer = collateral_utxo.into();
            self.body = self.body.collateral_input(collateral_utxo_pointer.into());

            // TODO: collateral output
        }

        // 4. serialize to CBOR
        match self.body.clone().build_conway(Some(evaluation)) {
            Ok(tx) => Ok(BuiltTx::new(self.body, tx)),
            Err(e) => Err(anyhow::anyhow!("Failed to build transaction: {}", e)),
        }
    }
}

pub fn language_view_for_script_kind(
    script_kind: ScriptKind,
    pparams: &ProtocolParams,
) -> Option<LanguageView> {
    match script_kind {
        ScriptKind::Native => None,
        ScriptKind::PlutusV1 => Some(LanguageView(
            1,
            pparams
                .plutus_cost_models
                .plutus_v1
                .as_ref()
                .unwrap()
                .0
                .clone(),
        )),
        ScriptKind::PlutusV2 => Some(LanguageView(
            2,
            pparams
                .plutus_cost_models
                .plutus_v2
                .as_ref()
                .unwrap()
                .0
                .clone(),
        )),
        ScriptKind::PlutusV3 => Some(LanguageView(
            3,
            pparams
                .plutus_cost_models
                .plutus_v3
                .as_ref()
                .unwrap()
                .0
                .clone(),
        )),
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
