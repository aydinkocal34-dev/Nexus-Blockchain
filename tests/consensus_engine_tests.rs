use nexus_blockchain::consensus::{
    CommitStatus, ConsensusEngine, Proposal, Validator, ValidatorSet, Vote, VoteDecision,
};

#[test]
fn leader_proposal_reaches_commit_quorum() {
    let validators = ValidatorSet::new(vec![
        Validator::new("a", 1).unwrap(),
        Validator::new("b", 1).unwrap(),
        Validator::new("c", 1).unwrap(),
    ])
    .unwrap();
    let leader = validators.leader_for_height(7).node_id.clone();
    let proposal = Proposal::new(7, "block-hash", leader);
    let mut engine = ConsensusEngine::new();

    engine.propose(&validators, proposal).unwrap();

    assert_eq!(
        engine.record_vote(
            &validators,
            Vote {
                height: 7,
                block_hash: "block-hash".into(),
                voter: "a".into(),
                decision: VoteDecision::Commit,
            },
        )
        .unwrap(),
        CommitStatus::Pending
    );

    assert_eq!(
        engine.record_vote(
            &validators,
            Vote {
                height: 7,
                block_hash: "block-hash".into(),
                voter: "b".into(),
                decision: VoteDecision::Commit,
            },
        )
        .unwrap(),
        CommitStatus::Committed
    );
    assert!(engine.is_committed(&validators));
}
