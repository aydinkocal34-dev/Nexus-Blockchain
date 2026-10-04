use serde::{Deserialize, Serialize};

use crate::{Block, Transaction};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PeerInfo {
    pub node_id: String,
    pub address: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum NetworkMessage {
    Hello(PeerInfo),
    Ping { nonce: u64 },
    Pong { nonce: u64 },
    GetLatestBlock,
    LatestBlock(Block),
    GetTransactions,
    Transactions(Vec<Transaction>),
    SubmitTransaction(Transaction),
}
