use pallas_network::miniprotocols::localtxsubmission::GenericClient;
use pallas_network::multiplexer::{Bearer, Plexer};
use pallas_primitives::conway::Tx;

pub async fn submit_transaction(tx: Tx) -> anyhow::Result<()> {
    let bearer = Bearer::connect_tcp("mainnet-cardano-node:3001").await?;
    let mut plexer = Plexer::new(bearer);
    let channel =
        plexer.subscribe_client(pallas_network::miniprotocols::PROTOCOL_N2C_TX_SUBMISSION);
    let mut client: GenericClient<Tx, ()> = GenericClient::new(channel);

    client.submit_tx(tx).await?;

    Ok(())
}
