pub fn calculate_fee(tx: StagingTransaction, config: &Config) -> Result<u64> {
    let signed_tx = tx
        .build_conway_raw()?
        .sign(config.wallet_payment_key.to_ed25519_private_key())?;

    let params = match config.network.into() {
        MultiEraProtocolParameters::Conway(params) => params,
        _ => todo!("Implement support for non-conway protocol parameters in fee computation"),
    };

    // TODO: calculate the fee for the script the simple way by setting a maximum mem and cpu usage
    // params.execution_costs.mem_price
    // params.execution_costs.cpu_price

    let coefficient = params.minfee_a;
    let constant = params.minfee_b;

    Ok((coefficient * (signed_tx.tx_bytes.0.len() as u32) + constant).into())
}
