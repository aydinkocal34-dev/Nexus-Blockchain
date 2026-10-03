use crate::transaction::Transaction;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Block {
    pub index: u64,
    pub timestamp: u64,
    pub previous_hash: String,
    pub transactions: Vec<Transaction>,
    pub hash: String,
}

impl Block {
    pub fn new(index: u64, timestamp: u64, previous_hash: impl Into<String>, transactions: Vec<Transaction>) -> Self {
        let mut block = Self { index, timestamp, previous_hash: previous_hash.into(), transactions, hash: String::new() };
        block.hash = block.calculate_hash();
        block
    }

    pub fn calculate_hash(&self) -> String {
        let payload = serde_json::to_vec(&(self.index, self.timestamp, &self.previous_hash, &self.transactions)).expect("block serialization must succeed");
        let digest = Sha256::digest(payload);
        format!("{digest:x}")
    }

    pub fn is_hash_valid(&self) -> bool { self.hash == self.calculate_hash() }
}
