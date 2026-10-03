use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Transaction {
    pub id: String,
    pub sender: String,
    pub recipient: String,
    pub amount: u64,
    pub nonce: u64,
}

impl Transaction {
    pub fn new(
        id: impl Into<String>,
        sender: impl Into<String>,
        recipient: impl Into<String>,
        amount: u64,
        nonce: u64,
    ) -> Self {
        Self {
            id: id.into(),
            sender: sender.into(),
            recipient: recipient.into(),
            amount,
            nonce,
        }
    }
}
