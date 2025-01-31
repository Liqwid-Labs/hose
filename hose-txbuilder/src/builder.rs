use std::collections::HashMap;

use hose_primitives::{Asset, AssetKey, MultiEraProtocolParameters, NetworkId, Output};
use pallas::{
    crypto::hash::Hash,
    ledger::addresses::{ Address, PaymentKeyHash },
    txbuilder::{ BuildConway, BuiltTransaction, Output, StagingTransaction, TxBuilderError }
};

use crate::{wallet::Wallet};

pub fn address_payment_key_hash(address: &Address) -> PaymentKeyHash {
    match address {
        Address::Shelley(x) => *x.payment().as_hash(),
        Address::Stake(x) => *x.payload().as_hash(),
        _ => panic!("not a payment address"),
    }
}

pub struct TransactionBuilder {
    pub network: NetworkId,
    pub tx: StagingTransaction,

    inputs: Vec<Output>,
    outputs: Vec<Output>,
    minted_assets: Vec<(AssetKey, i64)>,

    change_address: Address,
    change_output: Option<Output>,
}

impl TransactionBuilder {
    pub fn builder(network: NetworkId, change_address: Address) -> Self {
        Self {
            network,
            tx: StagingTransaction::new().change_address(change_address.clone()),
            inputs: Vec::new(),
            outputs: Vec::new(),
            minted_assets: Vec::new(),
            change_address,
            change_output: None,
        }
    }

    pub fn sign_address(mut self, address: &Address) -> Self {
        // TODO: Ignore duplicates?
        self.tx = self.tx.disclosed_signer(address_payment_key_hash(address));
        self
    }

    pub fn input(mut self, input: Output) -> Self {
        self.inputs.push(input.clone());
        self.tx = self.tx.input(input.into());
        self
    }
    pub fn reference_input(mut self, input: Output) -> Self {
        self.tx = self.tx.reference_input(input.into());
        self
    }

    pub fn mint_asset(
        mut self,
        policy: Hash<28>,
        name: Vec<u8>,
        amount: i64,
    ) -> Result<Self, TxBuilderError> {
        let asset = Asset::new(policy, name.clone(), amount.abs() as u64);
        self.minted_assets.push((asset.key.clone(), amount));
        self.tx = self.tx.mint_asset(policy, name, amount)?;
        Ok(self)
    }

    pub fn output(mut self, output: Output) -> Self {
        self.outputs.push(output.clone());
        self.tx = self.tx.output(output.into());
        self
    }

    pub fn datum(mut self, datum: Vec<u8>) -> Self {
        self.tx = self.tx.datum(datum);
        self
    }

    pub fn valid_from_slot(mut self, slot: u64) -> Self {
        self.tx = self.tx.valid_from_slot(slot);
        self
    }
    pub fn invalid_from_slot(mut self, slot: u64) -> Self {
        self.tx = self.tx.invalid_from_slot(slot);
        self
    }

    fn calculate_size_fee(&self, params: &MultiEraProtocolParameters) -> Result<u64, TxBuilderError> {
        // TODO: is it sufficient to generate a random key here? And is it performant enough?
        // TODO: we should sign with as many keys as designated signerss
        let signed_tx = self.tx.clone().build_conway_raw()?.sign(Wallet::generate().into())?;
        Ok(fee_for_size(signed_tx.tx_bytes.0.len(), params))
    }

    async fn calculate_script_fee<T>(&self, params: &MultiEraProtocolParameters, client: &mut T) -> Result<u64, TxBuilderError> 
    where T: hose_submission::EvaluateTx {
        // TODO: is it sufficient to generate a random key here? And is it performant enough?
        let signed_tx = self.tx.clone().build_conway_raw()?.sign(Wallet::generate().into())?;

        let (mem_price, cpu_price) = match params {
            MultiEraProtocolParameters::Conway(params) => (params.execution_costs.mem_price, params.execution_costs.step_price),
            _ => todo!("Implement support for non-conway protocol parameters in fee computation"),
        };
        let evals = client.evaluate_tx(&signed_tx.tx_bytes.0).await?;
        let script_cost = evals.iter().map(|e|
            ( e.memory_budget * mem_price.numerator ) / mem_price.denominator
            + ( e.cpu_budget * cpu_price.numerator ) / cpu_price.denominator)
            .sum::<u64>() as u64;

        Ok(script_cost)
    }


