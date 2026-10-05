use nexus_blockchain::config::{GenesisConfig, Network};

#[test]
fn mainnet_template_cannot_pass_release_gate() {
    let config = GenesisConfig {
        network: Network::Mainnet,
        protocol_version: 1,
        genesis_timestamp: 1,
        genesis_hash: "REPLACE_WITH_AUDITED_MAINNET_GENESIS_HASH".into(),
        max_supply: 100_000_000,
    };
    assert!(config.validate_for_mainnet().is_err());
}

#[test]
fn audited_shape_can_pass_mainnet_release_gate() {
    let config = GenesisConfig {
        network: Network::Mainnet,
        protocol_version: 1,
        genesis_timestamp: 1,
        genesis_hash: "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef".into(),
        max_supply: 100_000_000,
    };
    assert!(config.validate_for_mainnet().is_ok());
}
