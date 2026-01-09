#[cfg(test)]
pub mod lock;

#[cfg(test)]
pub mod util;

#[cfg(test)]
mod test {

    use std::str::FromStr as _;
    use std::sync::atomic::{AtomicBool, Ordering};

    use anyhow::Context as _;
    use clap::Parser as _;
    use hose::builder::{BuiltTx, Input, Output, TxBuilder};
    use hose::ogmios::OgmiosClient;
    use hose::ogmios::submit::SubmitResult;
    use hose::wallet::{Wallet, WalletBuilder};
    use pallas::ledger::addresses::Address;
    use pallas::ledger::primitives::NetworkId;
    use test_context::{AsyncTestContext, test_context};
    use tracing::{debug, info, warn};
    use url::Url;

    use crate::config::{self, Config};
    use crate::devnet_tests::lock::TestLock;
    use crate::devnet_tests::util;

    static LOCK: AtomicBool = AtomicBool::new(false);

    pub struct DevnetContext {
        pub config: Config,
        pub network_id: NetworkId,
        pub ogmios: OgmiosClient,
        pub protocol_params: hose::ogmios::pparams::ProtocolParams,
        pub wallet: Wallet,
    }

    impl AsyncTestContext for DevnetContext {
        async fn setup() -> Self {
            match tracing_subscriber::fmt::try_init() {
                Ok(_) => (),
                Err(e) => {
                    // Ignore error, tracing probably is already initialized
                    // TODO: Could we catch this better?
                }
            }
            dotenv::dotenv()
                .context("could not load .env file")
                .unwrap();

            let config = config::Config::parse();
            let network_id = NetworkId::try_from(config.network.value())
                .expect("failed to convert network to network id");

            let ogmios = OgmiosClient::new(Url::parse(&config.ogmios_url).unwrap());

            let protocol_params = ogmios.protocol_params().await.unwrap();

            let wallet = WalletBuilder::new(config.network.clone())
                .from_hex(config.private_key_hex.clone())
                .unwrap();

            Self {
                config,
                network_id,
                ogmios,
                protocol_params,
                wallet,
            }
        }

        fn teardown(self) -> impl std::future::Future<Output = ()> + Send {
            async {}
        }
    }

    async fn sign_and_submit_tx(
        context: &mut DevnetContext,
        tx: BuiltTx,
    ) -> anyhow::Result<(BuiltTx, SubmitResult)> {
        let signed = tx.sign(&context.wallet)?;
        match context.ogmios.submit(&signed.cbor()).await {
            Ok(res) => {
                debug!("Submitted transaction: {:?}", res.transaction.id);
                assert_eq!(res.transaction.id, signed.hash()?.to_string());
                Ok((signed, res))
            }
            Err(e) => {
                info!("Failed transaction CBOR: {:?}", signed.cbor_hex());
                Err(anyhow::anyhow!("Failed to submit transaction: {}", e))
            }
        }
    }

    #[test_context(DevnetContext)]
    #[tokio::test]
    async fn basic_tx(context: &mut DevnetContext) -> anyhow::Result<()> {
        let _lock = TestLock::wait_and_lock(&LOCK);

        let change_address = context.wallet.address().clone();
        let tx = TxBuilder::new(context.network_id)
            .change_address(Address::Shelley(change_address))
            .add_output(Output::new(
                Address::Shelley(context.wallet.address().clone()),
                10_000_000,
            ))
            .build(&context.ogmios, &context.protocol_params)
            .await?;

        let (_signed, _res) = sign_and_submit_tx(context, tx).await?;

        Ok(())
    }

    #[test_context(DevnetContext)]
    #[tokio::test]
    async fn utxo_with_datum(context: &mut DevnetContext) -> anyhow::Result<()> {
        let _lock = TestLock::wait_and_lock(&LOCK);

        let change_address = context.wallet.address().clone();
        let cbor = minicbor::to_vec(42)?;
        let tx = TxBuilder::new(context.network_id)
            .change_address(Address::Shelley(change_address.clone()))
            .add_output(
                Output::new(
                    Address::Shelley(context.wallet.address().clone()),
                    10_000_000,
                )
                .set_inline_datum(cbor),
            )
            .build(&context.ogmios, &context.protocol_params)
            .await?;

        let cbor_hex = tx.cbor_hex();

        let (_signed, _res) = sign_and_submit_tx(context, tx).await?;

        Ok(())
    }

    #[test_context(DevnetContext)]
    #[tokio::test]
    async fn spend_specific_output(context: &mut DevnetContext) -> anyhow::Result<()> {
        let _lock = TestLock::wait_and_lock(&LOCK);

        let change_address = context.wallet.address().clone();

        let (_signed_tx, output_pointer) = {
            let tx = TxBuilder::new(context.network_id)
                .change_address(Address::Shelley(change_address.clone()))
                .add_output(Output::new(
                    Address::Shelley(context.wallet.address().clone()),
                    42_000_000,
                ))
                .build(&context.ogmios, &context.protocol_params)
                .await?;

            let (signed, _res) = sign_and_submit_tx(context, tx).await?;

            let output_idx = signed
                .body()
                .outputs
                .as_ref()
                .context("no outputs in first transaction")?
                .iter()
                .position(|output| output.lovelace == 42_000_000)
                .context("output with 42 ada not found")?;

            let output_pointer: hydrant::primitives::TxOutputPointer =
                hydrant::primitives::TxOutputPointer::new(signed.hash()?.0.into(), output_idx);

            util::wait_n_slots(context, 1).await?;
            (signed, output_pointer)
        };

        let (_signed_tx, _res) = {
            let tx = TxBuilder::new(context.network_id)
                .change_address(Address::Shelley(change_address.clone()))
                .add_input(output_pointer.into())
                .build(&context.ogmios, &context.protocol_params)
                .await?;

            sign_and_submit_tx(context, tx).await?
        };

        Ok(())
    }
}
