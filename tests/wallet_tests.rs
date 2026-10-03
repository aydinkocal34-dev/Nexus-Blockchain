use nexus_blockchain::{Transaction, Wallet};

#[test]
fn wallet_signs_and_verifies_message() {
    let wallet = Wallet::new();
    let message = b"nexus test message";
    let signature = wallet.sign(message);

    assert!(wallet.verify(message, &signature));
    assert!(!wallet.verify(b"tampered message", &signature));
}

#[test]
fn signed_transaction_verifies() {
    let wallet = Wallet::new();
    let public_key = wallet.public_key_bytes();

    let unsigned = Transaction::new("tx-1", wallet.address(), "nexus-recipient", 100, 1);
    let signature = wallet.sign(&unsigned.signing_bytes());

    let signed = unsigned.with_signature(public_key, signature);

    assert!(signed.verify_signature());
    assert_eq!(signed.id_hash().len(), 64);
}

#[test]
fn transaction_signature_breaks_after_tampering() {
    let wallet = Wallet::new();
    let unsigned = Transaction::new("tx-2", wallet.address(), "nexus-recipient", 100, 1);
    let signature = wallet.sign(&unsigned.signing_bytes());
    let mut signed = unsigned.with_signature(wallet.public_key_bytes(), signature);

    assert!(signed.verify_signature());

    signed.amount = 101;

    assert!(!signed.verify_signature());
}
