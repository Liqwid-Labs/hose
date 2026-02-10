#[cfg(test)]
mod test {
    use anyhow::{Context, ensure};
    use hose::builder::TxBuilder;
    use hose::primitives::{Asset, AssetId, Hash, Output, PubKeyHash, RedeemerPurpose, Script, ScriptKind};
    use hose_devnet::prelude::*;
    use hose_devnet::{
        empty_redeemer, network_from_network_id, nonced_always_succeeds_script,
        validator_to_address,
    };
    use hydrant::primitives::TxOutputPointer;
    use pallas::codec::minicbor;
    use pallas::ledger::addresses::{
        Address, ShelleyAddress, ShelleyDelegationPart, ShelleyPaymentPart,
    };
    use pallas::ledger::primitives::Fragment;
    use pallas::ledger::primitives::alonzo::NativeScript;
    use pallas::ledger::traverse::ComputeHash;
    use tracing::info;

    const MIN_ADA: u64 = 2_000_000;

    fn address_to_pub_key_hash(address: Address) -> PubKeyHash {
        match address {
            Address::Shelley(s) => match s.payment() {
                ShelleyPaymentPart::Key(h) => Hash::from(*h),
                _ => panic!("expected key payment part"),
            },
            _ => panic!("unexpected address type"),
        }
    }

    #[hose_devnet::test]
    async fn basic_tx(context: &mut DevnetContext) -> anyhow::Result<()> {
        let tx = TxBuilder::new(context.network_id, context.wallet.address())
            .add_output(Output::new(context.wallet.address(), MIN_ADA))
            .build(&context.indexer, &context.ogmios, &context.protocol_params)
            .await?;

        let (_signed, _res) = context.sign_and_submit_tx(tx).await?;

        Ok(())
    }

    #[hose_devnet::test]
    async fn utxo_with_datum(context: &mut DevnetContext) -> anyhow::Result<()> {
        let cbor = minicbor::to_vec(42)?;
        let tx = TxBuilder::new(context.network_id, context.wallet.address())
            .add_output(Output::new(context.wallet.address(), MIN_ADA).set_datum(cbor))
            .build(&context.indexer, &context.ogmios, &context.protocol_params)
            .await?;

        let (_signed, _res) = context.sign_and_submit_tx(tx).await?;

        Ok(())
    }

    #[hose_devnet::test]
    async fn reference_input(context: &DevnetContext) -> anyhow::Result<()> {
        let validator = nonced_always_succeeds_script()?;
        let validator_address = validator_to_address(context, &validator);

        info!("Deploying the ref script");
        let deploy_tx = TxBuilder::new(context.network_id, context.wallet.address())
            // for convenience, we'll use the same validator for the ref input _and_ for the input
            // we'll want to spend later.
            // the output below holds a script we'll reference later on.
            .add_output(
                Output::new(validator_address.clone(), MIN_ADA)
                    .set_script(validator.kind, validator.bytes),
            )
            // and the output below is the one that will be spent later.
            .add_output(Output::new(validator_address.clone(), MIN_ADA))
            .build(&context.indexer, &context.ogmios, &context.protocol_params)
            .await?;

        let (signed, res) = context.sign_and_submit_tx(deploy_tx).await?;
        info!("deployment transaction id: {:#?}", res.transaction.id);

        let (ref_output_pointer, spend_output_pointer) = (
            TxOutputPointer::new(signed.hash()?, 0),
            TxOutputPointer::new(signed.hash()?, 1),
        );
        hose_devnet::wait_until_utxo_exists(context, ref_output_pointer.clone()).await?;

        info!("Spending from a validator using the ref script");
        let ref_and_spend_tx = TxBuilder::new(context.network_id, context.wallet.address())
            // Note that we don't attach the script, but instead read it from the ref input.
            .add_reference_input(ref_output_pointer.into())
            .add_script_input(
                spend_output_pointer.into(),
                empty_redeemer(),
                validator.kind,
            )
            .build(&context.indexer, &context.ogmios, &context.protocol_params)
            .await?;

        context.sign_and_submit_tx(ref_and_spend_tx).await?;

        Ok(())
    }

