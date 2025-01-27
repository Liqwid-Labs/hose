use hose_primitives::{MultiEraProtocolParameters, NetworkId, UTxO};
use pallas::{
    crypto::hash::Hash,
    ledger::addresses::{ Address, PaymentKeyHash },
    txbuilder::{ BuildConway, BuiltTransaction, Input, Output, StagingTransaction, TxBuilderError }
};

use crate::{transaction::Transaction, wallet::Wallet};

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
    inputs: Vec<UTxO>,
    coin_selection_address: Address,
    coin_selection_utxos: Option<Vec<UTxO>>,
}

impl TransactionBuilder {
    pub fn builder(network: NetworkId, address: Address) -> Self {
        Self {
            network,
            tx: StagingTransaction::new().disclosed_signer(address_payment_key_hash(&address)),
            inputs: Vec::new(),
            coin_selection_address: address,
            coin_selection_utxos: None
        }
    }

    pub fn from_utxos(mut self, utxos: &[UTxO]) -> Self {
        self.coin_selection_utxos = Some(utxos.to_vec());
        self
    }

    // TODO: make coin selection address optional
    /// Sets the address to be used for coin selection and signing
    // pub fn from_address(mut self, address: &Address) -> Self {
    //     self.coin_selection_address = Some(address.clone());
    //     self.tx = self.tx.disclosed_signer(address_payment_key_hash(address));
    //     self
    // }

    pub fn change_address(mut self, address: &Address) -> Self {
        self.tx = self.tx.change_address(address.clone());
        self
    }

    pub fn input(mut self, input: UTxO) -> Self {
        self.tx = self.tx.input(input.into());
        self
    }
    pub fn reference_input(mut self, input: UTxO) -> Self {
        self.tx = self.tx.reference_input(input.into());
        self
    }

    pub fn output(mut self, output: Output) -> Self {
        self.tx = self.tx.output(output.into());
        self
    }

    pub fn datum(mut self, datum: Vec<u8>) -> Self {
        self.tx = self.tx.datum(datum);
        self
    }

    pub fn mint_asset(
        mut self,
        policy: Hash<28>,
        name: Vec<u8>,
        amount: i64,
    ) -> Result<Self, TxBuilderError> {
        self.tx = self.tx.mint_asset(policy, name, amount)?;
        Ok(self)
    }

    pub fn valid_from_slot(mut self, slot: u64) -> Self {
        self.tx = self.tx.valid_from_slot(slot);
        self
    }
    pub fn invalid_from_slot(mut self, slot: u64) -> Self {
        self.tx = self.tx.invalid_from_slot(slot);
        self
    }

    async fn calculate_fee<T>(&self, client: &mut T) -> Result<u64, TxBuilderError> 
    where T: hose_submission::EvaluateTx {
        let signed_tx = self.tx
            .build_conway_raw()?
            // TODO: is it sufficient to generate a random key here? And is it performant enough?
            .sign(Wallet::generate().into())?;

        let (coefficient, constant) = match self.network.parameters() {
            MultiEraProtocolParameters::Conway(params) => (params.minfee_a, params.minfee_b),
            _ => todo!("Implement support for non-conway protocol parameters in fee computation"),
        };
        let tx_cost = coefficient * (signed_tx.tx_bytes.0.len() as u32) + constant;

        let (mem_price, cpu_price) = match self.network.parameters() {
            MultiEraProtocolParameters::Conway(params) => (params.execution_costs.mem_price, params.execution_costs.step_price),
            _ => todo!("Implement support for non-conway protocol parameters in fee computation"),
        };
        let evals = client.evaluate_tx(&signed_tx.tx_bytes.0).await?;
        let script_cost = evals.iter().map(|e| e.memory_budget * mem_price + e.cpu_budget * cpu_price).sum();

        Ok(tx_cost + script_cost)
    }

    pub async fn build<T>(mut self, client: &mut T) -> Result<BuiltTransaction, TxBuilderError>
    where T: hose_submission::EvaluateTx + hose_submission::QueryUTxOs
    {
        let utxos = client.query_utxos(&[self.coin_selection_address.clone()]).await?;

        let fee = self.calculate_fee(client).await?;
        self.tx = self.tx.fee(fee);

        let total_output = self.tx.outputs.unwrap_or_default().iter().map(|o| o.lovelace).sum() as i128;
        let total_input = self.inputs.iter().map(|i| i.lovelace).sum() as i128;
        let change = total_output - total_input - (fee as i128);

        // Perform coin selection (for inputs and collateral input) and balancing
        // Support collateral output?
        self.tx.build_conway_raw().map(Into::into)
    }
}
