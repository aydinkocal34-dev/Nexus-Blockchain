use nexus_blockchain::{Mempool, Transaction, Wallet};

fn signed_transaction(wallet: &Wallet, id: &str, amount: u64) -> Transaction {
    let unsigned = Transaction::new(id, wallet.address(), "nexus-recipient", amount, 1)
        .with_signature(wallet.public_key_bytes(), [0; 64]);
    let signature = wallet.sign(&unsigned.signing_bytes());
    unsigned.with_signature(wallet.public_key_bytes(), signature)
}

#[test]
fn valid_transaction_enters_mempool() {
    let wallet = Wallet::new();
    let mut mempool = Mempool::new();

    let tx = signed_transaction(&wallet, "tx-1", 100);

    assert!(mempool.submit(tx).is_ok());
    assert_eq!(mempool.len(), 1);
}

#[test]
fn invalid_signature_is_rejected() {
    let wallet = Wallet::new();
    let mut mempool = Mempool::new();

    let mut tx = signed_transaction(&wallet, "tx-2", 100);
    tx.amount = 999;

    assert!(mempool.submit(tx).is_err());
    assert!(mempool.is_empty());
}

#[test]
fn duplicate_transaction_is_rejected() {
    let wallet = Wallet::new();
    let mut mempool = Mempool::new();

    let tx = signed_transaction(&wallet, "tx-3", 100);
    assert!(mempool.submit(tx.clone()).is_ok());
    assert!(mempool.submit(tx).is_err());
    assert_eq!(mempool.len(), 1);
}
