use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::config::DEVNET_CHAIN_ID;
use crate::transaction::Transaction;
use crate::wallet::address_from_public_key;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct State {
    #[serde(default = "default_chain_id")]
    chain_id: u64,
    balances: HashMap<String, u64>,
    nonces: HashMap<String, u64>,
}

fn default_chain_id() -> u64 {
    DEVNET_CHAIN_ID
}

impl Default for State {
    fn default() -> Self {
        Self::new()
    }
}

impl State {
    pub fn new() -> Self {
        Self::new_for_chain(DEVNET_CHAIN_ID)
    }

    pub fn new_for_chain(chain_id: u64) -> Self {
        Self {
            chain_id,
            balances: HashMap::new(),
            nonces: HashMap::new(),
        }
    }

    pub fn chain_id(&self) -> u64 {
        self.chain_id
    }
    pub fn balance_of(&self, address: &str) -> u64 {
        self.balances.get(address).copied().unwrap_or(0)
    }
    pub fn nonce_of(&self, address: &str) -> u64 {
        self.nonces.get(address).copied().unwrap_or(0)
    }
    pub fn credit(&mut self, address: impl Into<String>, amount: u64) -> Result<(), &'static str> {
        let address = address.into();
        let current = self.balance_of(&address);
        let next = current.checked_add(amount).ok_or("balance overflow")?;
        self.balances.insert(address, next);
        Ok(())
    }
    pub fn validate_transaction(&self, transaction: &Transaction) -> Result<(), &'static str> {
        if transaction.chain_id != self.chain_id {
            return Err("transaction chain ID mismatch");
        }
        if !transaction.verify_signature() {
            return Err("invalid transaction signature");
        }
        let public_key = transaction.public_key.ok_or("missing public key")?;
        if address_from_public_key(&public_key) != transaction.sender {
            return Err("sender does not match public key");
        }
        if transaction.amount == 0 {
            return Err("amount must be greater than zero");
        }
        let expected_nonce = self
            .nonce_of(&transaction.sender)
            .checked_add(1)
            .ok_or("nonce overflow")?;
        if transaction.nonce != expected_nonce {
            return Err("invalid nonce");
        }
        if self.balance_of(&transaction.sender) < transaction.amount {
            return Err("insufficient balance");
        }
        Ok(())
    }
    pub fn apply_transaction(&mut self, transaction: &Transaction) -> Result<(), &'static str> {
        self.validate_transaction(transaction)?;
        let sender_balance = self.balance_of(&transaction.sender);
        let recipient_balance = self.balance_of(&transaction.recipient);
        let new_recipient_balance = recipient_balance
            .checked_add(transaction.amount)
            .ok_or("recipient balance overflow")?;
        self.balances.insert(
            transaction.sender.clone(),
            sender_balance - transaction.amount,
        );
        self.balances
            .insert(transaction.recipient.clone(), new_recipient_balance);
        self.nonces
            .insert(transaction.sender.clone(), transaction.nonce);
        Ok(())
    }
}
