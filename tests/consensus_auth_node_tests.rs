use std::collections::HashMap;

use nexus_blockchain::{
    consensus::{Proposal, Validator, ValidatorSet, Vote, VoteDecision},
    consensus_auth::{SignedProposal, SignedVote},
    network::NetworkMessage,
    Block, Node, Wallet,
};

#[test]
fn node_authenticates_signed_consensus_and_rejects_unsigned_messages() {
    let ids = ["a", "b", "c", "d"];
    let validators = ValidatorSet::new(
        ids.iter()
            .map(|id| Validator::new(*id, 1).unwrap())
            .collect(),
    )
    .unwrap();
    let wallets: HashMap<String, Wallet> = ids
        .iter()
        .map(|id| ((*id).to_string(), Wallet::new()))
        .collect();
    let leader = validators.leader_for_height(1).node_id.clone();
    let local_id = ids.iter().copied().find(|id| *id != leader).unwrap();

    let mut node = Node::new(1);
    node.configure_validators(validators);
    node.configure_identity(local_id);
    node.configure_consensus_signer(
        Wallet::from_encrypted_keystore(
            &wallets[local_id]
                .encrypt_keystore(b"temporary-test-password")
                .unwrap(),
            b"temporary-test-password",
        )
        .unwrap(),
    );

    for id in ids {
        node.configure_validator_public_key(id, wallets[id].public_key_bytes())
            .unwrap();
    }

    let block = Block::new(1, 2, node.blockchain.latest_block().hash.clone(), vec![]);
    let proposal = SignedProposal::sign(
        Proposal::new(1, block.hash.clone(), leader.clone()),
        &wallets[&leader],
    );

    let response = node
        .handle_network_message(NetworkMessage::SignedProposeBlock {
            block: block.clone(),
            proposal,
        })
        .unwrap()
        .unwrap();
    let NetworkMessage::SignedVote(vote) = response else {
        panic!("expected signed vote");
    };
    assert!(vote.verify(local_id, &wallets[local_id].public_key_bytes()));
    node.handle_network_message(NetworkMessage::SignedVote(vote))
        .unwrap();

    let other_voter = ids
        .iter()
        .copied()
        .find(|id| *id != leader && *id != local_id)
        .unwrap();
    let valid_vote = SignedVote::sign(
        Vote {
            height: 1,
            block_hash: block.hash.clone(),
            voter: other_voter.into(),
            decision: VoteDecision::Commit,
        },
        &wallets[other_voter],
    );
    node.handle_network_message(NetworkMessage::SignedVote(valid_vote))
        .unwrap();

    let tampered_voter = ids
        .iter()
        .copied()
        .find(|id| *id != leader && *id != local_id && *id != other_voter)
        .unwrap();
    let mut tampered_vote = SignedVote::sign(
        Vote {
            height: 1,
            block_hash: block.hash.clone(),
            voter: tampered_voter.into(),
            decision: VoteDecision::Commit,
        },
        &wallets[tampered_voter],
    );
    tampered_vote.signature[0] ^= 1;
    assert_eq!(
        node.handle_network_message(NetworkMessage::SignedVote(tampered_vote)),
        Err("invalid vote signature")
    );

    let unsigned_result = node.handle_network_message(NetworkMessage::ProposeBlock {
        block,
        proposer: leader,
    });
    assert_eq!(
        unsigned_result,
        Err("unsigned consensus messages are disabled")
    );
}

#[test]
fn signed_consensus_envelopes_survive_network_json_round_trip() {
    let wallet = Wallet::new();
    let vote = nexus_blockchain::consensus::Vote {
        height: 3,
        block_hash: "ef".repeat(32),
        voter: "validator".into(),
        decision: nexus_blockchain::consensus::VoteDecision::Commit,
    };
    let message = NetworkMessage::SignedVote(SignedVote::sign(vote, &wallet));
    let encoded = serde_json::to_vec(&message).unwrap();
    let decoded: NetworkMessage = serde_json::from_slice(&encoded).unwrap();

    let NetworkMessage::SignedVote(decoded_vote) = decoded else {
        panic!("expected signed vote after decoding");
    };
    assert!(decoded_vote.verify("validator", &wallet.public_key_bytes()));
}
