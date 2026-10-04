use nexus_blockchain::{NetworkMessage, PeerInfo, PeerManager};
use tokio::net::TcpListener;

#[tokio::test]
async fn inbound_application_messages_are_routed_to_events() {
    let server = PeerManager::new();
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();

    let server_task = server
        .listen_on(
            listener,
            PeerInfo {
                node_id: "server".into(),
                address: address.to_string(),
            },
        )
        .await
        .unwrap();

    let client = PeerManager::new();
    client
        .connect(
            &address.to_string(),
            PeerInfo {
                node_id: "client".into(),
                address: "client".into(),
            },
        )
        .await
        .unwrap();

    let mut events = server.take_event_receiver().await.unwrap();
    client.broadcast(&NetworkMessage::GetLatestBlock).await.unwrap();

    let event = tokio::time::timeout(std::time::Duration::from_secs(2), events.recv())
        .await
        .unwrap()
        .unwrap();

    assert_eq!(event.peer_id, "client");
    assert_eq!(event.message, NetworkMessage::GetLatestBlock);

    server_task.abort();
}
