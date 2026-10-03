use nexus_blockchain::{Blockchain, Transaction, Wallet};

fn signed_transaction(
    wallet: &Wallet,
    recipient: &str,
    amount: u64,
    nonce: u64,
) -> Transaction {
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
fn valid_transaction_is_committed_to_block_and_state() {
    let sender = Wallet::new();
    let recipient = Wallet::new();
    let mut blockchain = Blockchain::new(1);

    blockchain.state.credit(sender.address(), 100);

    let tx = signed_transaction(&sender, &recipient.address(), 40, 1);
    assert!(blockchain.execute_block(2, vec![tx]).is_ok());

    assert_eq!(blockchain.chain.len(), 2);
    assert_eq!(blockchain.balance_of(&sender.address()), 60);
    assert_eq!(blockchain.balance_of(&recipient.address()), 40);
    assert_eq!(blockchain.nonce_of(&sender.address()), 1);
    assert!(blockchain.is_valid());
}

#[test]
fn invalid_block_does_not_change_chain_or_state() {
    let sender = Wallet::new();
    let recipient = Wallet::new();
    let mut blockchain = Blockchain::new(1);

    blockchain.state.credit(sender.address(), 10);

    let tx = signed_transaction(&sender, &recipient.address(), 20, 1);
    assert!(blockchain.execute_block(2, vec![tx]).is_err());

    assert_eq!(blockchain.chain.len(), 1);
    assert_eq!(blockchain.balance_of(&sender.address()), 10);
    assert_eq!(blockchain.balance_of(&recipient.address()), 0);
}

#[test]
fn multiple_transactions_execute_in_order() {
    let sender = Wallet::new();
    let first_recipient = Wallet::new();
    let second_recipient = Wallet::new();
    let mut blockchain = Blockchain::new(1);

    blockchain.state.credit(sender.address(), 100);

    let first = signed_transaction(&sender, &first_recipient.address(), 30, 1);
    let second = signed_transaction(&sender, &second_recipient.address(), 20, 2);

    assert!(blockchain.execute_block(2, vec![first, second]).is_ok());
    assert_eq!(blockchain.balance_of(&sender.address()), 50);
    assert_eq!(blockchain.balance_of(&first_recipient.address()), 30);
    assert_eq!(blockchain.balance_of(&second_recipient.address()), 20);
    assert_eq!(blockchain.nonce_of(&sender.address()), 2);
}
