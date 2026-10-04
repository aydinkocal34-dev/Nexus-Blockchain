use nexus_blockchain::consensus::{ConsensusDecision, Validator, ValidatorSet};
use nexus_blockchain::Block;

#[test]
fn validator_set_is_deterministic() {
    let set = ValidatorSet::new(vec![
        Validator::new("node-b", 1).unwrap(),
        Validator::new("node-a", 2).unwrap(),
        Validator::new("node-c", 1).unwrap(),
    ])
    .unwrap();

    assert_eq!(set.validators()[0].node_id, "node-a");
    assert_eq!(set.total_weight(), 4);
    assert_eq!(set.quorum_weight(), 3);
    assert_eq!(
        set.leader_for_height(10).node_id,
        set.leader_for_height(10).node_id
    );
}

#[test]
fn validator_set_rejects_invalid_configuration() {
    assert!(ValidatorSet::new(Vec::new()).is_err());
    assert!(Validator::new("node-a", 0).is_err());
    assert!(ValidatorSet::new(vec![
        Validator::new("node-a", 1).unwrap(),
        Validator::new("node-a", 1).unwrap(),
    ])
    .is_err());
}

#[test]
fn higher_block_is_preferred() {
    let local = Block::new(1, 1, "a", Vec::new());
    let candidate = Block::new(2, 2, local.hash.clone(), Vec::new());

    assert_eq!(
        nexus_blockchain::consensus::prefer_candidate(&local, &candidate),
        ConsensusDecision::Accept
    );
}
