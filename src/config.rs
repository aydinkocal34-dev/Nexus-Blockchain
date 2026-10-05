use crate::economics::MAX_SUPPLY;
use serde::{Deserialize, Serialize};

pub const PROTOCOL_VERSION: u16 = 1;
pub const MAINNET_CHAIN_ID: u64 = 1_000_001;
pub const TESTNET_CHAIN_ID: u64 = 1_000_002;
pub const DEVNET_CHAIN_ID: u64 = 1_000_003;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum Network {
    Devnet,
    Testnet,
    Mainnet,
}

impl Network {
    pub fn chain_id(self) -> u64 {
        match self {
            Self::Devnet => DEVNET_CHAIN_ID,
            Self::Testnet => TESTNET_CHAIN_ID,
            Self::Mainnet => MAINNET_CHAIN_ID,
        }
    }

    pub fn is_production(self) -> bool {
        matches!(self, Self::Mainnet)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct GenesisConfig {
    pub network: Network,
    pub protocol_version: u16,
    pub genesis_timestamp: u64,
    pub genesis_hash: String,
    pub max_supply: u64,
}

impl GenesisConfig {
    pub fn new(network: Network, genesis_timestamp: u64, genesis_hash: impl Into<String>) -> Result<Self, &'static str> {
        let genesis_hash = genesis_hash.into();
        if genesis_hash.is_empty() { return Err("genesis hash must not be empty"); }
        if genesis_timestamp == 0 { return Err("genesis timestamp must be greater than zero"); }
        Ok(Self {
            network,
            protocol_version: PROTOCOL_VERSION,
            genesis_timestamp,
            genesis_hash,
            max_supply: MAX_SUPPLY,
        })
    }

    pub fn validate(&self) -> Result<(), &'static str> {
        if self.protocol_version != PROTOCOL_VERSION { return Err("unsupported protocol version"); }
        if self.genesis_timestamp == 0 { return Err("invalid genesis timestamp"); }
        if self.genesis_hash.is_empty() { return Err("missing genesis hash"); }
        if self.max_supply != MAX_SUPPLY { return Err("invalid maximum supply"); }
        Ok(())
    }
}
