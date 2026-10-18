use nexus_blockchain::{State, Transaction, Wallet};

fn signed_transaction(wallet: &Wallet, recipient: &str, amount: u64, nonce: u64) -> Transaction {
    let unsigned = Transaction::new(
        format!("tx-{nonce}"),
        wallet.address(),
        recipient,
        amount,
        nonce,
    )
    .with_signature(wallet.public_key_bytes(), [0; 64]);
    let signature = wallet.sign(&unsigned.signing_bytes());
    unsigned.with_signature(wallet.public_key_bytes(), signature)
}

#[test]
fn applies_transaction_and_updates_balances() {
    let sender = Wallet::new();
    let recipient = Wallet::new();
    let mut state = State::new();

    state.credit(sender.address(), 100).unwrap();

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

    state.credit(sender.address(), 20).unwrap();

    let tx = signed_transaction(&sender, &recipient.address(), 21, 1);
    assert_eq!(state.apply_transaction(&tx), Err("insufficient balance"));
}

#[test]
fn rejects_replayed_nonce() {
    let sender = Wallet::new();
    let recipient = Wallet::new();
    let mut state = State::new();

    state.credit(sender.address(), 100).unwrap();

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

    state.credit(sender.address(), 100).unwrap();

    let unsigned = Transaction::new(
        "identity-check",
        other.address(),
        recipient.address(),
        10,
        1,
    )
    .with_signature(sender.public_key_bytes(), [0; 64]);
    let signature = sender.sign(&unsigned.signing_bytes());
    let signed = unsigned.with_signature(sender.public_key_bytes(), signature);

    assert_eq!(
        state.apply_transaction(&signed),
        Err("sender does not match public key")
    );
}

#[test]
fn recipient_overflow_does_not_partially_mutate_state() {
    let sender = Wallet::new();
    let recipient = Wallet::new();
    let mut state = State::new();
    assert_eq!(state.credit(sender.address(), 10), Ok(()));
    assert_eq!(state.credit(recipient.address(), u64::MAX), Ok(()));
    let tx = signed_transaction(&sender, &recipient.address(), 10, 1);
    assert_eq!(
        state.apply_transaction(&tx),
        Err("recipient balance overflow")
    );
    assert_eq!(state.balance_of(&sender.address()), 10);
    assert_eq!(state.balance_of(&recipient.address()), u64::MAX);
    assert_eq!(state.nonce_of(&sender.address()), 0);
}

#[test]
fn credit_overflow_does_not_change_balance() {
    let wallet = Wallet::new();
    let mut state = State::new();
    assert_eq!(state.credit(wallet.address(), u64::MAX), Ok(()));
    assert_eq!(state.credit(wallet.address(), 1), Err("balance overflow"));
    assert_eq!(state.balance_of(&wallet.address()), u64::MAX);
}

#[test]
fn rejects_transaction_signed_for_another_chain() {
    use nexus_blockchain::config::{MAINNET_CHAIN_ID, TESTNET_CHAIN_ID};

    let sender = Wallet::new();
    let recipient = Wallet::new();
    let mut mainnet_state = State::new_for_chain(MAINNET_CHAIN_ID);
    mainnet_state.credit(sender.address(), 100).unwrap();

    let unsigned = Transaction::new_for_chain(
        "cross-chain-replay",
        sender.address(),
        recipient.address(),
        10,
        1,
        TESTNET_CHAIN_ID,
    )
    .with_signature(sender.public_key_bytes(), [0; 64]);
    let signature = sender.sign(&unsigned.signing_bytes());
    let testnet_transaction = unsigned.with_signature(sender.public_key_bytes(), signature);

    assert_eq!(
        mainnet_state.apply_transaction(&testnet_transaction),
        Err("transaction chain ID mismatch")
    );
    assert_eq!(mainnet_state.balance_of(&sender.address()), 100);
    assert_eq!(mainnet_state.nonce_of(&sender.address()), 0);
}

#[test]
fn chain_id_is_covered_by_transaction_signature() {
    use nexus_blockchain::config::{DEVNET_CHAIN_ID, MAINNET_CHAIN_ID};

    let sender = Wallet::new();
    let unsigned = Transaction::new_for_chain(
        "chain-bound-signature",
        sender.address(),
        "recipient",
        1,
        1,
        DEVNET_CHAIN_ID,
    )
    .with_signature(sender.public_key_bytes(), [0; 64]);
    let signature = sender.sign(&unsigned.signing_bytes());
    let mut signed = unsigned.with_signature(sender.public_key_bytes(), signature);
    assert!(signed.verify_signature());

    signed.chain_id = MAINNET_CHAIN_ID;
    assert!(!signed.verify_signature());
}
