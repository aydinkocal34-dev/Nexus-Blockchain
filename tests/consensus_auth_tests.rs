use nexus_blockchain::{
    consensus::{Proposal, Vote, VoteDecision},
    consensus_auth::{SignedProposal, SignedVote},
    Wallet,
};

#[test]
fn signed_proposal_verifies_and_rejects_tampering_or_wrong_identity() {
    let wallet = Wallet::new();
    let proposal = Proposal::new(7, "ab".repeat(32), "validator-1");
    let signed = SignedProposal::sign(proposal, &wallet);
    let public_key = wallet.public_key_bytes();

    assert!(signed.verify("validator-1", &public_key));
    assert!(!signed.verify("validator-2", &public_key));
    let mut tampered = signed.clone();
    tampered.proposal.height += 1;
    assert!(!tampered.verify("validator-1", &public_key));
    assert!(!signed.verify("validator-1", &[0; 32]));
}

#[test]
fn signed_vote_verifies_decision_height_and_identity() {
    let wallet = Wallet::new();
    let vote = Vote {
        height: 9,
        block_hash: "cd".repeat(32),
        voter: "validator-2".into(),
        decision: VoteDecision::Commit,
    };
    let signed = SignedVote::sign(vote, &wallet);
    let public_key = wallet.public_key_bytes();

    assert!(signed.verify("validator-2", &public_key));
    assert!(!signed.verify("validator-3", &public_key));
    let mut tampered = signed.clone();
    tampered.vote.decision = VoteDecision::Reject;
    assert!(!tampered.verify("validator-2", &public_key));
    let mut malformed = signed;
    malformed.signature.pop();
    assert!(!malformed.verify("validator-2", &public_key));
}
