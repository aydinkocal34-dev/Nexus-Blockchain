use nexus_blockchain::config::{GenesisConfig, Network, MAINNET_CHAIN_ID, TESTNET_CHAIN_ID};

#[test]
fn network_ids_are_distinct() {
    assert_ne!(Network::Mainnet.chain_id(), Network::Testnet.chain_id());
    assert_eq!(Network::Mainnet.chain_id(), MAINNET_CHAIN_ID);
    assert_eq!(Network::Testnet.chain_id(), TESTNET_CHAIN_ID);
}

#[test]
fn genesis_config_is_validated() {
    let config = GenesisConfig::new(Network::Testnet, 1, "genesis").unwrap();
    assert!(config.validate().is_ok());
    assert!(GenesisConfig::new(Network::Mainnet, 0, "genesis").is_err());
}
