use std::collections::HashMap;

use crate::config::DEVNET_CHAIN_ID;
use crate::state::State;
use crate::transaction::Transaction;

#[derive(Debug)]
pub struct Mempool {
    chain_id: u64,
    transactions: HashMap<String, Transaction>,
}

impl Default for Mempool {
    fn default() -> Self {
        Self::new()
    }
}

impl Mempool {
    pub fn new() -> Self {
        Self::new_for_chain(DEVNET_CHAIN_ID)
    }

    pub fn new_for_chain(chain_id: u64) -> Self {
        Self {
            chain_id,
            transactions: HashMap::new(),
        }
    }

    pub fn submit(&mut self, transaction: Transaction) -> Result<(), &'static str> {
        self.validate_basic(&transaction)?;
        if self.transactions.contains_key(&transaction.id) {
            return Err("duplicate transaction");
        }
        self.transactions
            .insert(transaction.id.clone(), transaction);
        Ok(())
    }

    pub fn submit_with_state(
        &mut self,
        state: &State,
        transaction: Transaction,
    ) -> Result<(), &'static str> {
        if state.chain_id() != self.chain_id {
            return Err("mempool chain ID mismatch");
        }
        self.validate_basic(&transaction)?;
        if self.transactions.contains_key(&transaction.id) {
            return Err("duplicate transaction");
        }

        let mut simulated = state.clone();
        let mut pending: Vec<&Transaction> = self.transactions.values().collect();
        pending.push(&transaction);
        pending.sort_by(|left, right| left.id.cmp(&right.id));

        for pending_tx in pending {
            simulated.apply_transaction(pending_tx)?;
        }

        self.transactions
            .insert(transaction.id.clone(), transaction);
        Ok(())
    }

    fn validate_basic(&self, transaction: &Transaction) -> Result<(), &'static str> {
        if transaction.chain_id != self.chain_id {
            return Err("transaction chain ID mismatch");
        }
        if !transaction.verify_signature() {
            return Err("invalid transaction signature");
        }
        if transaction.id.is_empty() || transaction.amount == 0 {
            return Err("invalid transaction fields");
        }
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
