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

#[test]
fn signed_consensus_envelopes_are_bound_to_chain_id() {
    use nexus_blockchain::config::{MAINNET_CHAIN_ID, TESTNET_CHAIN_ID};

    let wallet = Wallet::new();
    let public_key = wallet.public_key_bytes();

    let proposal = Proposal::new(4, "12".repeat(32), "validator-1");
    let signed_proposal = SignedProposal::sign_for_chain(proposal, &wallet, MAINNET_CHAIN_ID);
    assert!(signed_proposal.verify_for_chain("validator-1", &public_key, MAINNET_CHAIN_ID));
    assert!(!signed_proposal.verify_for_chain("validator-1", &public_key, TESTNET_CHAIN_ID));

    let vote = Vote {
        height: 4,
        block_hash: "34".repeat(32),
        voter: "validator-1".into(),
        decision: VoteDecision::Commit,
    };
    let signed_vote = SignedVote::sign_for_chain(vote, &wallet, MAINNET_CHAIN_ID);
    assert!(signed_vote.verify_for_chain("validator-1", &public_key, MAINNET_CHAIN_ID));
    assert!(!signed_vote.verify_for_chain("validator-1", &public_key, TESTNET_CHAIN_ID));

    let mut tampered_chain = signed_vote.clone();
    tampered_chain.chain_id = TESTNET_CHAIN_ID;
    assert!(!tampered_chain.verify_for_chain("validator-1", &public_key, TESTNET_CHAIN_ID));
}
