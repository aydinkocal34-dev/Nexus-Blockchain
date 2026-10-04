use nexus_blockchain::{storage::BlockchainStorage, Blockchain};

#[test]
fn blockchain_storage_round_trip() {
    let path = std::env::temp_dir().join(format!("nexus-storage-{}.json", std::process::id()));
    let storage = BlockchainStorage::new(&path);
    let chain = Blockchain::new(1);

    storage.save(&chain).unwrap();
    let loaded = storage.load().unwrap();

    assert_eq!(loaded.chain, chain.chain);
    assert_eq!(loaded.balance_of("missing"), 0);
    assert!(loaded.validate_full().is_ok());

    let _ = std::fs::remove_file(path);
}
