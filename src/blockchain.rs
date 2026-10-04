use crate::block::Block;
use crate::state::State;
use crate::transaction::Transaction;

#[derive(Debug, Clone)]
pub struct Blockchain {
    pub chain: Vec<Block>,
    pub state: State,
}

impl Blockchain {
    pub fn new(genesis_timestamp: u64) -> Self {
        Self {
            chain: vec![Block::new(0, genesis_timestamp, "0", Vec::new())],
            state: State::new(),
        }
    }

    pub fn latest_block(&self) -> &Block {
        self.chain
            .last()
            .expect("blockchain must contain genesis block")
    }

    pub fn execute_block(
        &mut self,
        timestamp: u64,
        transactions: Vec<Transaction>,
    ) -> Result<(), &'static str> {
        if transactions.is_empty() {
            return Err("block must contain at least one transaction");
        }

        let mut next_state = self.state.clone();
        for transaction in &transactions {
            next_state.apply_transaction(transaction)?;
        }

        let next_index = self
            .latest_block()
            .index
            .checked_add(1)
            .ok_or("block index overflow")?;
        let previous_hash = self.latest_block().hash.clone();

        let block = Block::new(next_index, timestamp, previous_hash, transactions);

        self.chain.push(block);
        self.state = next_state;
        Ok(())
    }

    pub fn balance_of(&self, address: &str) -> u64 {
        self.state.balance_of(address)
    }

    pub fn nonce_of(&self, address: &str) -> u64 {
        self.state.nonce_of(address)
    }

    pub fn is_valid(&self) -> bool {
        if self.chain.is_empty() || !self.chain[0].is_hash_valid() {
            return false;
        }

        for pair in self.chain.windows(2) {
            let previous = &pair[0];
            let current = &pair[1];

            let Some(expected_index) = previous.index.checked_add(1) else {
                return false;
            };

            if current.index != expected_index
                || current.previous_hash != previous.hash
                || !current.is_hash_valid()
            {
                return false;
            }
        }

        true
    }
}