    #[hose_devnet::test]
    async fn register_and_withdraw_zero_script_reward(
        context: &mut DevnetContext,
    ) -> anyhow::Result<()> {
        let script = nonced_always_succeeds_script()?;
        let registration_tx = TxBuilder::new(context.network_id, context.wallet.address())
            .register_script_stake(script.hash, script.kind, Some(empty_redeemer()))
            .add_script(script.kind, script.bytes.clone())
            .build(&context.indexer, &context.ogmios, &context.protocol_params)
            .await?;

        context.sign_and_submit_tx(registration_tx).await?;

        let withdrawal_tx = TxBuilder::new(context.network_id, context.wallet.address())
            .withdraw_from_script(script.hash, script.kind, 0, Some(empty_redeemer()))?
            .add_script(script.kind, script.bytes.clone())
            .build(&context.indexer, &context.ogmios, &context.protocol_params)
            .await?;

        let (_, withdrawal_tx_id) = context.sign_and_submit_tx(withdrawal_tx).await?;
        info!("Withdrawal tx hash: {}", withdrawal_tx_id.transaction.id);

        let deregistration_tx = TxBuilder::new(context.network_id, context.wallet.address())
            .deregister_script_stake(script.hash, script.kind, empty_redeemer())
            .add_script(script.kind, script.bytes.clone())
            .build(&context.indexer, &context.ogmios, &context.protocol_params)
            .await?;

        context.sign_and_submit_tx(deregistration_tx).await?;

        Ok(())
    }

    #[hose_devnet::test]
    async fn register_script_stake_without_redeemer(
        context: &mut DevnetContext,
    ) -> anyhow::Result<()> {
        let script = nonced_always_succeeds_script()?;
        let registration_tx = TxBuilder::new(context.network_id, context.wallet.address())
            .register_script_stake(script.hash, script.kind, None)
            .build(&context.indexer, &context.ogmios, &context.protocol_params)
            .await?;

        context.sign_and_submit_tx(registration_tx).await?;

        Ok(())
    }

    #[hose_devnet::test]
    async fn mint_and_burn_assets(context: &mut DevnetContext) -> anyhow::Result<()> {
        let policy_script = nonced_always_succeeds_script()?;
        let policy = policy_script.hash;
        let asset_name = b"qAda".to_vec();
        let mint_amount: u64 = 10_000_000;

        let mint_tx = TxBuilder::new(context.network_id, context.wallet.address())
            .mint_asset(
                Asset {
                    policy,
                    name: asset_name.clone(),
                    quantity: mint_amount,
                },
                policy_script.kind,
                empty_redeemer(),
            )?
            .add_script(policy_script.kind, policy_script.bytes.clone())
            .add_output(Output::new(context.wallet.address(), MIN_ADA).add_asset(
                policy,
                asset_name.clone(),
                mint_amount,
            )?)
            .build(&context.indexer, &context.ogmios, &context.protocol_params)
            .await?;

        let (signed, _res) = context.sign_and_submit_tx(mint_tx).await?;
        let asset_id = AssetId::new(policy, asset_name.clone());
        let output_idx = signed
            .body()
            .outputs
            .iter()
            .position(|output| {
                output
                    .assets
                    .as_ref()
                    .is_some_and(|assets| assets.get(&asset_id) == Some(&mint_amount))
            })
            .context("minted output not found")?;
        let output_pointer = TxOutputPointer::new(signed.hash()?.0.into(), output_idx as u64);
        hose_devnet::wait_until_utxo_exists(context, output_pointer.clone()).await?;

        let burn_tx = TxBuilder::new(context.network_id, context.wallet.address())
            .add_input(output_pointer.into())
            .add_output(Output::new(context.wallet.address(), MIN_ADA))
            .burn_asset(
                Asset {
                    policy,
                    name: asset_name,
                    quantity: mint_amount,
                },
                policy_script.kind,
                empty_redeemer(),
            )?
            .add_script(policy_script.kind, policy_script.bytes.clone())
            .build(&context.indexer, &context.ogmios, &context.protocol_params)
            .await?;

        context.sign_and_submit_tx(burn_tx).await?;

        Ok(())
    }

