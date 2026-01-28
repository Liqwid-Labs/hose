use pallas::ledger::addresses::{
    Address, Network, ShelleyAddress, ShelleyDelegationPart, ShelleyPaymentPart,
};
use pallas::ledger::primitives::conway::{Certificate as PallasCertificate, RedeemerTag, Tx};
use pallas::ledger::primitives::Fragment;

use super::StagingTransaction;
use crate::primitives::{Certificate, Hash, Output};

fn mock_output() -> Output {
    let payment_hash = Hash([1u8; 28]);
    let address = Address::Shelley(ShelleyAddress::new(
        Network::Testnet,
        ShelleyPaymentPart::Key(payment_hash.into()),
        ShelleyDelegationPart::Null,
    ));
    Output::new(address, 1)
}

#[test]
fn build_includes_registration_certificate_and_redeemer() {
    let script_hash = Hash([3u8; 28]);
    let tx = StagingTransaction::new()
        .network_id(5)
        .fee(0)
        .output(mock_output())
        .add_certificate(Certificate::StakeRegistrationScript {
            script_hash,
            deposit: 2,
        })
        .add_cert_redeemer(script_hash, vec![0u8], None);

    let built = tx.build_conway(None).expect("build conway failed");

    // now check that the certificates and registration redeemer is in there.
    let decoded = Tx::decode_fragment(&built.bytes).expect("could not decode tx");

    let certs = decoded
        .transaction_body
        .certificates
        .as_ref()
        .expect("certificates missing");
    let certs_vec: Vec<PallasCertificate> = certs.iter().cloned().collect();
    assert!(matches!(certs_vec[0], PallasCertificate::Reg(_, 2)));

    let redeemers = decoded
        .transaction_witness_set
        .redeemer
        .as_ref()
        .expect("redeemers missing");
    let redeemers = match &**redeemers {
        pallas::ledger::primitives::conway::Redeemers::List(list) => list,
        _ => panic!("unexpected redeemer"),
    };
    assert!(redeemers
        .iter()
        .any(|r| r.tag == RedeemerTag::Cert && r.index == 0));
}
