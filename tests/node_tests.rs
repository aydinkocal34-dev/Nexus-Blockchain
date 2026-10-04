use nexus_blockchain::{Node, Transaction, Wallet};

fn signed_transaction(wallet: &Wallet, id: &str, recipient: &str, amount: u64, nonce: u64) -> Transaction {
    let tx = Transaction::new(id, wallet.address(), recipient, amount, nonce);
    let signature = wallet.sign(&tx.signing_bytes());
    tx.with_signature(wallet.public_key_bytes(), signature)
}

#[test]
fn node_submits_and_mines_transaction() {
    let sender = Wallet::new();
    let recipient = Wallet::new();
    let mut node = Node::new(1);
    assert_eq!(node.blockchain.state.credit(sender.address(), 100), Ok(()));

    let tx = signed_transaction(&sender, "tx-1", &recipient.address(), 35, 1);
    assert!(node.submit_transaction(tx).is_ok());
    assert_eq!(node.mempool.len(), 1);

    assert_eq!(node.mine_pending(2), Ok(1));
    assert!(node.mempool.is_empty());
    assert_eq!(node.blockchain.balance_of(&sender.address()), 65);
    assert_eq!(node.blockchain.balance_of(&recipient.address()), 35);
    assert!(node.blockchain.is_valid());
}

#[test]
fn node_rejects_insufficient_balance_before_mempool() {
    let sender = Wallet::new();
    let recipient = Wallet::new();
    let mut node = Node::new(1);
    assert_eq!(node.blockchain.state.credit(sender.address(), 10), Ok(()));

    let tx = signed_transaction(&sender, "tx-2", &recipient.address(), 20, 1);
    assert_eq!(node.submit_transaction(tx), Err("insufficient balance"));
    assert!(node.mempool.is_empty());
}

#[test]
fn node_rejects_wrong_nonce_before_mempool() {
    let sender = Wallet::new();
    let recipient = Wallet::new();
    let mut node = Node::new(1);
    assert_eq!(node.blockchain.state.credit(sender.address(), 100), Ok(()));

    let tx = signed_transaction(&sender, "tx-3", &recipient.address(), 20, 2);
    assert_eq!(node.submit_transaction(tx), Err("invalid nonce"));
    assert!(node.mempool.is_empty());
}

#[test]
fn node_accepts_sequential_pending_transactions() {
    let sender = Wallet::new();
    let recipient = Wallet::new();
    let mut node = Node::new(1);
    assert_eq!(node.blockchain.state.credit(sender.address(), 100), Ok(()));

    let first = signed_transaction(&sender, "tx-4", &recipient.address(), 30, 1);
    let second = signed_transaction(&sender, "tx-5", &recipient.address(), 20, 2);

    assert!(node.submit_transaction(first).is_ok());
    assert!(node.submit_transaction(second).is_ok());
    assert_eq!(node.mine_pending(2), Ok(2));
    assert_eq!(node.blockchain.balance_of(&sender.address()), 50);
}

#[test]
fn failed_mining_restores_mempool() {
    let sender = Wallet::new();
    let recipient = Wallet::new();
    let mut node = Node::new(1);

    let tx = signed_transaction(&sender, "tx-6", &recipient.address(), 20, 1);
    node.mempool.submit(tx).expect("basic mempool admission should succeed");

    assert!(node.mine_pending(2).is_err());
    assert_eq!(node.mempool.len(), 1);
    assert_eq!(node.blockchain.chain.len(), 1);
}