    #[hose_devnet::test]
    async fn mint_two_assets_in_one_transaction(context: &mut DevnetContext) -> anyhow::Result<()> {
        let policy_script = nonced_always_succeeds_script()?;
        let policy = policy_script.hash;

        let asset_a = b"qAda".to_vec();
        let asset_b = b"qDJED".to_vec();
        let amount_a: u64 = 1_000_000;
        let amount_b: u64 = 10_000_000;

        let mint_output = Output::new(context.wallet.address(), MIN_ADA)
            .add_asset(policy, asset_a.clone(), amount_a)?
            .add_asset(policy, asset_b.clone(), amount_b)?;

        let mint_tx = TxBuilder::new(context.network_id, context.wallet.address())
            .mint_asset(
                Asset {
                    policy,
                    name: asset_a.clone(),
                    quantity: amount_a,
                },
                policy_script.kind,
                empty_redeemer(),
            )?
            .mint_asset(
                Asset {
                    policy,
                    name: asset_b.clone(),
                    quantity: amount_b,
                },
                policy_script.kind,
                empty_redeemer(),
            )?
            .add_script(policy_script.kind, policy_script.bytes.clone())
            .add_output(mint_output)
            .build(&context.indexer, &context.ogmios, &context.protocol_params)
            .await?;

        context.sign_and_submit_tx(mint_tx).await?;

        Ok(())
    }

    #[hose_devnet::test]
    async fn mint_and_burn_same_asset_is_noop(context: &mut DevnetContext) -> anyhow::Result<()> {
        let policy_script = nonced_always_succeeds_script()?;
        let policy = policy_script.hash;
        let asset_name = b"NETZERO".to_vec();

        let built = TxBuilder::new(context.network_id, context.wallet.address())
            .mint_asset(
                Asset {
                    policy,
                    name: asset_name.clone(),
                    quantity: 5,
                },
                policy_script.kind,
                empty_redeemer(),
            )?
            .burn_asset(
                Asset {
                    policy,
                    name: asset_name,
                    quantity: 5,
                },
                policy_script.kind,
                empty_redeemer(),
            )?
            .add_output(Output::new(context.wallet.address(), MIN_ADA))
            .build(&context.indexer, &context.ogmios, &context.protocol_params)
            .await?;

        ensure!(
            built.body().mint.is_empty(),
            "expected zero mint entries for net-zero mint"
        );
        if let Some(redeemers) = built.body().redeemers.as_ref() {
            ensure!(
                !redeemers.contains_key(&RedeemerPurpose::Mint(policy)),
                "expected no mint redeemer for net-zero mint"
            );
        }

        context.sign_and_submit_tx(built).await?;

        Ok(())
    }

    #[hose_devnet::test]
    async fn spend_specific_output(context: &mut DevnetContext) -> anyhow::Result<()> {
        let (_signed_tx, output_pointer) = {
            let tx = TxBuilder::new(context.network_id, context.wallet.address())
                .add_output(Output::new(context.wallet.address(), 42_000_000))
                .build(&context.indexer, &context.ogmios, &context.protocol_params)
                .await?;

            let (signed, _res) = context.sign_and_submit_tx(tx).await?;

            let output_idx = signed
                .body()
                .outputs
                .iter()
                .position(|output| output.lovelace == 42_000_000)
                .context("output with 42 ada not found")?;

            let output_pointer: TxOutputPointer =
                TxOutputPointer::new(signed.hash()?.0.into(), output_idx as u64);

            hose_devnet::wait_until_utxo_exists(context, output_pointer.clone()).await?;
            (signed, output_pointer)
        };

        let (_signed_tx, _res) = {
            let tx = TxBuilder::new(context.network_id, context.wallet.address())
                .add_input(output_pointer.into())
                .build(&context.indexer, &context.ogmios, &context.protocol_params)
                .await?;

            context.sign_and_submit_tx(tx).await?
        };

        Ok(())
    }

