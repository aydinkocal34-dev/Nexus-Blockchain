use crate::Block;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConsensusDecision {
    Accept,
    Reject,
}

/// Minimal deterministic chain-choice rule.
///
/// Consensus is intentionally separated from block execution: this module
/// decides whether a candidate chain is preferable, while Blockchain remains
/// responsible for validating and applying blocks.
pub fn prefer_candidate(local: &Block, candidate: &Block) -> ConsensusDecision {
    if candidate.index > local.index {
        ConsensusDecision::Accept
    } else {
        ConsensusDecision::Reject
    }
}
