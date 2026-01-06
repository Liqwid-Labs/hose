use std::cmp::Reverse;

use hydrant::primitives::{TxOutput, TxOutputPointer};
use pallas::txbuilder::StagingTransaction;

use crate::ogmios::OgmiosClient;
use crate::ogmios::codec::Assets;
use crate::ogmios::utxo::Utxo;

#[derive(Debug, Default, Clone)]
pub struct Coins {
    pub lovelace: u64,
    pub assets: Assets,
}

impl Coins {
    pub fn contained_within(&self, other: &Self) -> bool {
        self.lovelace <= other.lovelace && self.assets.contained_within(&other.assets)
    }

    pub fn saturating_sub(self, other: Self) -> Self {
        Self {
            lovelace: self.lovelace.saturating_sub(other.lovelace),
            assets: self.assets.saturating_sub(&other.assets),
        }
    }
}

impl From<Utxo> for Coins {
    fn from(utxo: Utxo) -> Self {
        Self {
            lovelace: utxo.value.lovelace,
            assets: utxo.value.assets,
        }
    }
}

pub async fn get_input_coins(ogmios: &OgmiosClient, inputs: &[TxOutputPointer]) -> Coins {
    let output_pointers = inputs
        .iter()
        .map(|input| input.clone().into())
        .collect::<Vec<_>>();
    let utxos = ogmios.utxos_by_output(&output_pointers).await.unwrap();

    let lovelace = utxos.iter().map(|utxo| utxo.value.lovelace).sum::<u64>();
    let assets = utxos.into_iter().map(|utxo| utxo.value.assets).sum();
    Coins { lovelace, assets }
}

pub async fn get_output_coins(tx: &StagingTransaction) -> Coins {
    let Some(outputs) = tx.outputs.as_ref() else {
        return Coins::default();
    };

    let lovelace = outputs.iter().map(|output| output.lovelace).sum::<u64>();
    let assets = outputs
        .iter()
        .flat_map(|output| output.assets.as_ref())
        .map(|assets| -> Assets { assets.into() })
        .sum::<Assets>();
    Coins { lovelace, assets }
}

pub async fn select_coins(
    possible_utxos: &[Utxo],
    inputs: &[TxOutputPointer],
    input_coins: &Coins,
    output_coins: &Coins,
    fee: u64,
) -> Vec<Utxo> {
    // Filter utxos already used as inputs
    // TODO: should also filter out utxos with scripts? utxos with datums?
    let mut possible_utxos = possible_utxos
        .into_iter()
        .filter(|utxo| {
            inputs.iter().all(|input| {
                hex::encode(*input.hash) != utxo.transaction.id
                    && input.index != (utxo.index as u64)
            })
        })
        .collect::<Vec<_>>();
    possible_utxos.sort_by_key(|utxo| Reverse(utxo.value.lovelace)); // Largest-first

    let mut selected_utxos = vec![];
    let mut required_coins = Coins {
        lovelace: output_coins.lovelace + fee,
        assets: output_coins.assets.clone(),
    };
    required_coins = required_coins.saturating_sub(input_coins.clone());

    // Select for lovelace
    while required_coins.lovelace > 0 && possible_utxos.len() > 0 {
        let utxo = possible_utxos.remove(0);
        required_coins = required_coins.saturating_sub(utxo.clone().into());
        selected_utxos.push(utxo);
    }

    // Select for assets
    while possible_utxos.len() > 0
        && let Some(required_asset) = required_coins.assets.first_non_zero_asset()
    {
        // Largest-first but now by asset amount
        possible_utxos.sort_by_key(|utxo| {
            Reverse(
                *utxo
                    .value
                    .assets
                    .get(&required_asset.0)
                    .and_then(|assets| assets.get(&required_asset.1))
                    .unwrap_or(&0),
            )
        });

        let utxo = possible_utxos.remove(0);
        if utxo
            .value
            .assets
            .get(&required_asset.0)
            .and_then(|assets| assets.get(&required_asset.1))
            .unwrap_or(&0)
            == &0
        {
            break;
        }

        required_coins = required_coins.saturating_sub(utxo.clone().into());
        selected_utxos.push(utxo);
    }

    // TODO: give a proper error
    assert!(
        required_coins.lovelace == 0 && required_coins.assets.is_empty(),
        "failed to select coins, wallet doesn't contain enough funds"
    );

    vec![]
}