    #[hose_devnet::test]
    async fn spend_from_always_succeeds_script(context: &mut DevnetContext) -> anyhow::Result<()> {
        let script_bytes =
            hex::decode("5101010023259800a518a4d136564004ae69").expect("invalid script bytes");
        let script = Script::new(ScriptKind::PlutusV3, script_bytes.clone());
        let script_address = validator_to_address(context, &script);

        // Create a transaction that sends some Ada to the script address.
        let (_signed_tx, output_pointer) = {
            let tx = TxBuilder::new(context.network_id, context.wallet.address())
                .add_output(Output::new(script_address.clone(), 42_000_000))
                .build(&context.indexer, &context.ogmios, &context.protocol_params)
                .await?;

            let (signed, _res) = context.sign_and_submit_tx(tx).await?;

            let output_idx = signed
                .body()
                .outputs
                .iter()
                .position(|output| output.address == script_address)
                .context("output with script address not found")?;

            let output_pointer: TxOutputPointer =
                TxOutputPointer::new(signed.hash()?.0.into(), output_idx as u64);

            (signed, output_pointer)
        };

        // Spend the output from the script address.
        {
            let tx = TxBuilder::new(context.network_id, context.wallet.address())
                .add_script_input(
                    output_pointer.into(),
                    empty_redeemer(),
                    ScriptKind::PlutusV3,
                )
                .add_script(ScriptKind::PlutusV3, script.bytes.clone())
                .build(&context.indexer, &context.ogmios, &context.protocol_params)
                .await?;

            context.sign_and_submit_tx(tx).await?;
        }

        Ok(())
    }

