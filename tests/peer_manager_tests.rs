use nexus_blockchain::network::{NetworkMessage, PeerInfo, PeerManager};
use tokio::time::{sleep, Duration};

#[tokio::test]
async fn peer_manager_establishes_persistent_hello_connection() {
    let server_manager = PeerManager::new();
    let listener = server_manager
        .listen(
            "127.0.0.1:0",
            PeerInfo {
                node_id: "node-a".into(),
                address: "127.0.0.1:0".into(),
            },
        )
        .await
        .unwrap();

    // The listener binds an ephemeral port, so use a dedicated local TCP
    // listener to discover a free port and then restart the manager there.
    listener.abort();

    let probe = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = probe.local_addr().unwrap().to_string();
    drop(probe);

    let listener = server_manager
        .listen(
            &address,
            PeerInfo {
                node_id: "node-a".into(),
                address: address.clone(),
            },
        )
        .await
        .unwrap();

    let client_manager = PeerManager::new();
    let peer = client_manager
        .connect(
            &address,
            PeerInfo {
                node_id: "node-b".into(),
                address: "127.0.0.1:0".into(),
            },
        )
        .await
        .unwrap();

    assert_eq!(peer.node_id, "node-a");

    for _ in 0..20 {
        if server_manager.connection_count().await == 1 {
            break;
        }
        sleep(Duration::from_millis(5)).await;
    }

    assert_eq!(client_manager.connection_count().await, 1);
    assert_eq!(server_manager.connection_count().await, 1);
    assert_eq!(server_manager.peers().await[0].node_id, "node-b");

    client_manager
        .send_to_peer("node-a", &NetworkMessage::Ping { nonce: 42 })
        .await
        .unwrap();

    // The server's reader loop consumes Ping and answers with Pong. Give the
    // asynchronous reader/writer tasks a moment to process the frame.
    sleep(Duration::from_millis(10)).await;

    listener.abort();
}
