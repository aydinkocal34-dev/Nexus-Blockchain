use nexus_blockchain::{State, Transaction, Wallet};

fn signed_transaction(
    wallet: &Wallet,
    recipient: &str,
    amount: u64,
    nonce: u64,
) -> Transaction {
    let tx = Transaction::new(
        wallet.address(),
        wallet.address(),
        recipient,
        amount,
        nonce,
    );
    let signature = wallet.sign(&tx.signing_bytes());
    tx.with_signature(wallet.public_key_bytes(), signature)
}

#[test]
fn applies_transaction_and_updates_balances() {
    let sender = Wallet::new();
    let recipient = Wallet::new();
    let mut state = State::new();

    state.credit(sender.address(), 100);

    let tx = signed_transaction(&sender, &recipient.address(), 40, 1);
    assert!(state.apply_transaction(&tx).is_ok());

    assert_eq!(state.balance_of(&sender.address()), 60);
    assert_eq!(state.balance_of(&recipient.address()), 40);
    assert_eq!(state.nonce_of(&sender.address()), 1);
}

#[test]
fn rejects_insufficient_balance() {
    let sender = Wallet::new();
    let recipient = Wallet::new();
    let mut state = State::new();

    state.credit(sender.address(), 20);

    let tx = signed_transaction(&sender, &recipient.address(), 21, 1);
    assert_eq!(
        state.apply_transaction(&tx),
        Err("insufficient balance")
    );
}

#[test]
fn rejects_replayed_nonce() {
    let sender = Wallet::new();
    let recipient = Wallet::new();
    let mut state = State::new();

    state.credit(sender.address(), 100);

    let first = signed_transaction(&sender, &recipient.address(), 10, 1);
    assert!(state.apply_transaction(&first).is_ok());

    let replay = signed_transaction(&sender, &recipient.address(), 10, 1);
    assert_eq!(state.apply_transaction(&replay), Err("invalid nonce"));
}

#[test]
fn rejects_wrong_sender_identity() {
    let sender = Wallet::new();
    let recipient = Wallet::new();
    let other = Wallet::new();
    let mut state = State::new();

    state.credit(sender.address(), 100);

    let tx = Transaction::new(
        "identity-check",
        other.address(),
        recipient.address(),
        10,
        1,
    );
    let signature = sender.sign(&tx.signing_bytes());
    let signed = tx.with_signature(sender.public_key_bytes(), signature);

    assert_eq!(
        state.apply_transaction(&signed),
        Err("sender does not match public key")
    );
}
