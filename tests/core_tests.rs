use nexus_blockchain::{Blockchain, Transaction};

#[test]
fn creates_valid_genesis_chain() {
    let chain = Blockchain::new(1_760_000_000);
    assert_eq!(chain.chain.len(), 1);
    assert!(chain.is_valid());
}

#[test]
fn appends_and_validates_block() {
    let mut chain = Blockchain::new(1_760_000_000);
    chain.add_block(1_760_000_001, vec![Transaction::new("tx-1", "alice", "bob", 100, 1)]);
    assert_eq!(chain.chain.len(), 2);
    assert!(chain.is_valid());
}

#[test]
fn detects_tampering() {
    let mut chain = Blockchain::new(1_760_000_000);
    chain.add_block(1_760_000_001, vec![Transaction::new("tx-1", "alice", "bob", 100, 1)]);
    chain.chain[1].transactions[0].amount = 999;
    assert!(!chain.is_valid());
}
