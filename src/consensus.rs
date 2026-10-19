use crate::Block;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConsensusDecision {
    Accept,
    Reject,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
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
        // A validator set whose total weight cannot be represented in u64
        // must be rejected. Saturating the total would silently lower quorum.
        validators.iter().try_fold(0u64, |total, validator| {
            total
                .checked_add(validator.weight)
                .ok_or("total validator weight overflows u64")
        })?;
        Ok(Self { validators })
    }

    pub fn validators(&self) -> &[Validator] {
        &self.validators
    }

    pub fn total_weight(&self) -> u64 {
        // ValidatorSet::new guarantees this sum fits in u64.
        self.validators
            .iter()
            .map(|validator| validator.weight)
            .sum()
    }

    pub fn quorum_weight(&self) -> u64 {
        let total = self.total_weight();
        if total == 0 {
            return 0;
        }
        // Use a wider intermediate so 2 * total cannot overflow u64.
        (((u128::from(total) * 2) / 3) + 1).min(u128::from(total)) as u64
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

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
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

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
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
        if self
            .votes
            .iter()
            .any(|existing| existing.voter == vote.voter)
        {
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

    pub fn is_rejected(&self, validators: &ValidatorSet) -> bool {
        self.rejected_weight(validators) >= validators.quorum_weight()
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
    Rejected,
}

#[derive(Debug, Clone)]
pub struct ConsensusEngine {
    proposal: Option<Proposal>,
    collector: Option<VoteCollector>,
    /// First accepted vote per (height, validator); used to detect equivocation
    /// even when a vote arrives through a different collector path.
    seen_votes: BTreeMap<(u64, String), (String, VoteDecision)>,
}

impl ConsensusEngine {
    pub fn new() -> Self {
        Self {
            proposal: None,
            collector: None,
            seen_votes: BTreeMap::new(),
        }
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
        if let Some(active_proposal) = self.proposal.as_ref() {
            if active_proposal.height == proposal.height {
                return Err("proposal already exists for height");
            }
            return Err("active proposal must be cleared before changing height");
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
        let key = (vote.height, vote.voter.clone());
        if let Some((existing_hash, existing_decision)) = self.seen_votes.get(&key) {
            if existing_hash != &vote.block_hash || existing_decision != &vote.decision {
                return Err("equivocation detected");
            }
            return Err("duplicate validator vote");
        }

        let collector = self.collector.as_mut().ok_or("no active proposal")?;
        collector.add_vote(validators, vote.clone())?;
        self.seen_votes
            .insert(key, (vote.block_hash.clone(), vote.decision));
        if collector.is_committed(validators) {
            Ok(CommitStatus::Committed)
        } else if collector.is_rejected(validators) {
            Ok(CommitStatus::Rejected)
        } else {
            Ok(CommitStatus::Pending)
        }
    }

    pub fn proposal(&self) -> Option<&Proposal> {
        self.proposal.as_ref()
    }

    pub fn is_committed(&self, validators: &ValidatorSet) -> bool {
        self.collector
            .as_ref()
            .map(|c| c.is_committed(validators))
            .unwrap_or(false)
    }

    pub fn clear(&mut self) {
        self.proposal = None;
        self.collector = None;
    }
}

impl Default for ConsensusEngine {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod quorum_overflow_tests {
    use super::{Validator, ValidatorSet};

    fn set_with_weights(weights: &[u64]) -> Result<ValidatorSet, &'static str> {
        ValidatorSet::new(
            weights
                .iter()
                .enumerate()
                .map(|(i, weight)| Validator::new(format!("validator-{i}"), *weight).unwrap())
                .collect(),
        )
    }

    #[test]
    fn rejects_validator_weight_sum_overflow() {
        assert!(set_with_weights(&[u64::MAX, 1]).is_err());
    }

    #[test]
    fn quorum_is_correct_above_u64_half() {
        let total = 12_000_000_000_000_000_000u64;
        let validators =
            set_with_weights(&[6_000_000_000_000_000_000, 6_000_000_000_000_000_000]).unwrap();
        assert_eq!(validators.total_weight(), total);
        assert_eq!(validators.quorum_weight(), 8_000_000_000_000_000_001);
    }

    #[test]
    fn quorum_boundary_values_are_correct() {
        for total in [u64::MAX / 2, (u64::MAX / 2) + 1, u64::MAX] {
            let validators = set_with_weights(&[total]).unwrap();
            let expected = (((u128::from(total) * 2) / 3) + 1).min(u128::from(total)) as u64;
            assert_eq!(validators.quorum_weight(), expected);
        }
    }
}
