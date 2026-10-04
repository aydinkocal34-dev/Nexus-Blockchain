use nexus_blockchain::{NetworkMessage, PeerInfo, Transaction, Wallet};

#[test]
fn network_messages_round_trip_through_json() {
    let wallet = Wallet::new();
    let tx = Transaction::new("p2p-1", wallet.address(), "recipient", 10, 1);
    let message = NetworkMessage::SubmitTransaction(tx);

    let encoded = serde_json::to_vec(&message).expect("message must serialize");
    let decoded: NetworkMessage =
        serde_json::from_slice(&encoded).expect("message must deserialize");

    assert_eq!(decoded, message);
}

#[test]
fn peer_info_round_trips_through_json() {
    let peer = PeerInfo {
        node_id: "node-1".into(),
        address: "127.0.0.1:30303".into(),
    };

    let encoded = serde_json::to_string(&NetworkMessage::Hello(peer.clone()))
        .expect("hello must serialize");
    let decoded: NetworkMessage =
        serde_json::from_str(&encoded).expect("hello must deserialize");

    assert_eq!(decoded, NetworkMessage::Hello(peer));
}

#[test]
fn ping_and_pong_preserve_nonce() {
    let ping = NetworkMessage::Ping { nonce: 42 };
    let pong = NetworkMessage::Pong { nonce: 42 };

    let ping_bytes = serde_json::to_vec(&ping).unwrap();
    let pong_bytes = serde_json::to_vec(&pong).unwrap();

    assert_eq!(
        serde_json::from_slice::<NetworkMessage>(&ping_bytes).unwrap(),
        ping
    );
    assert_eq!(
        serde_json::from_slice::<NetworkMessage>(&pong_bytes).unwrap(),
        pong
    );
}
