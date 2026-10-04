use crate::blockchain::Blockchain;
use crate::mempool::Mempool;
use crate::transaction::Transaction;

#[derive(Debug)]
pub struct Node {
    pub blockchain: Blockchain,
    pub mempool: Mempool,
}

impl Node {
    pub fn new(genesis_timestamp: u64) -> Self {
        Self {
            blockchain: Blockchain::new(genesis_timestamp),
            mempool: Mempool::new(),
        }
    }

    pub fn submit_transaction(&mut self, transaction: Transaction) -> Result<(), &'static str> {
        self.mempool.submit(transaction)
    }

    pub fn mine_pending(&mut self, timestamp: u64) -> Result<usize, &'static str> {
        let mut transactions = self.mempool.take_all();

        if transactions.is_empty() {
            return Err("mempool is empty");
        }

        transactions.sort_by(|left, right| left.id.cmp(&right.id));

        if let Err(error) = self
            .blockchain
            .execute_block(timestamp, transactions.clone())
        {
            for transaction in transactions {
                let _ = self.mempool.submit(transaction);
            }
            return Err(error);
        }

        Ok(self.blockchain.latest_block().transactions.len())
    }
}
