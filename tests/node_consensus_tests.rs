use nexus_blockchain::{
    consensus::{Validator, ValidatorSet},
    network::NetworkMessage,
    Block, Node, Transaction, Wallet,
};

#[test]
fn node_commits_proposed_block_after_quorum() {
    let validators = ValidatorSet::new(vec![
        Validator::new("a", 1).unwrap(),
        Validator::new("b", 1).unwrap(),
        Validator::new("c", 1).unwrap(),
        Validator::new("d", 1).unwrap(),
    ])
    .unwrap();

    let leader = validators.leader_for_height(1).node_id.clone();
    let mut node = Node::new(1);
    node.configure_identity(leader.clone());
    node.configure_validators(validators);

    let sender = Wallet::new();
    let recipient = Wallet::new();
    node.blockchain.state.credit(sender.address(), 100).unwrap();

    let unsigned = Transaction::new("tx-1", sender.address(), recipient.address(), 10, 1)
        .with_signature(sender.public_key_bytes(), sender.sign(b"unused"));
    let signature = sender.sign(&unsigned.signing_bytes());
    let tx = unsigned.with_signature(sender.public_key_bytes(), signature);

    let block = Block::new(
        1,
        2,
        node.blockchain.latest_block().hash.clone(),
        vec![tx],
    );

    let response = node
        .handle_network_message(NetworkMessage::ProposeBlock {
            block: block.clone(),
            proposer: leader.clone(),
        })
        .unwrap();

    let own_vote = match response {
        Some(NetworkMessage::Vote { height, block_hash, voter, decision }) =>
            NetworkMessage::Vote { height, block_hash, voter, decision },
        _ => panic!("expected vote response"),
    };
    node.handle_network_message(own_vote).unwrap();

    let voters = ["a", "b", "c", "d"];
    let mut added = 0;
    for voter in voters {
        if voter == leader { continue; }
        node.handle_network_message(NetworkMessage::Vote {
            height: 1,
            block_hash: block.hash.clone(),
            voter: voter.into(),
            decision: true,
        }).unwrap();
        added += 1;
        if added == 2 { break; }
    }

    assert_eq!(node.blockchain.latest_block().index, 1);
    assert_eq!(node.blockchain.balance_of(&sender.address()), 90);
}
