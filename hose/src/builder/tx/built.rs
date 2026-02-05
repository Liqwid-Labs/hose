use std::collections::HashMap;

use pallas::crypto::key::ed25519;
use pallas::ledger::primitives::{Fragment, NonEmptySet, conway};

use super::TxBuilderError;
use crate::primitives::{Ed25519Signer, Hash, PublicKey, Signature, TxHash};

#[derive(PartialEq, Eq, Debug, Clone)]
pub struct BuiltTransaction {
    pub hash: TxHash,
    pub bytes: Vec<u8>,
    pub signatures: Option<HashMap<PublicKey, Signature>>,
}

impl BuiltTransaction {
    pub fn sign<K: Ed25519Signer>(mut self, private_key: &K) -> Result<Self, TxBuilderError> {
        let pubkey: [u8; 32] = private_key
            .public_key()
            .as_ref()
            .try_into()
            .map_err(|_| TxBuilderError::MalformedKey)?;

        let signature: [u8; ed25519::Signature::SIZE] =
            private_key.sign(self.hash.0).as_ref().try_into().unwrap();

        let mut new_sigs = self.signatures.unwrap_or_default();
        new_sigs.insert(Hash(pubkey), Hash(signature));
        self.signatures = Some(new_sigs.clone());

        // TODO: chance for serialisation round trip issues?
        let mut tx = conway::Tx::decode_fragment(&self.bytes)
            .map_err(|_| TxBuilderError::CorruptedTxBytes)?;

        let vkey_witnesses = new_sigs
            .into_iter()
            .map(|(pk, sig)| conway::VKeyWitness {
                vkey: pk.to_vec().into(),
                signature: sig.to_vec().into(),
            })
            .collect::<Vec<_>>();

        tx.transaction_witness_set.vkeywitness =
            Some(NonEmptySet::from_vec(vkey_witnesses).unwrap());

        self.bytes = tx.encode_fragment().unwrap();

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
        self.signatures = Some(new_sigs.clone());

        // TODO: chance for serialisation round trip issues?
        let mut tx = conway::Tx::decode_fragment(&self.bytes)
            .map_err(|_| TxBuilderError::CorruptedTxBytes)?;

        let vkey_witnesses = new_sigs
            .into_iter()
            .map(|(pk, sig)| conway::VKeyWitness {
                vkey: pk.to_vec().into(),
                signature: sig.to_vec().into(),
            })
            .collect::<Vec<_>>();

        tx.transaction_witness_set.vkeywitness =
            Some(NonEmptySet::from_vec(vkey_witnesses).unwrap());

        self.bytes = tx.encode_fragment().unwrap();

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
        let mut tx = conway::Tx::decode_fragment(&self.bytes)
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

        self.bytes = tx.encode_fragment().unwrap();

        Ok(self)
    }
}
