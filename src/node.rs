use crate::blockchain::Blockchain;
use crate::consensus::{CommitStatus, ConsensusEngine, Proposal, ValidatorSet, Vote, VoteDecision};
use crate::mempool::Mempool;
use crate::network::{read_message, write_message, NetworkEvent, NetworkMessage, PeerInfo, PeerManager};
use crate::transaction::Transaction;
use tokio::net::TcpStream;
use tokio::sync::mpsc;

#[derive(Debug)]
pub struct Node {
    pub blockchain: Blockchain,
    pub mempool: Mempool,
    pub network: PeerManager,
    pub consensus: ConsensusEngine,
    pub validators: Option<ValidatorSet>,
    pub node_id: Option<String>,
    pending_consensus_block: Option<crate::Block>,
}

impl Node {
    pub fn new(genesis_timestamp: u64) -> Self {
        Self {
            blockchain: Blockchain::new(genesis_timestamp),
            mempool: Mempool::new(),
            network: PeerManager::new(),
            consensus: ConsensusEngine::new(),
            validators: None,
            node_id: None,
            pending_consensus_block: None,
        }
    }

    pub fn configure_identity(&mut self, node_id: impl Into<String>) {
        self.node_id = Some(node_id.into());
    }

    pub fn configure_validators(&mut self, validators: ValidatorSet) {
        self.validators = Some(validators);
    }

    pub fn submit_transaction(&mut self, transaction: Transaction) -> Result<(), &'static str> {
        self.mempool.submit_with_state(&self.blockchain.state, transaction)
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
        if let Err(error) = self.blockchain.execute_block(timestamp, transactions.clone()) {
            for transaction in transactions {
                let _ = self.mempool.submit(transaction);
            }
            return Err(error);
        }
        Ok(self.blockchain.latest_block().transactions.len())
    }

    pub async fn mine_pending_and_broadcast(&mut self, timestamp: u64) -> Result<usize, &'static str> {
        let count = self.mine_pending(timestamp)?;
        self.broadcast_latest_block().await?;
        Ok(count)
    }

    pub async fn sync_from_peer(&mut self, address: &str, local: PeerInfo) -> Result<usize, &'static str> {
        let stream = TcpStream::connect(address).await.map_err(|_| "failed to connect to sync peer")?;
        let (mut reader, mut writer) = stream.into_split();
        write_message(&mut writer, &NetworkMessage::Hello(local)).await.map_err(|_| "failed to send hello")?;
        match read_message(&mut reader).await.map_err(|_| "failed to read hello")? {
            NetworkMessage::Hello(_) => {}
            _ => return Err("expected hello"),
        }
        let next_index = self.blockchain.latest_block().index.checked_add(1).ok_or("block index overflow")?;
        write_message(&mut writer, &NetworkMessage::GetBlocks { from_index: next_index })
            .await.map_err(|_| "failed to request blocks")?;
        let response = read_message(&mut reader).await.map_err(|_| "failed to read blocks")?;
        let NetworkMessage::Blocks(blocks) = response else { return Err("expected blocks response"); };
        let mut applied = 0;
        for block in blocks {
            self.blockchain.apply_existing_block(block)?;
            applied += 1;
        }
        Ok(applied)
    }

    pub async fn broadcast_latest_block(&self) -> Result<usize, &'static str> {
        self.network.broadcast(&NetworkMessage::LatestBlock(self.blockchain.latest_block().clone()))
            .await.map_err(|_| "failed to broadcast block")
    }

    pub fn handle_network_message(&mut self, message: NetworkMessage) -> Result<Option<NetworkMessage>, &'static str> {
        match message {
            NetworkMessage::SubmitTransaction(transaction) => {
                self.submit_transaction(transaction)?;
                Ok(None)
            }
            NetworkMessage::GetLatestBlock => Ok(Some(NetworkMessage::LatestBlock(self.blockchain.latest_block().clone()))),
            NetworkMessage::GetBlocks { from_index } => Ok(Some(NetworkMessage::Blocks(self.blockchain.blocks_from(from_index)))),
            NetworkMessage::GetTransactions => Ok(Some(NetworkMessage::Transactions(self.mempool.transactions()))),
            NetworkMessage::LatestBlock(block) => {
                let expected_index = self.blockchain.latest_block().index.checked_add(1).ok_or("block index overflow")?;
                if block.index != expected_index { return Err("unexpected block height"); }
                if block.previous_hash != self.blockchain.latest_block().hash { return Err("invalid previous hash"); }
                if !block.is_hash_valid() { return Err("invalid block hash"); }
                self.blockchain.apply_existing_block(block)?;
                Ok(None)
            }
            NetworkMessage::Transactions(transactions) => {
                for transaction in transactions { self.submit_transaction(transaction)?; }
                Ok(None)
            }
            NetworkMessage::ProposeBlock { block, proposer } => {
                let validators = self.validators.as_ref().ok_or("validator set is not configured")?;
                let expected_index = self.blockchain.latest_block().index.checked_add(1).ok_or("block index overflow")?;
                if block.index != expected_index { return Err("unexpected proposal height"); }
                if block.previous_hash != self.blockchain.latest_block().hash { return Err("invalid proposal previous hash"); }
                if !block.is_hash_valid() { return Err("invalid proposed block hash"); }
                let proposal = Proposal::new(block.index, block.hash.clone(), proposer);
                self.consensus.propose(validators, proposal)?;
                self.pending_consensus_block = Some(block);
                let voter = self.node_id.as_ref().ok_or("node identity is not configured")?.clone();
                if validators.weight_of(&voter) == 0 { return Err("node identity is not a validator"); }
                Ok(Some(NetworkMessage::Vote {
                    height: expected_index,
                    block_hash: self.pending_consensus_block.as_ref().unwrap().hash.clone(),
                    voter,
                    decision: true,
                }))
            }
            NetworkMessage::Proposal { .. } => Ok(None),
            NetworkMessage::Vote { height, block_hash, voter, decision } => {
                let validators = self.validators.as_ref().ok_or("validator set is not configured")?;
                let status = self.consensus.record_vote(validators, Vote {
                    height,
                    block_hash: block_hash.clone(),
                    voter,
                    decision: if decision { VoteDecision::Commit } else { VoteDecision::Reject },
                })?;
                if status == CommitStatus::Committed {
                    let block = self.pending_consensus_block.take().ok_or("committed proposal block is missing")?;
                    if block.index != height || block.hash != block_hash { return Err("committed vote does not match pending block"); }
                    self.blockchain.apply_existing_block(block)?;
                    self.consensus.clear();
                }
                Ok(None)
            }
            NetworkMessage::Ping { .. } | NetworkMessage::Pong { .. } | NetworkMessage::Hello(_) => Ok(None),
        }
    }

    pub async fn take_network_events(&self) -> Option<mpsc::Receiver<NetworkEvent>> {
        self.network.take_event_receiver().await
    }

    pub async fn process_network_events(&mut self) -> Result<(), &'static str> {
        let mut receiver = self.network.take_event_receiver().await.ok_or("network event receiver already taken")?;
        while let Some(event) = receiver.recv().await {
            let response = match self.handle_network_message(event.message) {
                Ok(response) => response,
                Err(_) => continue,
            };
            if let Some(response) = response {
                let _ = self.network.send_to_peer(&event.peer_id, &response).await;
            }
        }
        Ok(())
    }
}