    fn input_size(&self, input: &Output) -> usize {
        let tx = self.tx.clone();
        let size_before = tx.clone().build_conway_raw().unwrap();
        let size_after = tx.input(input.clone().into()).build_conway_raw().unwrap();
        size_after.tx_bytes.0.len() - size_before.tx_bytes.0.len()
    }

    fn output_size(&self, output: &Output) -> usize {
        let tx = self.tx.clone();
        let size_before = tx.clone().build_conway_raw().unwrap();
        let size_after = tx.output(output.clone().into()).build_conway_raw().unwrap();
        size_after.tx_bytes.0.len() - size_before.tx_bytes.0.len()
    }

    /// The Cardano chain requires that each UTxO have a minimum ADA value, such that the chain
    /// doesn't balloon with many small UTxOs. This function calculates the minimum ADA value
    /// for a given output.
    fn min_lovelace_for_output(&self, output: &Output, params: &MultiEraProtocolParameters) -> Result<u64, TxBuilderError> {
        let lovelace_per_utxo_byte = match params {
            MultiEraProtocolParameters::Conway(params) => params.ada_per_utxo_byte,
            _ => todo!("Implement support for non-conway protocol parameters in fee computation"),
        };
        let output_size = self.output_size(output);
        Ok(output_size as u64 * lovelace_per_utxo_byte)
    }

    fn is_balanced(&self, fee: u64) -> bool {
        let lovelace_diff_with_change = self.lovelace_diff(fee, true);
        let asset_diff_with_change = self.asset_diff(true);

        lovelace_diff_with_change == 0 && asset_diff_with_change.is_empty()
    }

    fn attempt_balance(mut self, params: &MultiEraProtocolParameters, coin_selection_utxos: &[Output], fee: u64) -> Result<bool, TxBuilderError> {
        let mut lovelace_diff = self.lovelace_diff(fee, false);
        let mut asset_diff = self.asset_diff(false);

        let mut selection_utxos = coin_selection_utxos.clone().to_vec();
        // Remove inputs from available UTxOs
        selection_utxos.retain(|utxo| {
            !self.inputs.contains(&utxo)
        });
        // Sort by largest first
        selection_utxos.sort_by(|a, b| a.lovelace.cmp(&b.lovelace));
        if selection_utxos.is_empty() {
            return Err(TxBuilderError::NoUtxosToSpend);
        }

        // Add inputs until we have enough lovelace and assets to cover outputs and fee
        let mut balanced = false;
        for utxo in selection_utxos {
            if lovelace_diff >= 0
                && asset_diff.iter().all(|(_, diff)| *diff >= 0)
                // UTxOs have a minimum size, so make sure we have enough to cover it
                && self.min_lovelace_for_output(self.change_output(fee), params)? as i64 <= lovelace_diff {
                // TODO: ugly, refactor
                balanced = true;
                break;
            }

            if lovelace_diff < 0 || utxo.assets.iter().any(|(key, amount)| asset_diff.contains_key(&key)) {
                self = self.input(utxo.clone());
                lovelace_diff = self.lovelace_diff(fee, false);
                asset_diff = self.asset_diff(false);
            }
        }

        // Not enough lovelace and assets to cover outputs and fee
        if !balanced {
            return Err(TxBuilderError::InsufficientFunds);
        }

        // Build change output
        self.change_output = Some(self.change_output(fee));

         Ok(false)
    }

