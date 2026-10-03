use std::collections::HashMap;

use crate::transaction::Transaction;
use crate::wallet::address_from_public_key;

#[derive(Debug, Default, Clone)]
pub struct State {
    balances: HashMap<String, u64>,
    nonces: HashMap<String, u64>,
}

impl State {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn balance_of(&self, address: &str) -> u64 {
        self.balances.get(address).copied().unwrap_or(0)
    }

    pub fn nonce_of(&self, address: &str) -> u64 {
        self.nonces.get(address).copied().unwrap_or(0)
    }

    pub fn credit(&mut self, address: impl Into<String>, amount: u64) {
        let address = address.into();
        let balance = self.balances.entry(address).or_default();
        *balance = balance.saturating_add(amount);
    }

    pub fn validate_transaction(&self, transaction: &Transaction) -> Result<(), &'static str> {
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

        let expected_nonce = self.nonce_of(&transaction.sender) + 1;
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

        let sender_balance = self
            .balances
            .get_mut(&transaction.sender)
            .expect("validated sender must have a balance");
        *sender_balance -= transaction.amount;

        let recipient_balance = self.balances.entry(transaction.recipient.clone()).or_default();
        *recipient_balance = recipient_balance
            .checked_add(transaction.amount)
            .ok_or("recipient balance overflow")?;

        self.nonces
            .insert(transaction.sender.clone(), transaction.nonce);

        Ok(())
    }
}
