use hydrant::primitives::TxOutputPointer;
use pallas::txbuilder::StagingTransaction;

use crate::ogmios::OgmiosClient;
use crate::ogmios::codec::Assets;

mod largest_first;
mod random_improve;

pub async fn input_coins(ogmios: &OgmiosClient, tx: &StagingTransaction) -> (u64, Assets) {
    let output_pointers = tx
        .inputs
        .iter()
        .flatten()
        .map(|input| TxOutputPointer {
            hash: input.tx_hash.0.into(),
            index: input.txo_index,
        })
        .map(Into::into)
        .collect::<Vec<_>>();
    let utxos = ogmios.utxos_by_output(&output_pointers).await.unwrap();

    let lovelace = utxos.iter().map(|utxo| utxo.value.lovelace).sum::<u64>();
    let assets = utxos.into_iter().map(|utxo| utxo.value.assets).sum();
    (lovelace, assets)
}

pub async fn output_coins(ogmios: &OgmiosClient, tx: &StagingTransaction) -> (u64, Assets) {
    let Some(outputs) = tx.outputs.as_ref() else {
        return (0, Assets::default());
    };

    let lovelace = outputs.iter().map(|output| output.lovelace).sum::<u64>();
    let assets = outputs
        .iter()
        .flat_map(|output| output.assets.as_ref())
        .map(|assets| -> Assets { assets.into() })
        .sum::<Assets>();
    (lovelace, assets)
}
