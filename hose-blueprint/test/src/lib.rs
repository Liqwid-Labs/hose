#[cfg(test)]
mod tests {
    use std::str::FromStr;
    use pallas::codec::utils::{AnyUInt, Bytes};

    pub mod example {
        use hose_blueprint::blueprint;
        blueprint!("hose-blueprint/test/static/plutus.json");
    }

    #[test]
    fn test_generate_cbor_struct() {
        use example::root::DelegatedTo;
        use example::root::cardano::address::Credential;

        let real_datum = 
            String::from("9f1a5f0929eed8799f581c99669d1885f1b6f10d3447647403be44beacedab9039a4ad9394e492ffd8799fd8799f581cf40f4d6270c46f4bc324ddbe8399a0fe463b598e6abdf6076b8fe03affff80ff");

        let bytes = hex::decode(&real_datum).unwrap();
        let decoded_datum: example::root::Datum = minicbor::decode(&bytes).unwrap();

        let example::root::Datum(staked_amount, owner, delegated_to, proposal_locks) = decoded_datum.clone();

        assert_eq!(staked_amount, AnyUInt::U32(1_594_436_078u32));
        assert_eq!(
            owner,
            Credential::VerificationKey(
                Bytes::from_str("99669d1885f1b6f10d3447647403be44beacedab9039a4ad9394e492")
                    .unwrap()
            )
        );
        assert_eq!(
            delegated_to,
            DelegatedTo::Some(
                Credential::VerificationKey(
                    Bytes::from_str("f40f4d6270c46f4bc324ddbe8399a0fe463b598e6abdf6076b8fe03a")
                    .unwrap()
                )
            )
        );
        assert_eq!(proposal_locks, vec![]);

        // Check if encoding it backwards is the same as the original
        let back_to_bytes = hex::encode(minicbor::to_vec(decoded_datum).unwrap());

        assert_eq!(back_to_bytes, real_datum);
    }
}
