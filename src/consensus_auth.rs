use crate::config::DEVNET_CHAIN_ID;
use crate::consensus::{Proposal, Vote, VoteDecision};
use crate::Wallet;
use ed25519_dalek::{Signature, Verifier, VerifyingKey};
use serde::{Deserialize, Serialize};

/// A consensus proposal accompanied by the public key and Ed25519 signature
/// of the claimed proposer. This envelope is not accepted by Node until the
/// validator-set key binding is checked by the caller.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SignedProposal {
    /// Chain ID is included in the signature payload and checked by the receiver.
    #[serde(default = "default_chain_id")]
    pub chain_id: u64,
    pub proposal: Proposal,
    pub public_key: [u8; 32],
    pub signature: Vec<u8>,
}

impl SignedProposal {
    /// Sign a proposal for devnet. Production callers should use sign_for_chain.
    pub fn sign(proposal: Proposal, signer: &Wallet) -> Self {
        Self::sign_for_chain(proposal, signer, DEVNET_CHAIN_ID)
    }

    pub fn sign_for_chain(proposal: Proposal, signer: &Wallet, chain_id: u64) -> Self {
        let signature = signer.sign(&proposal_signing_bytes(&proposal, chain_id));
        Self {
            chain_id,
            proposal,
            public_key: signer.public_key_bytes(),
            signature: signature.to_vec(),
        }
    }

    pub fn verify(&self, expected_proposer: &str, expected_public_key: &[u8; 32]) -> bool {
        self.verify_for_chain(expected_proposer, expected_public_key, DEVNET_CHAIN_ID)
    }

    pub fn verify_for_chain(
        &self,
        expected_proposer: &str,
        expected_public_key: &[u8; 32],
        expected_chain_id: u64,
    ) -> bool {
        if self.chain_id != expected_chain_id
            || self.proposal.proposer != expected_proposer
            || &self.public_key != expected_public_key
        {
            return false;
        }
        verify_signature(
            expected_public_key,
            &proposal_signing_bytes(&self.proposal, self.chain_id),
            &self.signature,
        )
    }
}

/// A consensus vote accompanied by the public key and Ed25519 signature
/// of the claimed voter. The caller must bind voter ID to the validator set.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SignedVote {
    /// Chain ID is included in the signature payload and checked by the receiver.
    #[serde(default = "default_chain_id")]
    pub chain_id: u64,
    pub vote: Vote,
    pub public_key: [u8; 32],
    pub signature: Vec<u8>,
}

impl SignedVote {
    /// Sign a vote for devnet. Production callers should use sign_for_chain.
    pub fn sign(vote: Vote, signer: &Wallet) -> Self {
        Self::sign_for_chain(vote, signer, DEVNET_CHAIN_ID)
    }

    pub fn sign_for_chain(vote: Vote, signer: &Wallet, chain_id: u64) -> Self {
        let signature = signer.sign(&vote_signing_bytes(&vote, chain_id));
        Self {
            chain_id,
            vote,
            public_key: signer.public_key_bytes(),
            signature: signature.to_vec(),
        }
    }

    pub fn verify(&self, expected_voter: &str, expected_public_key: &[u8; 32]) -> bool {
        self.verify_for_chain(expected_voter, expected_public_key, DEVNET_CHAIN_ID)
    }

    pub fn verify_for_chain(
        &self,
        expected_voter: &str,
        expected_public_key: &[u8; 32],
        expected_chain_id: u64,
    ) -> bool {
        if self.chain_id != expected_chain_id
            || self.vote.voter != expected_voter
            || &self.public_key != expected_public_key
        {
            return false;
        }
        verify_signature(
            expected_public_key,
            &vote_signing_bytes(&self.vote, self.chain_id),
            &self.signature,
        )
    }
}

fn default_chain_id() -> u64 {
    DEVNET_CHAIN_ID
}

fn proposal_signing_bytes(proposal: &Proposal, chain_id: u64) -> Vec<u8> {
    serde_json::to_vec(&(
        "NEXUS-CONSENSUS-PROPOSAL-V2",
        chain_id,
        proposal.height,
        &proposal.block_hash,
        &proposal.proposer,
    ))
    .expect("proposal signing payload must serialize")
}

fn vote_signing_bytes(vote: &Vote, chain_id: u64) -> Vec<u8> {
    let decision = match vote.decision {
        VoteDecision::Commit => "commit",
        VoteDecision::Reject => "reject",
    };
    serde_json::to_vec(&(
        "NEXUS-CONSENSUS-VOTE-V2",
        chain_id,
        vote.height,
        &vote.block_hash,
        &vote.voter,
        decision,
    ))
    .expect("vote signing payload must serialize")
}

fn verify_signature(public_key: &[u8; 32], message: &[u8], signature: &[u8]) -> bool {
    let Ok(public_key) = VerifyingKey::from_bytes(public_key) else {
        return false;
    };
    let Ok(signature_bytes) = <[u8; 64]>::try_from(signature) else {
        return false;
    };
    public_key
        .verify(message, &Signature::from_bytes(&signature_bytes))
        .is_ok()
}
