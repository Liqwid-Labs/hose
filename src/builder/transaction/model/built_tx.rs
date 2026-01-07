use std::collections::HashMap;

use pallas::crypto::key::ed25519;
use pallas::ledger::primitives::{Fragment, NonEmptySet, conway};

use super::super::error::TxBuilderError;
use super::super::model::Ed25519Signer;
use super::{Bytes, Hash, PublicKey, Signature, TxHash};

#[derive(PartialEq, Eq, Debug, Clone)]
pub struct BuiltTransaction {
    pub tx_hash: TxHash,
    pub tx_bytes: Bytes,
    pub signatures: Option<HashMap<PublicKey, Signature>>,
}

impl BuiltTransaction {
    pub fn sign<K: Ed25519Signer>(mut self, private_key: &K) -> Result<Self, TxBuilderError> {
        let pubkey: [u8; 32] = private_key
            .public_key()
            .as_ref()
            .try_into()
            .map_err(|_| TxBuilderError::MalformedKey)?;

        let signature: [u8; ed25519::Signature::SIZE] = private_key
            .sign(self.tx_hash.0)
            .as_ref()
            .try_into()
            .unwrap();

        let mut new_sigs = self.signatures.unwrap_or_default();
        new_sigs.insert(Hash(pubkey), Hash(signature));
        self.signatures = Some(new_sigs);

        // TODO: chance for serialisation round trip issues?
        let mut tx = conway::Tx::decode_fragment(&self.tx_bytes.0)
            .map_err(|_| TxBuilderError::CorruptedTxBytes)?;

        let mut vkey_witnesses = tx
            .transaction_witness_set
            .vkeywitness
            .as_ref()
            .map(|x| x.clone().to_vec())
            .unwrap_or_default();

        vkey_witnesses.push(conway::VKeyWitness {
            vkey: Vec::from(pubkey.as_ref()).into(),
            signature: Vec::from(signature.as_ref()).into(),
        });

        tx.transaction_witness_set.vkeywitness =
            Some(NonEmptySet::from_vec(vkey_witnesses).unwrap());

        self.tx_bytes = tx.encode_fragment().unwrap().into();

        Ok(self)
    }

    pub fn add_signature(
        mut self,
        pub_key: ed25519::PublicKey,
        signature: [u8; 64],
    ) -> Result<Self, TxBuilderError> {
        let mut new_sigs = self.signatures.unwrap_or_default();
        new_sigs.insert(
            Hash(
                pub_key
                    .as_ref()
                    .try_into()
                    .map_err(|_| TxBuilderError::MalformedKey)?,
            ),
            Hash(signature),
        );
        self.signatures = Some(new_sigs);

        // TODO: chance for serialisation round trip issues?
        let mut tx = conway::Tx::decode_fragment(&self.tx_bytes.0)
            .map_err(|_| TxBuilderError::CorruptedTxBytes)?;

        let mut vkey_witnesses = tx
            .transaction_witness_set
            .vkeywitness
            .as_ref()
            .map(|x| x.clone().to_vec())
            .unwrap_or_default();
        vkey_witnesses.push(conway::VKeyWitness {
            vkey: Vec::from(pub_key.as_ref()).into(),
            signature: Vec::from(signature.as_ref()).into(),
        });
        tx.transaction_witness_set.vkeywitness =
            Some(NonEmptySet::from_vec(vkey_witnesses).unwrap());

        self.tx_bytes = tx.encode_fragment().unwrap().into();

        Ok(self)
    }

    pub fn remove_signature(mut self, pub_key: ed25519::PublicKey) -> Result<Self, TxBuilderError> {
        let mut new_sigs = self.signatures.unwrap_or_default();

        let pk = Hash(
            pub_key
                .as_ref()
                .try_into()
                .map_err(|_| TxBuilderError::MalformedKey)?,
        );
        new_sigs.remove(&pk);

        self.signatures = Some(new_sigs);

        // TODO: chance for serialisation round trip issues?
        let mut tx = conway::Tx::decode_fragment(&self.tx_bytes.0)
            .map_err(|_| TxBuilderError::CorruptedTxBytes)?;
        let mut vkey_witnesses = tx
            .transaction_witness_set
            .vkeywitness
            .as_ref()
            .map(|x| x.clone().to_vec())
            .unwrap_or_default();

        vkey_witnesses.retain(|x| *x.vkey != pk.0.to_vec());

        tx.transaction_witness_set.vkeywitness =
            Some(NonEmptySet::from_vec(vkey_witnesses).unwrap());

        self.tx_bytes = tx.encode_fragment().unwrap().into();

        Ok(self)
    }
}
