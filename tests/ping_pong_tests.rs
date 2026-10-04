use nexus_blockchain::network::{respond_to, NetworkMessage};

#[test]
fn ping_generates_matching_pong() {
    let ping = NetworkMessage::Ping { nonce: 123456 };
    assert_eq!(
        respond_to(&ping),
        Some(NetworkMessage::Pong { nonce: 123456 })
    );
}

#[test]
fn pong_does_not_generate_response() {
    let pong = NetworkMessage::Pong { nonce: 123456 };
    assert_eq!(respond_to(&pong), None);
}
