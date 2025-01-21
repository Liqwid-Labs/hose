#[cfg(test)]
mod tests {
    use blueprint::generate_cbor_struct;
    use pallas::codec::utils::AnyUInt;

    generate_cbor_struct!("../blueprint/plutus.json");

    #[test]
    fn test_generate_cbor_struct() {
        let action_value = liqwid_ActionValue {
            supply_diff: AnyUInt::U64(1),
            q_tokens_diff: AnyUInt::U64(2),
            principal_diff: AnyUInt::U64(3),
            interest_diff: AnyUInt::U64(4),
            extra_interest_repaid: AnyUInt::U64(5),
        };

        let bytes = minicbor::to_vec(&action_value).unwrap();

        let action_value: ActionValue = minicbor::decode(&bytes).unwrap();

        assert_eq!(action_value.supply_diff, AnyUInt::U64(1));
        assert_eq!(action_value.q_tokens_diff, AnyUInt::U64(2));
        assert_eq!(action_value.principal_diff, AnyUInt::U64(3));
        assert_eq!(action_value.interest_diff, AnyUInt::U64(4));
        assert_eq!(action_value.extra_interest_repaid, AnyUInt::U64(5));
    }
}
