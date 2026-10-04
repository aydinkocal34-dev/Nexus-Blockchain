use crate::blockchain::Blockchain;
use crate::mempool::Mempool;
use crate::network::{NetworkMessage, PeerManager};
use crate::transaction::Transaction;

#[derive(Debug)]
pub struct Node {
    pub blockchain: Blockchain,
    pub mempool: Mempool,
    pub network: PeerManager,
}

impl Node {
    pub fn new(genesis_timestamp: u64) -> Self {
        Self {
            blockchain: Blockchain::new(genesis_timestamp),
            mempool: Mempool::new(),
            network: PeerManager::new(),
        }
    }

    pub fn submit_transaction(&mut self, transaction: Transaction) -> Result<(), &'static str> {
        self.mempool
            .submit_with_state(&self.blockchain.state, transaction)
    }

    pub async fn submit_transaction_and_broadcast(
        &mut self,
        transaction: Transaction,
    ) -> Result<usize, &'static str> {
        self.submit_transaction(transaction.clone())?;
        self.network
            .broadcast(&NetworkMessage::SubmitTransaction(transaction))
            .await
            .map_err(|_| "failed to broadcast transaction")
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

    pub async fn mine_pending_and_broadcast(
        &mut self,
        timestamp: u64,
    ) -> Result<usize, &'static str> {
        let count = self.mine_pending(timestamp)?;
        self.broadcast_latest_block().await?;
        Ok(count)
    }

    pub async fn broadcast_latest_block(&self) -> Result<usize, &'static str> {
        let block = self.blockchain.latest_block().clone();
        self.network
            .broadcast(&NetworkMessage::LatestBlock(block))
            .await
            .map_err(|_| "failed to broadcast block")
    }

    pub fn handle_network_message(
        &mut self,
        message: NetworkMessage,
    ) -> Result<Option<NetworkMessage>, &'static str> {
        match message {
            NetworkMessage::SubmitTransaction(transaction) => {
                self.submit_transaction(transaction)?;
                Ok(None)
            }
            NetworkMessage::GetLatestBlock => {
                Ok(Some(NetworkMessage::LatestBlock(
                    self.blockchain.latest_block().clone(),
                )))
            }
            NetworkMessage::GetTransactions => {
                Ok(Some(NetworkMessage::Transactions(
                    self.mempool.transactions(),
                )))
            }
            NetworkMessage::LatestBlock(block) => {
                let expected_index = self
                    .blockchain
                    .latest_block()
                    .index
                    .checked_add(1)
                    .ok_or("block index overflow")?;

                if block.index != expected_index {
                    return Err("unexpected block height");
                }
                if block.previous_hash != self.blockchain.latest_block().hash {
                    return Err("invalid previous hash");
                }
                if !block.is_hash_valid() {
                    return Err("invalid block hash");
                }

                self.blockchain
                    .execute_block(block.timestamp, block.transactions)?;
                Ok(None)
            }
            NetworkMessage::Transactions(transactions) => {
                for transaction in transactions {
                    self.submit_transaction(transaction)?;
                }
                Ok(None)
            }
            NetworkMessage::Ping { .. }
            | NetworkMessage::Pong { .. }
            | NetworkMessage::Hello(_) => Ok(None),
        }
    }
}
