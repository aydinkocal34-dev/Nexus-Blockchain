use nexus_blockchain::{
    consensus::{Validator, ValidatorSet},
    network::NetworkMessage,
    Block, Node, Transaction, Wallet,
};

fn validator_set() -> ValidatorSet {
    ValidatorSet::new(vec![
        Validator::new("node-0", 1).unwrap(),
        Validator::new("node-1", 1).unwrap(),
        Validator::new("node-2", 1).unwrap(),
        Validator::new("node-3", 1).unwrap(),
    ])
    .unwrap()
}

#[test]
fn four_node_consensus_reaches_commit() {
    let validators = validator_set();
    let height = 1;
    let leader = validators.leader_for_height(height).node_id.clone();

    let sender = Wallet::new();
    let recipient = Wallet::new();
    let mut unsigned = Transaction::new(
        "testnet-tx-1",
        sender.address(),
        recipient.address(),
        10,
        1,
    )
    .with_signature(sender.public_key_bytes(), [0; 64]);
    let signature = sender.sign(&unsigned.signing_bytes());
    unsigned = unsigned.with_signature(sender.public_key_bytes(), signature);

    let mut nodes = Vec::new();
    for index in 0..4 {
        let mut node = Node::new(1);
        node.configure_identity(format!("node-{index}"));
        node.configure_validators(validators.clone());
        node.blockchain.state.credit(sender.address(), 100).unwrap();
        nodes.push(node);
    }

    let block = Block::new(
        height,
        2,
        nodes[0].blockchain.latest_block().hash.clone(),
        vec![unsigned],
    );

    let mut votes = Vec::new();
    for node in &mut nodes {
        if let Some(NetworkMessage::Vote { height, block_hash, voter, decision }) =
            node.handle_network_message(NetworkMessage::ProposeBlock {
                block: block.clone(),
                proposer: leader.clone(),
            }).unwrap()
        {
            votes.push(NetworkMessage::Vote { height, block_hash, voter, decision });
        }
    }

    assert_eq!(votes.len(), 4);

    let leader_index = leader.strip_prefix("node-").unwrap().parse::<usize>().unwrap();
    let leader_node = &mut nodes[leader_index];

    for vote in votes {
        leader_node.handle_network_message(vote).unwrap();
    }

    assert_eq!(leader_node.blockchain.latest_block().index, 1);
    assert_eq!(leader_node.blockchain.balance_of(&sender.address()), 90);
}
