use nexus_blockchain::{NetworkMessage, PeerInfo, PeerManager};
use tokio::net::TcpListener;
use tokio::time::{sleep, Duration};

#[tokio::test(flavor = "current_thread")]
async fn four_node_testnet_topology_connects() {
    let mut nodes = Vec::new();
    let mut listeners = Vec::new();

    for index in 0..4 {
        let manager = PeerManager::new();
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        let local = PeerInfo {
            node_id: format!("testnet-{index}"),
            address: address.to_string(),
        };
        let handle = manager.listen_on(listener, local.clone()).await.unwrap();
        nodes.push((manager, local));
        listeners.push(handle);
    }

    for index in 1..4 {
        let (client, _) = &nodes[index];
        let (_, server_info) = &nodes[0];
        client.connect(&server_info.address, nodes[index].1.clone()).await.unwrap();
    }

    sleep(Duration::from_millis(50)).await;

    assert_eq!(nodes[0].0.connection_count().await, 3);
    assert_eq!(nodes[1].0.connection_count().await, 1);
    assert_eq!(nodes[2].0.connection_count().await, 1);
    assert_eq!(nodes[3].0.connection_count().await, 1);

    let sent = nodes[1].0.broadcast(&NetworkMessage::Ping { nonce: 42 }).await.unwrap();
    assert_eq!(sent, 1);

    for handle in listeners {
        handle.abort();
    }
}
