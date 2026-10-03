use nexus_blockchain::{Node, Transaction, Wallet};

fn signed_transaction(wallet: &Wallet, recipient: &str, amount: u64, nonce: u64) -> Transaction {
    let tx = Transaction::new(
        format!("tx-{nonce}"),
        wallet.address(),
        recipient,
        amount,
        nonce,
    );
    let signature = wallet.sign(&tx.signing_bytes());
    tx.with_signature(wallet.public_key_bytes(), signature)
}

#[test]
fn node_submits_and_mines_transaction() {
    let sender = Wallet::new();
    let recipient = Wallet::new();
    let mut node = Node::new(1);

    node.blockchain.state.credit(sender.address(), 100);

    let tx = signed_transaction(&sender, &recipient.address(), 35, 1);
    assert!(node.submit_transaction(tx).is_ok());
    assert_eq!(node.mempool.len(), 1);

    assert_eq!(node.mine_pending(2), Ok(1));
    assert!(node.mempool.is_empty());
    assert_eq!(node.blockchain.balance_of(&sender.address()), 65);
    assert_eq!(node.blockchain.balance_of(&recipient.address()), 35);
    assert!(node.blockchain.is_valid());
}

#[test]
fn failed_mining_restores_mempool() {
    let sender = Wallet::new();
    let recipient = Wallet::new();
    let mut node = Node::new(1);

    node.blockchain.state.credit(sender.address(), 10);

    let tx = signed_transaction(&sender, &recipient.address(), 20, 1);
    assert!(node.submit_transaction(tx).is_ok());

    assert!(node.mine_pending(2).is_err());
    assert_eq!(node.mempool.len(), 1);
    assert_eq!(node.blockchain.chain.len(), 1);
}
