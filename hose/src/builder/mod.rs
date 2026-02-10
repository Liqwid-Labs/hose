//! High-level transaction builder API

use std::collections::HashSet;
use std::sync::Arc;

use anyhow::{Context, Result, bail, ensure};
use hydrant::UtxoIndexer;
use intervals_general::Interval;
use ogmios_client::OgmiosHttpClient;
use ogmios_client::method::pparams::ProtocolParams;
use pallas::ledger::addresses::Address;
use pallas::ledger::primitives::conway::LanguageView;
use tokio::sync::Mutex;

use crate::primitives::{DatumOption, Output, ScriptKind, TxHash};
use crate::wallet::Wallet;

mod api;
pub mod coin_selection;
mod collateral;
pub mod fee;
pub mod tx;

use tx::{BuiltTransaction, StagingTransaction};

pub struct TxBuilder {
    body: StagingTransaction,
    collateral_address: Option<Address>,
    change_address: Address,
    change_datum: Option<DatumOption>,
    script_kinds: HashSet<ScriptKind>,
    pub validity_interval: Interval<u64>,
}

// TODO: redeemers, auxillary data, language view, delegation, governance
impl TxBuilder {
    /// 1. Balance inputs/outputs with fee (estimated on first run, actual on future runs)
    /// 2. Evaluate transaction and get the actual fee
    /// 3. Apply fee to the transaction
    /// 4. Check if balanced (true -> continue, false -> back to step 1, max of X tries)
    /// 5. BUILD
    pub async fn build(
        mut self,
        indexer: &Arc<Mutex<UtxoIndexer>>,
        ogmios: &OgmiosHttpClient,
        pparams: &ProtocolParams,
    ) -> Result<BuiltTx> {
        let validity_interval = self.validity_interval;
        self = self.apply_validity_interval(&validity_interval)?;
        // TODO: language view can only be set once per transaction, so this doens't make sense
        for script_kind in self.script_kinds.iter() {
            if let Some(language_view) = language_view_for_script_kind(*script_kind, pparams) {
                self.body = self.body.language_view(*script_kind, language_view.1);
            }
        }
        self.body = self
            .body
            .apply_stake_credential_deposit(pparams.stake_credential_deposit.lovelace);

        let address_utxos = {
            let indexer = indexer.lock().await;
            indexer.address_utxos(&self.change_address.to_vec())?
        };

        // balance inputs/outputs with fee in a loop until stable
        let (mut fee, mut evaluation) =
            TxBuilder::min_fee(&self.body, indexer, ogmios, pparams, None).await?;
        self.body = self.body.fee(fee);

        let mut loop_count = 0;
        const MAX_ITERATIONS: usize = 20;
        loop {
            loop_count += 1;
            ensure!(
                loop_count <= MAX_ITERATIONS,
                "failed to balance transaction fee after {} iterations",
                MAX_ITERATIONS
            );

            for input in self
                .select_coins(indexer, &address_utxos, fee, pparams)
                .await?
            {
                self.body = self.body.input(input.into());
            }

            // Recalculate fee with the change output and collateral input included
            let finalized_body = {
                let mut body = self.body.clone();
                for collateral_input in self
                    .collateral_inputs(indexer, &address_utxos, pparams, fee)
                    .await?
                {
                    body = body.collateral_input(collateral_input);
                }
                // TODO: if change output not present, must burn it in fee. perhaps disallow this?
                let change_output = self
                    .change_output(indexer, fee, pparams)
                    .await?
                    .context("failed to create change output")?;
                body = body.output(change_output);
                body
            };
            let (next_fee, next_evaluation) = TxBuilder::min_fee(
                &finalized_body,
                indexer,
                ogmios,
                pparams,
                Some(evaluation.clone()),
            )
            .await?;

            // Same as the last iteration, fully balanced
            if next_fee == fee {
                self.body = finalized_body;
                break;
            }

            self.body = self.body.fee(next_fee);
            fee = next_fee;
            evaluation = next_evaluation;
        }

        // serialize to CBOR
        let tx = self
            .body
            .clone()
            .build_conway(Some(evaluation))
            .context("failed to build transaction")?;
        Ok(BuiltTx::new(self.body, tx))
    }

