use nexus_blockchain::economics::{
    fee_split, validate_genesis_allocations, validate_supply, GenesisAllocation, MAX_SUPPLY,
};

#[test]
fn supply_is_hard_capped() {
    assert!(validate_supply(MAX_SUPPLY).is_ok());
    assert!(validate_supply(MAX_SUPPLY + 1).is_err());
}

#[test]
fn genesis_cannot_exceed_maximum_supply() {
    assert!(validate_genesis_allocations(&[GenesisAllocation {
        address: "address".into(),
        amount: MAX_SUPPLY,
    }]).is_ok());
    assert!(validate_genesis_allocations(&[GenesisAllocation {
        address: "address".into(),
        amount: MAX_SUPPLY,
    }, GenesisAllocation {
        address: "address2".into(),
        amount: 1,
    }]).is_err());
}

#[test]
fn fee_split_is_bounded() {
    assert_eq!(fee_split(10_000, 1_000, 500).unwrap(), (1_000, 500, 8_500));
    assert!(fee_split(1, 10_001, 0).is_err());
}
