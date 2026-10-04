use crate::Block;
use sha2::{Digest, Sha256};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConsensusDecision {
    Accept,
    Reject,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Validator {
    pub node_id: String,
    pub weight: u64,
}

impl Validator {
    pub fn new(node_id: impl Into<String>, weight: u64) -> Result<Self, &'static str> {
        if weight == 0 {
            return Err("validator weight must be greater than zero");
        }
        Ok(Self {
            node_id: node_id.into(),
            weight,
        })
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
        (self.total_weight() / 3).saturating_mul(2).saturating_add(1)
    }

    pub fn leader_for_height(&self, height: u64) -> &Validator {
        let digest = Sha256::digest(height.to_be_bytes());
        let mut value = [0u8; 8];
        value.copy_from_slice(&digest[..8]);
        let index = u64::from_be_bytes(value) % self.validators.len() as u64;
        &self.validators[index as usize]
    }
}

/// Deterministic first-stage fork rule: a candidate at a higher height wins.
/// Equal-height candidates are rejected here and require explicit consensus
/// evidence in a later protocol stage.
pub fn prefer_candidate(local: &Block, candidate: &Block) -> ConsensusDecision {
    if candidate.index > local.index {
        ConsensusDecision::Accept
    } else {
        ConsensusDecision::Reject
    }
}