    pub fn apply_validity_interval(mut self, validity_interval: &Interval<u64>) -> Result<Self> {
        // Note: Cardano validity interval semantics.
        //
        // Cardano treats the validity interval as a left-half-open interval, i.e. [start, end)
        // Therefore, we need to adjust the start and end slots to account for this.
        //
        match validity_interval {
            Interval::Closed { bound_pair: pair } => {
                self.body = self.body.valid_from_slot(*pair.left());
                self.body = self.body.invalid_from_slot(*pair.right() - 1);
            }
            Interval::UnboundedClosedLeft { left } => {
                // specified: [start, +inf)
                self.body = self.body.valid_from_slot(*left);
            }
            Interval::UnboundedClosedRight { right } => {
                // specified: (-inf, end]
                self.body = self.body.invalid_from_slot(*right - 1);
            }
            Interval::UnboundedOpenLeft { left } => {
                // specified: (start, +inf)
                self.body = self.body.valid_from_slot(*left - 1);
            }
            Interval::UnboundedOpenRight { right } => {
                // specified: (-inf, end)
                self.body = self.body.invalid_from_slot(*right);
            }
            Interval::Unbounded => {
                // specified: (-inf, +inf)
            }
            Interval::Open { bound_pair } => {
                // specified: (start, end)
                self.body = self.body.valid_from_slot(*bound_pair.left() + 1);
                self.body = self.body.invalid_from_slot(*bound_pair.right());
            }
            Interval::LeftHalfOpen { bound_pair } => {
                // specified: (start, end]
                self.body = self.body.valid_from_slot(*bound_pair.left() + 1);
                self.body = self.body.invalid_from_slot(*bound_pair.right() - 1);
            }
            Interval::RightHalfOpen { bound_pair } => {
                // specified: [start, end)
                // no adjustment needed!
                self.body = self.body.valid_from_slot(*bound_pair.left());
                self.body = self.body.invalid_from_slot(*bound_pair.right());
            }
            Interval::Singleton { at } => {
                // specified: [start, end], where start == end
                self.body = self.body.valid_from_slot(*at);
                self.body = self.body.invalid_from_slot(*at + 1);
            }

            Interval::Empty => {
                bail!("Validity interval is empty. This is likely a bug in hose.");
            }
        }
        Ok(self)
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

    pub fn sign(mut self, wallet: &Wallet) -> Result<Self> {
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

    pub fn hash(&self) -> Result<TxHash> {
        Ok(self.tx.hash.0.into())
    }
}

#[cfg(test)]
mod tests {
    use pallas::ledger::addresses::{
        Address as PallasAddress, Network, ShelleyAddress, ShelleyDelegationPart,
        ShelleyPaymentPart,
    };
    use pallas::ledger::primitives::NetworkId;

    use super::TxBuilder;
    use crate::primitives::{Asset, Hash, RedeemerPurpose, ScriptKind};

    fn dummy_address() -> PallasAddress {
        let payment_hash = Hash([1u8; 28]);
        PallasAddress::Shelley(ShelleyAddress::new(
            Network::Testnet,
            ShelleyPaymentPart::Key(payment_hash.into()),
            ShelleyDelegationPart::Null,
        ))
    }

    fn has_mint_redeemer(builder: &TxBuilder, policy: Hash<28>) -> bool {
        builder
            .body
            .redeemers
            .as_ref()
            .map(|redeemers| redeemers.contains_key(&RedeemerPurpose::Mint(policy)))
            .unwrap_or(false)
    }

    #[test]
    fn mint_then_burn_same_asset_removes_redeemer() {
        let policy = Hash([4u8; 28]);
        let builder = TxBuilder::new(NetworkId::Testnet, dummy_address())
            .mint_asset(
                Asset {
                    policy,
                    name: b"NETZERO".to_vec(),
                    quantity: 5,
                },
                ScriptKind::PlutusV3,
                vec![0u8],
            )
            .expect("mint")
            .burn_asset(
                Asset {
                    policy,
                    name: b"NETZERO".to_vec(),
                    quantity: 5,
                },
                ScriptKind::PlutusV3,
                vec![1u8],
            )
            .expect("burn");

        assert!(builder.body.mint.is_empty());
        assert!(!has_mint_redeemer(&builder, policy));
    }
}
