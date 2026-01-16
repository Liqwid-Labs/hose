#[cfg(test)]
pub mod util;

#[cfg(test)]
pub mod context;

#[cfg(test)]
mod test {

    use anyhow::Context as _;
    use hose::builder::{BuiltTx, TxBuilder};
    use hose::ogmios::submit::SubmitResult;
    use hose::primitives::Output;
    use pallas::ledger::addresses::Address;
    use tracing::{debug, info};

    use crate::devnet_tests::context::DevnetContext;
    use crate::devnet_tests::util;

    async fn sign_and_submit_tx(
        context: &mut DevnetContext,
        tx: BuiltTx,
    ) -> anyhow::Result<(BuiltTx, SubmitResult)> {
        let signed = tx.sign(&context.wallet)?;
        info!("Submitting transaction: {}", signed.hash()?);
        match context.ogmios.submit(&signed.cbor()).await {
            Ok(res) => {
                debug!("Submitted transaction: {:?}", res.transaction.id);
                assert_eq!(res.transaction.id, signed.hash()?.to_string());
                util::wait_until_tx_is_included(context, signed.hash()?).await?;
                Ok((signed, res))
            }
            Err(e) => {
                info!("Failed transaction CBOR: {:?}", signed.cbor_hex());
                Err(anyhow::anyhow!("Failed to submit transaction: {}", e))
            }
        }
    }

    #[tokio::test]
    async fn basic_tx() -> anyhow::Result<()> {
        let context = DevnetContext::get().await;
        let mut context = context.lock().await;
        if let Err(e) = context.sync.run_until_synced().await {
            panic!("Failed to sync: {:?}", e);
        }

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

        let (_signed, _res) = sign_and_submit_tx(&mut context, tx).await?;

        Ok(())
    }

    #[tokio::test]
    async fn utxo_with_datum() -> anyhow::Result<()> {
        let context = DevnetContext::get().await;
        let mut context = context.lock().await;
        if let Err(e) = context.sync.run_until_synced().await {
            panic!("Failed to sync: {:?}", e);
        }

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

        let cbor_hex = tx.cbor_hex();

        let (_signed, _res) = sign_and_submit_tx(&mut context, tx).await?;

        Ok(())
    }

    #[tokio::test]
    async fn spend_specific_output() -> anyhow::Result<()> {
        let context = DevnetContext::get().await;
        let mut context = context.lock().await;
        if let Err(e) = context.sync.run_until_synced().await {
            panic!("Failed to sync: {:?}", e);
        }

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

            let (signed, _res) = sign_and_submit_tx(&mut context, tx).await?;

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

            util::wait_until_utxo_exists(&mut context, output_pointer.clone()).await?;
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

            sign_and_submit_tx(&mut context, tx).await?
        };

        Ok(())
    }
}
