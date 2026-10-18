use nexus_blockchain::consensus::{
    CommitStatus, ConsensusEngine, Proposal, Validator, ValidatorSet, Vote, VoteDecision,
};

#[test]
fn leader_proposal_reaches_commit_quorum() {
    let validators = ValidatorSet::new(vec![
        Validator::new("a", 1).unwrap(),
        Validator::new("b", 1).unwrap(),
        Validator::new("c", 1).unwrap(),
        Validator::new("d", 1).unwrap(),
    ])
    .unwrap();
    let leader = validators.leader_for_height(7).node_id.clone();
    let proposal = Proposal::new(7, "block-hash", leader);
    let mut engine = ConsensusEngine::new();

    engine.propose(&validators, proposal).unwrap();

    assert_eq!(
        engine
            .record_vote(
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
        engine
            .record_vote(
                &validators,
                Vote {
                    height: 7,
                    block_hash: "block-hash".into(),
                    voter: "b".into(),
                    decision: VoteDecision::Commit,
                },
            )
            .unwrap(),
        CommitStatus::Pending
    );
    assert_eq!(
        engine
            .record_vote(
                &validators,
                Vote {
                    height: 7,
                    block_hash: "block-hash".into(),
                    voter: "c".into(),
                    decision: VoteDecision::Commit
                },
            )
            .unwrap(),
        CommitStatus::Committed
    );
    assert!(engine.is_committed(&validators));
}


#[test]
fn active_proposal_cannot_be_replaced_by_lower_or_higher_height() {
    let validators = ValidatorSet::new(vec![
        Validator::new("a", 1).unwrap(),
        Validator::new("b", 1).unwrap(),
        Validator::new("c", 1).unwrap(),
    ])
    .unwrap();
    let mut engine = ConsensusEngine::new();
    let active_height = 7;
    let leader = validators.leader_for_height(active_height).node_id.clone();
    engine
        .propose(
            &validators,
            Proposal::new(active_height, "active-block", leader),
        )
        .unwrap();

    let lower_height = 6;
    let lower_leader = validators.leader_for_height(lower_height).node_id.clone();
    assert_eq!(
        engine.propose(
            &validators,
            Proposal::new(lower_height, "stale-block", lower_leader),
        ),
        Err("active proposal must be cleared before changing height")
    );
    let higher_height = 8;
    let higher_leader = validators.leader_for_height(higher_height).node_id.clone();
    assert_eq!(
        engine.propose(
            &validators,
            Proposal::new(higher_height, "future-block", higher_leader),
        ),
        Err("active proposal must be cleared before changing height")
    );
    assert_eq!(engine.proposal().unwrap().height, active_height);
    assert_eq!(engine.proposal().unwrap().block_hash, "active-block");
}
