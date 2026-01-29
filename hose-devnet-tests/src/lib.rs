#[cfg(test)]
mod test {
    use anyhow::Context as _;
    use hose::builder::TxBuilder;
    use hose::primitives::{Output, Script, ScriptKind};
    use hose_devnet::prelude::*;
    use pallas::codec::minicbor;
    use pallas::ledger::addresses::{
        Address, Network, ShelleyAddress, ShelleyDelegationPart, ShelleyPaymentPart,
    };
    use pallas::ledger::primitives::NetworkId;
    use serde_json::Value;
    use tracing::info;

    #[hose_devnet::test]
    async fn basic_tx(context: &mut DevnetContext) -> anyhow::Result<()> {
        let change_address = context.wallet.address().clone();
        let tx = TxBuilder::new(context.network_id)
            .change_address(Address::Shelley(change_address))
            .add_output(Output::new(
                Address::Shelley(context.wallet.address().clone()),
                10_000_000,
            ))
            .build(
                context.indexer.clone(),
                &context.ogmios,
                &context.protocol_params,
            )
            .await?;

        let (_signed, _res) = context.sign_and_submit_tx(tx).await?;

        Ok(())
    }

    #[hose_devnet::test]
    async fn utxo_with_datum(context: &mut DevnetContext) -> anyhow::Result<()> {
        let change_address = context.wallet.address().clone();
        let cbor = minicbor::to_vec(42)?;
        let tx = TxBuilder::new(context.network_id)
            .change_address(Address::Shelley(change_address.clone()))
            .add_output(
                Output::new(
                    Address::Shelley(context.wallet.address().clone()),
                    10_000_000,
                )
                .set_datum(cbor),
            )
            .build(
                context.indexer.clone(),
                &context.ogmios,
                &context.protocol_params,
            )
            .await?;

        let (_signed, _res) = context.sign_and_submit_tx(tx).await?;

        Ok(())
    }

    #[hose_devnet::test]
    async fn register_and_withdraw_zero_script_reward(
        context: &mut DevnetContext,
    ) -> anyhow::Result<()> {
        let change_address = context.wallet.address().clone();
        let script_bytes =
            hex::decode("5101010023259800a518a4d136564004ae69").expect("invalid script bytes");

        // TODO: we should actually extend the ogmios-client to parse the key deposit from the
        // protocol params.
        let key_deposit = {
            let genesis_path = context
                .config
                .genesis_shelley_path
                .as_ref()
                .context("genesis_shelley_path not set")?;
            let genesis_str = std::fs::read_to_string(genesis_path)?;
            let genesis_json: Value = serde_json::from_str(&genesis_str)?;
            genesis_json["protocolParams"]["keyDeposit"]
                .as_u64()
                .context("missing protocolParams.keyDeposit")?
        };

        let redeemer = hex::decode("00").unwrap();

        let script_kind = ScriptKind::PlutusV3;
        let script_hash = script_kind.hash(&script_bytes);
        let registration_tx = TxBuilder::new(context.network_id)
            .change_address(Address::Shelley(change_address.clone()))
            .register_script_stake(
                script_hash,
                script_kind,
                Some(redeemer.clone()),
                None,
                key_deposit,
            )
            .add_script(script_kind, script_bytes.clone())
            .build(
                context.indexer.clone(),
                &context.ogmios,
                &context.protocol_params,
            )
            .await?;

        context.sign_and_submit_tx(registration_tx).await?;

        let withdrawal_tx = TxBuilder::new(context.network_id)
            .change_address(Address::Shelley(change_address.clone()))
            .withdraw_from_script(
                script_hash,
                script_kind,
                0,
                redeemer,
                None,
            )
            .add_script(script_kind, script_bytes.clone())
            .build(
                context.indexer.clone(),
                &context.ogmios,
                &context.protocol_params,
            )
            .await?;

        context.sign_and_submit_tx(withdrawal_tx).await?;

        Ok(())
    }

