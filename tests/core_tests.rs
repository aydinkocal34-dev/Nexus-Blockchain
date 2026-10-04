use nexus_blockchain::{Blockchain, Transaction, Wallet};

fn signed_transaction(wallet: &Wallet, recipient: &str) -> Transaction {
    let tx = Transaction::new(
        "tx-core-1",
        wallet.address(),
        recipient,
        10,
        1,
    );
    let signature = wallet.sign(&tx.signing_bytes());
    tx.with_signature(wallet.public_key_bytes(), signature)
}

#[test]
fn creates_valid_genesis_chain() {
    let chain = Blockchain::new(1_760_000_000);
    assert_eq!(chain.chain.len(), 1);
    assert!(chain.is_valid());
}

#[test]
fn executes_and_validates_block() {
    let sender = Wallet::new();
    let recipient = Wallet::new();
    let mut chain = Blockchain::new(1_760_000_000);
    let tx = signed_transaction(&sender, &recipient.address());

    assert_eq!(chain.state.credit(sender.address(), 20), Ok(()));
    assert!(chain.execute_block(1_760_000_001, vec![tx]).is_ok());
    assert_eq!(chain.chain.len(), 2);
    assert!(chain.is_valid());
}

#[test]
fn detects_tampering() {
    let sender = Wallet::new();
    let recipient = Wallet::new();
    let mut chain = Blockchain::new(1_760_000_000);
    let tx = signed_transaction(&sender, &recipient.address());

    assert_eq!(chain.state.credit(sender.address(), 20), Ok(()));
    assert!(chain.execute_block(1_760_000_001, vec![tx]).is_ok());

    chain.chain[1].transactions[0].amount = 999;
    assert!(!chain.is_valid());
}

#[test]
fn rejects_empty_block() {
    let mut chain = Blockchain::new(1_760_000_000);
    assert_eq!(
        chain.execute_block(1_760_000_001, Vec::new()),
        Err("block must contain at least one transaction")
    );
    assert_eq!(chain.chain.len(), 1);
}
