use nexus_blockchain::network::{NetworkMessage, PeerInfo, PeerManager, read_message, write_message};
use tokio::net::TcpListener;
use tokio::time::{sleep, Duration};

#[tokio::test]
async fn peer_manager_establishes_hello_handshake() {
    let manager = PeerManager::new();
    let listener = manager
        .listen(
            "127.0.0.1:0",
            PeerInfo { node_id: "node-a".into(), address: "a".into() },
        )
        .await
        .unwrap();

    let probe = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = probe.local_addr().unwrap();
    drop(probe);

    listener.abort();

    let server = TcpListener::bind(address).await.unwrap();
    let server_task = tokio::spawn(async move {
        let (mut stream, _) = server.accept().await.unwrap();
        let message = read_message(&mut stream).await.unwrap();
        assert!(matches!(message, NetworkMessage::Hello(_)));
        write_message(
            &mut stream,
            &NetworkMessage::Hello(PeerInfo {
                node_id: "node-b".into(),
                address: address.to_string(),
            }),
        ).await.unwrap();
    });

    let client = tokio::net::TcpStream::connect(address).await.unwrap();
    let mut client = client;
    write_message(
        &mut client,
        &NetworkMessage::Hello(PeerInfo {
            node_id: "node-a".into(),
            address: "a".into(),
        }),
    ).await.unwrap();

    let response = read_message(&mut client).await.unwrap();
    assert!(matches!(response, NetworkMessage::Hello(_)));
    server_task.await.unwrap();
    sleep(Duration::from_millis(1)).await;
}
