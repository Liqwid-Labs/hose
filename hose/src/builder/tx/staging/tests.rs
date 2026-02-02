use pallas::ledger::addresses::{
    Address as PallasAddress, Network, ShelleyAddress, ShelleyDelegationPart, ShelleyPaymentPart,
};
use pallas::ledger::primitives::Fragment;
use pallas::ledger::primitives::conway::{Certificate as PallasCertificate, RedeemerTag, Tx};

use super::StagingTransaction;
use crate::primitives::{Certificate, Hash, Output, RewardAccount};

fn dummy_output() -> Output {
    let payment_hash = Hash([1u8; 28]);
    let address = PallasAddress::Shelley(ShelleyAddress::new(
        Network::Testnet,
        ShelleyPaymentPart::Key(payment_hash.into()),
        ShelleyDelegationPart::Null,
    ));
    Output::new(address, 1)
}

#[test]
fn withdrawals_last_write_wins() {
    let script_hash = Hash([2u8; 28]);
    let reward_account = RewardAccount::from_script_hash(Network::Testnet, script_hash);
    let tx = StagingTransaction::new()
        .withdrawal(reward_account.clone(), 1)
        .withdrawal(reward_account.clone(), 2);

    assert_eq!(tx.withdrawals.get(&reward_account), Some(&2));
}

#[test]
fn build_includes_registration_certificate_and_redeemer() {
    let script_hash = Hash([3u8; 28]);
    let tx = StagingTransaction::new()
        .network_id(0)
        .fee(0)
        .output(dummy_output())
        .add_certificate(Certificate::StakeRegistrationScript {
            script_hash,
            deposit: Some(2),
        })
        .add_cert_redeemer(script_hash, vec![0u8], None);

    let built = tx.build_conway(None, 0).expect("build conway");
    let decoded = Tx::decode_fragment(&built.bytes).expect("decode tx");

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
        _ => panic!("unexpected redeemer format"),
    };
    assert!(
        redeemers
            .iter()
            .any(|r| r.tag == RedeemerTag::Cert && r.index == 0)
    );
}

#[test]
fn build_includes_deregistration_certificate_and_redeemer() {
    let script_hash = Hash([4u8; 28]);
    let tx = StagingTransaction::new()
        .network_id(0)
        .fee(0)
        .output(dummy_output())
        .add_certificate(Certificate::StakeDeregistrationScript {
            script_hash,
            deposit: Some(2),
        })
        .add_cert_redeemer(script_hash, vec![0u8], None);

    let built = tx.build_conway(None, 0).expect("build conway");
    let decoded = Tx::decode_fragment(&built.bytes).expect("decode tx");

    let certs = decoded
        .transaction_body
        .certificates
        .as_ref()
        .expect("certificates missing");
    let certs_vec: Vec<PallasCertificate> = certs.iter().cloned().collect();
    assert!(matches!(certs_vec[0], PallasCertificate::UnReg(_, 2)));

    let redeemers = decoded
        .transaction_witness_set
        .redeemer
        .as_ref()
        .expect("redeemers missing");
    let redeemers = match &**redeemers {
        pallas::ledger::primitives::conway::Redeemers::List(list) => list,
        _ => panic!("unexpected redeemer format"),
    };
    assert!(
        redeemers
            .iter()
            .any(|r| r.tag == RedeemerTag::Cert && r.index == 0)
    );
}

#[test]
fn build_includes_withdrawal_and_reward_redeemer() {
    let script_hash = Hash([5u8; 28]);
    let reward_account = RewardAccount::from_script_hash(Network::Testnet, script_hash);
    let tx = StagingTransaction::new()
        .network_id(0)
        .fee(0)
        .output(dummy_output())
        .withdrawal(reward_account.clone(), 0)
        .add_reward_redeemer(reward_account.clone(), vec![0u8], None);

    let built = tx.build_conway(None, 0).expect("build conway");
    let decoded = Tx::decode_fragment(&built.bytes).expect("decode tx");

    let withdrawals = decoded
        .transaction_body
        .withdrawals
        .as_ref()
        .expect("withdrawals missing");
    let withdrawals_vec: Vec<(pallas::codec::utils::Bytes, u64)> =
        withdrawals.iter().map(|(k, v)| (k.clone(), *v)).collect();
    assert_eq!(withdrawals_vec.len(), 1);
    let expected_account: pallas::codec::utils::Bytes = reward_account.into();
    assert_eq!(withdrawals_vec[0], (expected_account, 0));

    let redeemers = decoded
        .transaction_witness_set
        .redeemer
        .as_ref()
        .expect("redeemers missing");
    let redeemers = match &**redeemers {
        pallas::ledger::primitives::conway::Redeemers::List(list) => list,
        _ => panic!("unexpected redeemer format"),
    };
    assert!(
        redeemers
            .iter()
            .any(|r| r.tag == RedeemerTag::Reward && r.index == 0)
    );
}
