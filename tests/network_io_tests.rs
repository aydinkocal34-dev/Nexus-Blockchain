use nexus_blockchain::network::{read_message, write_message, NetworkMessage, PeerInfo};
use tokio::io::duplex;

#[tokio::test]
async fn framed_message_round_trip() {
    let (mut left, mut right) = duplex(4096);
    let message = NetworkMessage::Ping { nonce: 987 };

    let writer = tokio::spawn(async move {
        write_message(&mut left, &message).await.unwrap();
    });

    let decoded = read_message(&mut right).await.unwrap();
    writer.await.unwrap();

    assert_eq!(decoded, message);
}

#[tokio::test]
async fn multiple_messages_keep_their_boundaries() {
    let (mut left, mut right) = duplex(4096);
    let first = NetworkMessage::Hello(PeerInfo {
        node_id: "node-a".into(),
        address: "127.0.0.1:3000".into(),
    });
    let second = NetworkMessage::Ping { nonce: 42 };

    write_message(&mut left, &first).await.unwrap();
    write_message(&mut left, &second).await.unwrap();

    assert_eq!(read_message(&mut right).await.unwrap(), first);
    assert_eq!(read_message(&mut right).await.unwrap(), second);
}
