use nexus_blockchain::consensus::{
    Proposal, Validator, ValidatorSet, Vote, VoteCollector, VoteDecision,
};

fn validators() -> ValidatorSet {
    ValidatorSet::new(vec![
        Validator::new("a", 1).unwrap(),
        Validator::new("b", 1).unwrap(),
        Validator::new("c", 1).unwrap(),
        Validator::new("d", 1).unwrap(),
    ])
    .unwrap()
}

#[test]
fn two_thirds_quorum_commits() {
    let set = validators();
    let proposal = Proposal::new(7, "block-hash", "a");
    let mut collector = VoteCollector::new(&proposal);

    assert!(!collector
        .add_vote(
            &set,
            Vote {
                height: 7,
                block_hash: "block-hash".into(),
                voter: "a".into(),
                decision: VoteDecision::Commit,
            },
        )
        .unwrap());

    assert!(collector
        .add_vote(
            &set,
            Vote {
                height: 7,
                block_hash: "block-hash".into(),
                voter: "b".into(),
                decision: VoteDecision::Commit,
            },
        )
        .unwrap());

    assert!(collector.is_committed(&set));
}

#[test]
fn duplicate_and_unknown_votes_are_rejected() {
    let set = validators();
    let proposal = Proposal::new(7, "block-hash", "a");
    let mut collector = VoteCollector::new(&proposal);

    let vote = Vote {
        height: 7,
        block_hash: "block-hash".into(),
        voter: "a".into(),
        decision: VoteDecision::Commit,
    };

    collector.add_vote(&set, vote.clone()).unwrap();
    assert_eq!(collector.add_vote(&set, vote), Err("duplicate validator vote"));

    assert_eq!(
        collector.add_vote(
            &set,
            Vote {
                height: 7,
                block_hash: "block-hash".into(),
                voter: "unknown".into(),
                decision: VoteDecision::Commit,
            },
        ),
        Err("voter is not a validator")
    );
}
