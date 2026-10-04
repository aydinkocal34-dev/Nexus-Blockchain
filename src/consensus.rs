use crate::Block;
use sha2::{Digest, Sha256};
use std::collections::BTreeSet;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConsensusDecision {
    Accept,
    Reject,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VoteDecision {
    Commit,
    Reject,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Validator {
    pub node_id: String,
    pub weight: u64,
}

impl Validator {
    pub fn new(node_id: impl Into<String>, weight: u64) -> Result<Self, &'static str> {
        let node_id = node_id.into();
        if node_id.is_empty() {
            return Err("validator node id must not be empty");
        }
        if weight == 0 {
            return Err("validator weight must be greater than zero");
        }
        Ok(Self { node_id, weight })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ValidatorSet {
    validators: Vec<Validator>,
}

impl ValidatorSet {
    pub fn new(mut validators: Vec<Validator>) -> Result<Self, &'static str> {
        if validators.is_empty() {
            return Err("validator set must not be empty");
        }
        validators.sort_by(|a, b| a.node_id.cmp(&b.node_id));
        for pair in validators.windows(2) {
            if pair[0].node_id == pair[1].node_id {
                return Err("duplicate validator");
            }
        }
        Ok(Self { validators })
    }

    pub fn validators(&self) -> &[Validator] {
        &self.validators
    }

    pub fn total_weight(&self) -> u64 {
        self.validators
            .iter()
            .fold(0u64, |total, validator| total.saturating_add(validator.weight))
    }

    pub fn quorum_weight(&self) -> u64 {
        let total = self.total_weight();
        if total == 0 { return 0; }
        total.saturating_mul(2).saturating_div(3).saturating_add(1).min(total)
    }

    pub fn weight_of(&self, node_id: &str) -> u64 {
        self.validators
            .iter()
            .find(|validator| validator.node_id == node_id)
            .map(|validator| validator.weight)
            .unwrap_or(0)
    }

    pub fn leader_for_height(&self, height: u64) -> &Validator {
        let digest = Sha256::digest(height.to_be_bytes());
        let mut value = [0u8; 8];
        value.copy_from_slice(&digest[..8]);
        let index = u64::from_be_bytes(value) % self.validators.len() as u64;
        &self.validators[index as usize]
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Proposal {
    pub height: u64,
    pub block_hash: String,
    pub proposer: String,
}

impl Proposal {
    pub fn new(height: u64, block_hash: impl Into<String>, proposer: impl Into<String>) -> Self {
        Self {
            height,
            block_hash: block_hash.into(),
            proposer: proposer.into(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Vote {
    pub height: u64,
    pub block_hash: String,
    pub voter: String,
    pub decision: VoteDecision,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VoteCollector {
    height: u64,
    block_hash: String,
    votes: Vec<Vote>,
}

impl VoteCollector {
    pub fn new(proposal: &Proposal) -> Self {
        Self {
            height: proposal.height,
            block_hash: proposal.block_hash.clone(),
            votes: Vec::new(),
        }
    }

    pub fn add_vote(
        &mut self,
        validators: &ValidatorSet,
        vote: Vote,
    ) -> Result<bool, &'static str> {
        if vote.height != self.height || vote.block_hash != self.block_hash {
            return Err("vote does not match proposal");
        }
        if validators.weight_of(&vote.voter) == 0 {
            return Err("voter is not a validator");
        }
        if self.votes.iter().any(|existing| existing.voter == vote.voter) {
            return Err("duplicate validator vote");
        }

        self.votes.push(vote);
        Ok(self.committed_weight(validators) >= validators.quorum_weight())
    }

    pub fn committed_weight(&self, validators: &ValidatorSet) -> u64 {
        self.votes
            .iter()
            .filter(|vote| vote.decision == VoteDecision::Commit)
            .map(|vote| validators.weight_of(&vote.voter))
            .fold(0u64, u64::saturating_add)
    }

    pub fn rejected_weight(&self, validators: &ValidatorSet) -> u64 {
        self.votes
            .iter()
            .filter(|vote| vote.decision == VoteDecision::Reject)
            .map(|vote| validators.weight_of(&vote.voter))
            .fold(0u64, u64::saturating_add)
    }

    pub fn is_committed(&self, validators: &ValidatorSet) -> bool {
        self.committed_weight(validators) >= validators.quorum_weight()
    }

    pub fn voters(&self) -> BTreeSet<&str> {
        self.votes.iter().map(|vote| vote.voter.as_str()).collect()
    }
}

pub fn prefer_candidate(local: &Block, candidate: &Block) -> ConsensusDecision {
    if candidate.index > local.index {
        ConsensusDecision::Accept
    } else {
        ConsensusDecision::Reject
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CommitStatus {
    Pending,
    Committed,
}

#[derive(Debug, Clone)]
pub struct ConsensusEngine {
    proposal: Option<Proposal>,
    collector: Option<VoteCollector>,
}

impl ConsensusEngine {
    pub fn new() -> Self {
        Self { proposal: None, collector: None }
    }

    pub fn propose(
        &mut self,
        validators: &ValidatorSet,
        proposal: Proposal,
    ) -> Result<(), &'static str> {
        if validators.leader_for_height(proposal.height).node_id != proposal.proposer {
            return Err("proposal proposer is not the deterministic leader");
        }
        if proposal.block_hash.is_empty() {
            return Err("proposal block hash must not be empty");
        }
        if self.proposal.as_ref().map(|p| p.height == proposal.height).unwrap_or(false) {
            return Err("proposal already exists for height");
        }
        self.collector = Some(VoteCollector::new(&proposal));
        self.proposal = Some(proposal);
        Ok(())
    }

    pub fn record_vote(
        &mut self,
        validators: &ValidatorSet,
        vote: Vote,
    ) -> Result<CommitStatus, &'static str> {
        let collector = self.collector.as_mut().ok_or("no active proposal")?;
        let committed = collector.add_vote(validators, vote)?;
        Ok(if committed { CommitStatus::Committed } else { CommitStatus::Pending })
    }

    pub fn proposal(&self) -> Option<&Proposal> { self.proposal.as_ref() }

    pub fn is_committed(&self, validators: &ValidatorSet) -> bool {
        self.collector.as_ref().map(|c| c.is_committed(validators)).unwrap_or(false)
    }

    pub fn clear(&mut self) {
        self.proposal = None;
        self.collector = None;
    }
}

impl Default for ConsensusEngine {
    fn default() -> Self { Self::new() }
}