    fn lovelace_diff(&self, fee: u64, include_change: bool) -> i64 {
        let mut outputs = self.outputs.clone();
        if include_change {
            if let Some(change_output) = &self.change_output {
                outputs.push(change_output.clone());
            }
        }

        return self.inputs.iter().map(|i| i.lovelace).sum::<u64>() as i64
            - outputs.iter().map(|o| o.lovelace).sum::<u64>() as i64
            - fee as i64
    }

    fn asset_diff(&self, include_change: bool) -> HashMap<AssetKey, i64> {
        let outputs = self.outputs.clone();
        if include_change {
            if let Some(change_output) = &self.change_output {
                outputs.push(change_output.clone());
            }
        }
        
        let mut asset_diff: HashMap<AssetKey, i64> = HashMap::new();

        let input_assets = self.inputs.iter().flat_map(|input| input.assets.iter().map(|(k, v)| (k.clone(), *v as i64)));
        let minted_assets = self.minted_assets.iter().map(|a| (a.key.clone(), a.amount));
        for (key, amount) in input_assets.chain(minted_assets) {
            if let Some(diff) = asset_diff.get_mut(&key) {
                *diff -= amount;
            } else {
                asset_diff.insert(key.clone(), amount);
            }
        }
    
        for (key, amount) in outputs.iter().flat_map(|output| output.assets.iter()) {
            if let Some(diff) = asset_diff.get_mut(&key) {
                *diff += *amount as i64;
            } else {
                asset_diff.insert(key.clone(), *amount as i64);
            }
        }
        asset_diff
    }

    fn change_output(&self, fee: u64) -> Output {
        let mut change_output = self.change_output.unwrap_or(Output::default_from_address(&self.change_address)).clone();
        change_output.lovelace = self.lovelace_diff(fee, false) as u64;
        change_output.assets = self.asset_diff(false).iter().map(|(key, diff)| Asset { key: key.clone(), amount: *diff as u64 }).collect();
        change_output
    }

    pub async fn build<T>(mut self, coin_selection_utxos: Vec<Output>, client: &mut T) -> Result<BuiltTransaction, TxBuilderError>
    where T: hose_submission::EvaluateTx
    {
        // 1. Calculate fee using existing inputs and outputs, but without script evaluation
        // 2. Gather all assets in the outputs and inputs
        //   - Output - (Input + Minted) where negative values indicate oversupply, and vice versa
        // 3. Gather available UTxOs
        //   - Exclude any that are already in the inputs
        //   - Exclude any with reference scripts (possible from Ogmios? why does Lucid do this?)
        // 4. Select UTxOs starting from the largest to smallest (LargestFirst strategy)
        //    for the assets
        //   - For each UTxO, check if it includes an asset we need to balance the Tx.
        //     If so, add it and subtract from the list of required assets
        //   - Check if all of the assets and lovelace are covered, and that the change output
        //     meets the minimum ADA requirement
        // 5. Select UTxOs starting from the largest to smallest (LargestFirst strategy)
        //    for the required ADA (output ada + fee + minimum ADA for change output - input ada)
        // 6. Repeat with new fee calculation (including scripts after first run) until balanced
        //    and fee paid
        let params = self.network.parameters();

        let fee = self.calculate_size_fee(&params)?;
        self.attempt_balance(&params, &coin_selection_utxos, fee)

        let mut fee = self.calculate_size_fee(&params)? + self.calculate_script_fee(&params, client).await?;
        // TODO: limit iteration count
        while !self.is_balanced(fee) {
            self.attempt_balance(&params, &coin_selection_utxos, fee)?;
            fee = self.calculate_size_fee(&params)? + self.calculate_script_fee(&params, client).await?;
        }
        self.tx.output(self.change_output(fee).into());

        Ok(self.tx.build_conway_raw()?.into())
    }
}

fn fee_for_size(size: usize, params: &MultiEraProtocolParameters) -> u64 {
    let (coefficient, constant) = match params {
        MultiEraProtocolParameters::Conway(params) => (params.minfee_a as u64, params.minfee_b as u64),
        _ => todo!("Implement support for non-conway protocol parameters in fee computation"),
    };
    coefficient * (size as u64) + constant
}
