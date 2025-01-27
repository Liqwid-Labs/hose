use pallas::{txbuilder::{BuiltTransaction, TxBuilderError}, wallet::keystore::PrivateKey};

pub struct Transaction {
    tx: BuiltTransaction,
}

impl Transaction {
    pub fn sign(mut self, key: PrivateKey) -> Result<Self, TxBuilderError> {
        self.tx = self.tx.sign(key)?;
        Ok(self)
    }

    pub async fn submit<T>(self, client: &mut T) -> Result<(), T::Error>
    where T: hose_submission::SubmitTx {
        client.submit_tx(&self.tx.tx_bytes.0).await?;
        Ok(())
    }
}

impl From<BuiltTransaction> for Transaction {
    fn from(tx: BuiltTransaction) -> Self {
        Self { tx }
    }
}
