use nexus_blockchain::Wallet;

#[test]
fn encrypted_wallet_round_trip_preserves_address() {
    let wallet = Wallet::new();
    let data = wallet.encrypt_keystore(b"strong-test-password").unwrap();
    let restored = Wallet::from_encrypted_keystore(&data, b"strong-test-password").unwrap();
    assert_eq!(wallet.address(), restored.address());
    assert!(Wallet::from_encrypted_keystore(&data, b"wrong").is_err());
}
