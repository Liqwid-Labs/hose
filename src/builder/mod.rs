//! High-level transaction builder API

use hydrant::primitives::TxHash;
use pallas::crypto::hash::Hash;
use pallas::ledger::addresses::Address;
use pallas::ledger::primitives::NetworkId;
use pallas::txbuilder::{BuildConway, BuiltTransaction, StagingTransaction};
pub use pallas::txbuilder::{Input, Output};

pub mod fee;
pub mod selection;

use crate::builder::fee::calculate_min_fee;
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

    pub fn change_address(mut self, address: Address) -> Self {
        self.change_address = Some(address);
        self
    }

    pub async fn build(
        self,
        ogmios: &OgmiosClient,
        pparams: &ProtocolParams,
    ) -> anyhow::Result<BuiltTx> {
        let body = self.body.clone();
        let fee = calculate_min_fee(&ogmios, &body, pparams).await;
        let body = body.fee(fee);
        let change_address = self.change_address.unwrap_or_else(|| {
            // Get rid of clone
            let outputs = &body.clone().outputs.unwrap_or_default();
            let address = outputs.into_iter().last().unwrap().address.clone();
            address.0
        });
        let body = simple_balance_fee(body, change_address, fee)?;
        // TODO: handle change address
        match body.build_conway_raw() {
            Ok(tx) => Ok(BuiltTx::new(self.body, tx, self.network)),
            Err(e) => Err(anyhow::anyhow!("Failed to build transaction: {}", e)),
        }
    }
}

fn simple_balance_fee(
    tx: StagingTransaction,
    change_address: Address,
    fee: u64,
) -> anyhow::Result<StagingTransaction> {
    let outputs = tx.outputs.clone().unwrap_or_default();
    let output_to_pay_fee: Option<usize> =
        outputs
            .iter()
            .enumerate()
            .find_map(|(i, o)| -> Option<usize> {
                if o.address.0 == change_address && o.lovelace > fee {
                    Some(i)
                } else {
                    None
                }
            });

    let tx = if let Some(index) = output_to_pay_fee {
        let output = outputs[index].clone();
        let tx = tx.remove_output(index);
        let tx = tx.output(Output {
            lovelace: output.lovelace - fee,
            ..output
        });
        tx
    } else {
        tx
    };

    Ok(tx)
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
        let cbor = self.tx.tx_bytes.0.clone();
        cbor
    }

    pub fn cbor_hex(&self) -> String {
        let cbor = self.cbor();
        let hex = hex::encode(cbor);
        hex
    }

    pub fn hash(&self) -> anyhow::Result<TxHash> {
        Ok(self.tx.tx_hash.0.into())
    }
}