    #[hose_devnet::test]
    async fn register_script_stake_without_redeemer(
        context: &mut DevnetContext,
    ) -> anyhow::Result<()> {
        let change_address = context.wallet.address().clone();
        let script_bytes =
            hex::decode("5101010023259800a518a4d136564004ae69").expect("invalid script bytes");
        // NOTE: use a different script kind to avoid "already registered" errors in the ledger.

        // TODO: we should actually extend the ogmios-client to parse the key deposit from the
        // protocol params.
        let key_deposit = {
            let genesis_path = context
                .config
                .genesis_shelley_path
                .as_ref()
                .context("genesis_shelley_path not set")?;
            let genesis_str = std::fs::read_to_string(genesis_path)?;
            let genesis_json: Value = serde_json::from_str(&genesis_str)?;
            genesis_json["protocolParams"]["keyDeposit"]
                .as_u64()
                .context("missing protocolParams.keyDeposit")?
        };

        let script_kind = ScriptKind::PlutusV2;
        let script_hash = script_kind.hash(&script_bytes);
        let registration_tx = TxBuilder::new(context.network_id)
            .change_address(Address::Shelley(change_address.clone()))
            .register_script_stake(
                script_hash,
                script_kind,
                None,
                None,
                key_deposit,
            )
            .build(
                context.indexer.clone(),
                &context.ogmios,
                &context.protocol_params,
            )
            .await?;

        context.sign_and_submit_tx(registration_tx).await?;

        Ok(())
    }

    #[hose_devnet::test]
    async fn spend_specific_output(context: &mut DevnetContext) -> anyhow::Result<()> {
        let change_address = context.wallet.address().clone();

        let (_signed_tx, output_pointer) = {
            let tx = TxBuilder::new(context.network_id)
                .change_address(Address::Shelley(change_address.clone()))
                .add_output(Output::new(
                    Address::Shelley(context.wallet.address().clone()),
                    42_000_000,
                ))
                .build(
                    context.indexer.clone(),
                    &context.ogmios,
                    &context.protocol_params,
                )
                .await?;

            let (signed, _res) = context.sign_and_submit_tx(tx).await?;

            let output_idx = signed
                .body()
                .outputs
                .iter()
                .position(|output| output.lovelace == 42_000_000)
                .context("output with 42 ada not found")?;

            let output_pointer: hydrant::primitives::TxOutputPointer =
                hydrant::primitives::TxOutputPointer::new(
                    signed.hash()?.0.into(),
                    output_idx as u64,
                );

            hose_devnet::wait_until_utxo_exists(context, output_pointer.clone()).await?;
            (signed, output_pointer)
        };

        let (_signed_tx, _res) = {
            let tx = TxBuilder::new(context.network_id)
                .change_address(Address::Shelley(change_address.clone()))
                .add_input(output_pointer.into())
                .build(
                    context.indexer.clone(),
                    &context.ogmios,
                    &context.protocol_params,
                )
                .await?;

            context.sign_and_submit_tx(tx).await?
        };

        Ok(())
    }