    #[hose_devnet::test]
    async fn chain_spend(context: &mut DevnetContext) -> anyhow::Result<()> {
        const NUM_TXS: u64 = 10;
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

            let pointer: TxOutputPointer = output.clone().into();
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

            let tx = TxBuilder::new(context.network_id, context.wallet.address())
                .add_input(current_pointer.clone().into())
                .add_output(Output::new(context.wallet.address(), next_amount))
                .build(&context.indexer, &context.ogmios, &context.protocol_params)
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

            current_pointer = TxOutputPointer::new(signed.hash()?.0.into(), output_idx as u64);
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

    #[hose_devnet::test]
    async fn multi_witness_tx(context: &mut DevnetContext) -> anyhow::Result<()> {
        // 1. Create a second wallet
        // Just increment the last byte of the configured key to get a new one
        let original_key_hex = &context.config.private_key_hex;
        let mut key_bytes = hex::decode(original_key_hex)?;
        key_bytes[0] = key_bytes[0].wrapping_add(1); // Simple perturbation
        let wallet2 = hose::wallet::WalletBuilder::new(context.config.network)
            .from_hex(hex::encode(key_bytes))?;

        // 2. Fund Wallet 2 (send 10 ADA)
        {
            let tx = TxBuilder::new(context.network_id, context.wallet.address())
                .add_output(Output::new(wallet2.address(), 10_000_000))
                .build(&context.indexer, &context.ogmios, &context.protocol_params)
                .await?;
            context.sign_and_submit_tx(tx).await?;
        }

        // 3. Find inputs for both wallets
        let input1 = {
            let indexer = context.indexer.lock().await;
            let utxos = indexer.address_utxos(&context.wallet.address().to_vec())?;
            let output = utxos
                .iter()
                .max_by_key(|u| u.lovelace)
                .context("wallet 1 empty")?;
            TxOutputPointer::from(output.clone())
        };

        // Wait for Wallet 2 funds
        let input2 = loop {
            let indexer = context.indexer.lock().await;
            let utxos = indexer.address_utxos(&wallet2.address().to_vec())?;
            if let Some(utxo) = utxos.first() {
                break TxOutputPointer::from(utxo.clone());
            }
            tokio::time::sleep(std::time::Duration::from_millis(500)).await;
        };

        // 4. Construct Multi-witness Tx
        // Input from Wallet 1 + Input from Wallet 2 -> Output to Wallet 1
        let tx = TxBuilder::new(context.network_id, context.wallet.address())
            .add_input(input1.into())
            .add_input(input2.into())
            .add_output(Output::new(context.wallet.address(), 5_000_000)) // Just sending some back
            .build(&context.indexer, &context.ogmios, &context.protocol_params)
            .await?;

        // 5. Sign with second wallet
        // Note: `sign` adds signatures.
        let tx = tx.sign(&wallet2)?; // Sign with Wallet 2

        // 6. Sign and submit with first wallet
        context.sign_and_submit_tx(tx).await?;

        Ok(())
    }

    #[hose_devnet::test]
    async fn delegate_to_unknown_pool(context: &mut DevnetContext) -> anyhow::Result<()> {
        let pub_key_hash = address_to_pub_key_hash(context.wallet.address());

        // 1. Register Stake Key
        let registration_tx = TxBuilder::new(context.network_id, context.wallet.address())
            .register_stake(pub_key_hash)
            .build(&context.indexer, &context.ogmios, &context.protocol_params)
            .await?;

        match context.sign_and_submit_tx(registration_tx).await {
            Ok(_) => {}
            Err(e) => {
                let err_msg = e.to_string();
                info!(
                    "Register stake tx failed (assuming already registered), continuing: {}",
                    err_msg
                );
            }
        }

        // 2. Delegate to dummy pool (Expect Failure)
        let dummy_pool_id = Hash::from([0xAA; 28]);
        let delegation_tx = TxBuilder::new(context.network_id, context.wallet.address())
            .delegate_stake(pub_key_hash, dummy_pool_id)
            .build(&context.indexer, &context.ogmios, &context.protocol_params)
            .await?;

        match context.sign_and_submit_tx(delegation_tx).await {
            Ok(_) => panic!("Delegation to dummy pool should have failed"),
            Err(e) => {
                let err_msg = e.to_string();
                if !err_msg.contains("UnknownStakePool") {
                    panic!("Unexpected error during delegation: {}", err_msg);
                }
                info!("Delegation failed as expected: UnknownStakePool");
            }
        }

        Ok(())
    }

    #[hose_devnet::test]
    async fn delegate_to_known_pool(context: &mut DevnetContext) -> anyhow::Result<()> {
        let pub_key_hash = address_to_pub_key_hash(context.wallet.address());

        // 1. Register Stake Key
        let registration_tx = TxBuilder::new(context.network_id, context.wallet.address())
            .register_stake(pub_key_hash)
            .build(&context.indexer, &context.ogmios, &context.protocol_params)
            .await?;

        match context.sign_and_submit_tx(registration_tx).await {
            Ok(_) => {}
            Err(e) => {
                let err_msg = e.to_string();
                info!(
                    "Register stake tx failed (assuming already registered), continuing: {}",
                    err_msg
                );
            }
        }

        // 2. Delegate to valid pool
        let pool_hex = "8a219b698d3b6e034391ae84cee62f1d76b6fbc45ddfe4e31e0d4b60";
        let pool_bytes = hex::decode(pool_hex)?;
        let valid_pool_id =
            Hash::from(TryInto::<[u8; 28]>::try_into(pool_bytes).expect("invalid pool id length"));

        let delegation_tx = TxBuilder::new(context.network_id, context.wallet.address())
            .delegate_stake(pub_key_hash, valid_pool_id)
            .build(&context.indexer, &context.ogmios, &context.protocol_params)
            .await?;

        context.sign_and_submit_tx(delegation_tx).await?;

        Ok(())
    }

    #[hose_devnet::test]
    async fn delegate_script_stake_to_known_pool(
        context: &mut DevnetContext,
    ) -> anyhow::Result<()> {
        let script = nonced_always_succeeds_script()?;
        let pool_hex = "8a219b698d3b6e034391ae84cee62f1d76b6fbc45ddfe4e31e0d4b60";
        let pool_bytes = hex::decode(pool_hex)?;
        let valid_pool_id =
            Hash::from(TryInto::<[u8; 28]>::try_into(pool_bytes).expect("invalid pool id length"));

        // 1. Register Script Stake
        let registration_tx = TxBuilder::new(context.network_id, context.wallet.address())
            .register_script_stake(script.hash, script.kind, Some(empty_redeemer()))
            .add_script(script.kind, script.bytes.clone())
            .build(&context.indexer, &context.ogmios, &context.protocol_params)
            .await?;

        match context.sign_and_submit_tx(registration_tx).await {
            Ok(_) => {}
            Err(e) => {
                let err_msg = e.to_string();
                info!(
                    "Register script stake tx failed (assuming already registered), continuing: {}",
                    err_msg
                );
            }
        }

        // 2. Delegate Script Stake
        let delegation_tx = TxBuilder::new(context.network_id, context.wallet.address())
            .delegate_script_stake(
                script.hash,
                valid_pool_id,
                script.kind,
                Some(empty_redeemer()),
                None,
            )
            .add_script(script.kind, script.bytes.clone())
            .build(&context.indexer, &context.ogmios, &context.protocol_params)
            .await?;

        context.sign_and_submit_tx(delegation_tx).await?;

        Ok(())
    }

    #[hose_devnet::test]
    async fn register_and_deregister_stake_key(context: &mut DevnetContext) -> anyhow::Result<()> {
        let pub_key_hash = address_to_pub_key_hash(context.wallet.address());

        // 1. Register Stake Key
        let registration_tx = TxBuilder::new(context.network_id, context.wallet.address())
            .register_stake(pub_key_hash)
            .build(&context.indexer, &context.ogmios, &context.protocol_params)
            .await?;

        match context.sign_and_submit_tx(registration_tx).await {
            Ok(_) => {}
            Err(e) => {
                let err_msg = e.to_string();
                info!(
                    "Register stake tx failed (assuming already registered), continuing: {}",
                    err_msg
                );
            }
        }

        // 2. Deregister Stake Key
        let deregistration_tx = TxBuilder::new(context.network_id, context.wallet.address())
            .deregister_stake(pub_key_hash)
            .build(&context.indexer, &context.ogmios, &context.protocol_params)
            .await?;

        context.sign_and_submit_tx(deregistration_tx).await?;

        Ok(())
    }

    #[hose_devnet::test]
    async fn spend_from_native_script(context: &mut DevnetContext) -> anyhow::Result<()> {
        let script =
            NativeScript::ScriptPubkey(address_to_pub_key_hash(context.wallet.address()).into());
        let script_address = Address::Shelley(ShelleyAddress::new(
            network_from_network_id(context.network_id),
            ShelleyPaymentPart::Script(script.compute_hash().into()),
            ShelleyDelegationPart::Null,
        ));
        let script_bytes = script
            .encode_fragment()
            .expect("failed to encode native script as cbor");

        let pay_to_script_tx = TxBuilder::new(context.network_id, context.wallet.address().clone())
            .add_output(Output::new(script_address, MIN_ADA))
            .build(&context.indexer, &context.ogmios, &context.protocol_params)
            .await?;
        let (signed, _res) = context.sign_and_submit_tx(pay_to_script_tx).await?;
        let script_output_pointer =
            hydrant::primitives::TxOutputPointer::new(signed.hash()?.into(), 0);
        hose_devnet::wait_until_utxo_exists(context, script_output_pointer.clone()).await?;

        let spend_from_script_tx =
            TxBuilder::new(context.network_id, context.wallet.address().clone())
                .add_input(script_output_pointer.into())
                .add_script(ScriptKind::Native, script_bytes)
                .build(&context.indexer, &context.ogmios, &context.protocol_params)
                .await?;

        context.sign_and_submit_tx(spend_from_script_tx).await?;

        Ok(())
    }

    #[hose_devnet::test]
    async fn withdraw_from_native_script(context: &mut DevnetContext) -> anyhow::Result<()> {
        let script =
            NativeScript::ScriptPubkey(address_to_pub_key_hash(context.wallet.address()).into());
        let script_bytes = script
            .encode_fragment()
            .expect("failed to encode native script as cbor");

        let registration_tx = TxBuilder::new(context.network_id, context.wallet.address().into())
            .register_script_stake(script.compute_hash().into(), ScriptKind::Native, None)
            .build(&context.indexer, &context.ogmios, &context.protocol_params)
            .await?;

        match context.sign_and_submit_tx(registration_tx).await {
            Ok((signed, _res)) => {
                hose_devnet::wait_until_tx_is_included(context, signed.hash()?.into()).await?;
            }
            Err(e) => {
                let err_msg = e.to_string();
                info!(
                    "Register stake tx failed (assuming already registered), continuing: {}",
                    err_msg
                );
            }
        }

        let withdrawal_tx = TxBuilder::new(context.network_id, context.wallet.address().into())
            .withdraw_from_script(script.compute_hash().into(), ScriptKind::Native, 0, None)?
            .add_script(ScriptKind::Native, script_bytes)
            .build(&context.indexer, &context.ogmios, &context.protocol_params)
            .await?;

        context.sign_and_submit_tx(withdrawal_tx).await?;

        Ok(())
    }
}
