use std::collections::HashMap;

use crate::transaction::Transaction;

#[derive(Debug, Default)]
pub struct Mempool {
    transactions: HashMap<String, Transaction>,
}

impl Mempool {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn submit(&mut self, transaction: Transaction) -> Result<(), &'static str> {
        if !transaction.verify_signature() {
            return Err("invalid transaction signature");
        }

        if transaction.id.is_empty() || transaction.amount == 0 {
            return Err("invalid transaction fields");
        }

        if self.transactions.contains_key(&transaction.id) {
            return Err("duplicate transaction");
        }

        self.transactions.insert(transaction.id.clone(), transaction);
        Ok(())
    }

    pub fn contains(&self, transaction_id: &str) -> bool {
        self.transactions.contains_key(transaction_id)
    }

    pub fn len(&self) -> usize {
        self.transactions.len()
    }

    pub fn is_empty(&self) -> bool {
        self.transactions.is_empty()
    }

    pub fn transactions(&self) -> impl Iterator<Item = &Transaction> {
        self.transactions.values()
    }

    pub fn take_all(&mut self) -> Vec<Transaction> {
        std::mem::take(&mut self.transactions)
            .into_values()
            .collect()
    }
}