    #[hose_devnet::test]
    async fn spend_from_always_succeeds_script(context: &mut DevnetContext) -> anyhow::Result<()> {
        let change_address = context.wallet.address().clone();
        let script_bytes =
            hex::decode("5101010023259800a518a4d136564004ae69").expect("invalid script bytes");
        let script = Script::new(ScriptKind::PlutusV3, script_bytes.clone());

        let network = match context.network_id {
            NetworkId::Testnet => Network::Testnet,
            NetworkId::Mainnet => Network::Mainnet,
        };

        let script_address = Address::Shelley(ShelleyAddress::new(
            network,
            ShelleyPaymentPart::Script(script.hash.into()),
            ShelleyDelegationPart::Null,
        ));

        // Create a transaction that sends some Ada to the script address.
        let (_signed_tx, output_pointer) = {
            let tx = TxBuilder::new(context.network_id)
                .change_address(Address::Shelley(change_address.clone()))
                .add_output(Output::new(script_address.clone(), 42_000_000))
                .build(
                    context.indexer.clone(),
                    &context.ogmios,
                    &context.protocol_params,
                )
                .await?;

            let (signed, _res) = context.sign_and_submit_tx(tx).await?;

            let output_idx = signed
                .body()
                .outputs
                .iter()
                .position(|output| output.address == script_address)
                .context("output with script address not found")?;

            let output_pointer: hydrant::primitives::TxOutputPointer =
                hydrant::primitives::TxOutputPointer::new(
                    signed.hash()?.0.into(),
                    output_idx as u64,
                );

            (signed, output_pointer)
        };

        // Spend the output from the script address.
        {
            let tx = TxBuilder::new(context.network_id)
                .change_address(Address::Shelley(change_address.clone()))
                .add_script_input(
                    output_pointer.into(),
                    hex::decode("00").unwrap(),
                    None,
                    ScriptKind::PlutusV3,
                )
                .add_script(ScriptKind::PlutusV3, script.bytes.clone())
                .build(
                    context.indexer.clone(),
                    &context.ogmios,
                    &context.protocol_params,
                )
                .await?;

            context.sign_and_submit_tx(tx).await?;
        }

        Ok(())
    }

    #[hose_devnet::test]
    async fn chain_spend(context: &mut DevnetContext) -> anyhow::Result<()> {
        const NUM_TXS: u64 = 50;
        const AMOUNT_STEP: u64 = 1_000_000;
        // Start with enough to cover decreasing amounts + a buffer for tx fees
        let start_amount = (NUM_TXS * AMOUNT_STEP) + AMOUNT_STEP;

        // 1. Grab initial UTXO from wallet
        let mut current_pointer = {
            let indexer = context.indexer.lock().await;
            let utxos = indexer.address_utxos(&context.wallet.address().to_vec())?;

            // Find the UTXO with the most funds to ensure we can cover the chain
            let output = utxos
                .into_iter()
                .max_by_key(|output| output.lovelace)
                .context("no utxos found in wallet")?;

            anyhow::ensure!(
                output.lovelace >= start_amount,
                "wallet does not have enough funds: has {}, needs {}",
                output.lovelace,
                start_amount
            );

            let pointer: hydrant::primitives::TxOutputPointer = output.clone().into();
            info!(
                "Starting chain with UTXO: {}#{} ({} lovelace)",
                pointer.hash, pointer.index, output.lovelace
            );
            pointer
        };

        let start_time = std::time::Instant::now();

        // 2. Chain Loop
        for i in 0..NUM_TXS {
            // Decrease by AMOUNT_STEP each time
            let next_amount = start_amount - ((i + 1) * AMOUNT_STEP);
            info!(
                "Submitting tx {}/{} (target output: {})",
                i + 1,
                NUM_TXS,
                next_amount
            );

            let wallet_addr = Address::Shelley(context.wallet.address().clone());
            let tx = TxBuilder::new(context.network_id)
                .change_address(wallet_addr.clone())
                .add_input(current_pointer.clone().into())
                .add_output(Output::new(wallet_addr, next_amount))
                .build(
                    context.indexer.clone(),
                    &context.ogmios,
                    &context.protocol_params,
                )
                .await?;

            let (signed, _) = context.sign_and_submit_tx(tx).await?;

            // Identify the output to spend in the next iteration.
            // We look for the output with the specific amount we set.
            // Since `next_amount` is distinct and decreased each time, this is safe.
            let output_idx = signed
                .body()
                .outputs
                .iter()
                .position(|output| output.lovelace == next_amount)
                .context("chained output not found in transaction")?;

            current_pointer = hydrant::primitives::TxOutputPointer::new(
                signed.hash()?.0.into(),
                output_idx as u64,
            );
        }

        let elapsed = start_time.elapsed();
        info!(
            "Submitted {} chained txs in {:.2?}, average tx time: {:.2?}",
            NUM_TXS,
            elapsed,
            elapsed / NUM_TXS as u32
        );

        Ok(())
    }
}
