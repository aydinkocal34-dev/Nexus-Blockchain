use std::collections::HashMap;

use nexus_blockchain::{
    consensus::{Proposal, Validator, ValidatorSet, Vote, VoteDecision},
    consensus_auth::{SignedProposal, SignedVote},
    network::NetworkMessage,
    Block, Node, Transaction, Wallet,
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
    assert!(node.validate_authenticated_consensus_ready().is_ok());

    let sender = Wallet::new();
    let recipient = Wallet::new();
    node.blockchain.state.credit(sender.address(), 100).unwrap();
    let unsigned = Transaction::new(
        "consensus-auth-test-tx",
        sender.address(),
        recipient.address(),
        10,
        1,
    )
    .with_signature(sender.public_key_bytes(), [0; 64]);
    let signature = sender.sign(&unsigned.signing_bytes());
    let transaction = unsigned.with_signature(sender.public_key_bytes(), signature);
    let block = Block::new(
        1,
        2,
        node.blockchain.latest_block().hash.clone(),
        vec![transaction],
    );
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

#[test]
fn authenticated_consensus_readiness_fails_closed_for_missing_validator_keys() {
    let validators = ValidatorSet::new(vec![
        Validator::new("a", 1).unwrap(),
        Validator::new("b", 1).unwrap(),
        Validator::new("c", 1).unwrap(),
    ])
    .unwrap();
    let mut node = Node::new(1);
    node.configure_validators(validators);
    let signer = Wallet::new();
    let signer_public_key = signer.public_key_bytes();
    node.configure_identity("a");
    node.configure_consensus_signer(signer);
    node.configure_validator_public_key("a", signer_public_key)
        .unwrap();
    assert_eq!(
        node.validate_authenticated_consensus_ready(),
        Err("validator key registry is incomplete")
    );
}

#[test]
fn direct_block_messages_are_rejected_when_consensus_validators_are_configured() {
    let validators = ValidatorSet::new(vec![
        Validator::new("a", 1).unwrap(),
        Validator::new("b", 1).unwrap(),
        Validator::new("c", 1).unwrap(),
    ])
    .unwrap();
    let mut node = Node::new(1);
    node.configure_validators(validators);
    let block = Block::new(1, 2, node.blockchain.latest_block().hash.clone(), vec![]);
    assert_eq!(
        node.handle_network_message(NetworkMessage::LatestBlock(block)),
        Err("direct block acceptance is disabled when consensus validators are configured")
    );
}

#[test]
fn configuring_identity_does_not_overwrite_a_trusted_validator_key() {
    let validators = ValidatorSet::new(vec![Validator::new("a", 1).unwrap()]).unwrap();
    let mut node = Node::new(1);
    node.configure_validators(validators);

    let trusted_key = Wallet::new().public_key_bytes();
    node.configure_validator_public_key("a", trusted_key)
        .unwrap();
    node.configure_consensus_signer(Wallet::new());
    node.configure_identity("a");

    assert_eq!(
        node.validate_authenticated_consensus_ready(),
        Err("local signer does not match registered validator key")
    );
}

#[test]
fn validator_public_key_cannot_be_rebound_to_a_different_key() {
    let validators = ValidatorSet::new(vec![Validator::new("a", 1).unwrap()]).unwrap();
    let mut node = Node::new(1);
    node.configure_validators(validators);

    let trusted_key = Wallet::new().public_key_bytes();
    let attacker_key = Wallet::new().public_key_bytes();
    node.configure_validator_public_key("a", trusted_key)
        .unwrap();

    assert_eq!(
        node.configure_validator_public_key("a", attacker_key),
        Err("validator public key is already configured")
    );
    assert_eq!(
        node.configure_validator_public_key("a", trusted_key),
        Ok(())
    );
}


#[test]
fn validator_public_key_cannot_be_reused_for_another_identity() {
    let validators = ValidatorSet::new(vec![
        Validator::new("a", 1).unwrap(),
        Validator::new("b", 1).unwrap(),
    ])
    .unwrap();
    let mut node = Node::new(1);
    node.configure_validators(validators);

    let shared_key = Wallet::new().public_key_bytes();
    node.configure_validator_public_key("a", shared_key)
        .unwrap();

    assert_eq!(
        node.configure_validator_public_key("b", shared_key),
        Err("validator public key is already bound to another identity")
    );
}
