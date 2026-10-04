use std::io;
use std::sync::Arc;

use serde::{Deserialize, Serialize};
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::Mutex;

use crate::{Block, Transaction};

const MAX_FRAME_SIZE: usize = 4 * 1024 * 1024;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PeerInfo {
    pub node_id: String,
    pub address: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum NetworkMessage {
    Hello(PeerInfo),
    Ping { nonce: u64 },
    Pong { nonce: u64 },
    GetLatestBlock,
    LatestBlock(Block),
    GetTransactions,
    Transactions(Vec<Transaction>),
    SubmitTransaction(Transaction),
}

pub async fn write_message<W>(writer: &mut W, message: &NetworkMessage) -> io::Result<()>
where W: AsyncWrite + Unpin {
    let payload = serde_json::to_vec(message)
        .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
    if payload.len() > MAX_FRAME_SIZE {
        return Err(io::Error::new(io::ErrorKind::InvalidData, "message too large"));
    }
    let length = u32::try_from(payload.len())
        .map_err(|_| io::Error::new(io::ErrorKind::InvalidData, "message too large"))?;
    writer.write_u32(length).await?;
    writer.write_all(&payload).await?;
    writer.flush().await
}

pub async fn read_message<R>(reader: &mut R) -> io::Result<NetworkMessage>
where R: AsyncRead + Unpin {
    let length = reader.read_u32().await? as usize;
    if length > MAX_FRAME_SIZE {
        return Err(io::Error::new(io::ErrorKind::InvalidData, "message too large"));
    }
    let mut payload = vec![0u8; length];
    reader.read_exact(&mut payload).await?;
    serde_json::from_slice(&payload)
        .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))
}

#[derive(Debug, Clone)]
pub struct PeerManager {
    peers: Arc<Mutex<Vec<PeerInfo>>>,
}

impl PeerManager {
    pub fn new() -> Self {
        Self { peers: Arc::new(Mutex::new(Vec::new())) }
    }

    pub async fn add_peer(&self, peer: PeerInfo) {
        let mut peers = self.peers.lock().await;
        if !peers.iter().any(|existing| existing.node_id == peer.node_id) {
            peers.push(peer);
        }
    }

    pub async fn peers(&self) -> Vec<PeerInfo> {
        self.peers.lock().await.clone()
    }

    pub async fn connect(&self, address: &str, local: PeerInfo) -> io::Result<PeerInfo> {
        let mut stream = TcpStream::connect(address).await?;
        write_message(&mut stream, &NetworkMessage::Hello(local)).await?;

        let response = read_message(&mut stream).await?;
        let NetworkMessage::Hello(peer) = response else {
            return Err(io::Error::new(io::ErrorKind::InvalidData, "expected hello"));
        };

        self.add_peer(peer.clone()).await;
        Ok(peer)
    }

    pub async fn listen(
        &self,
        address: &str,
        local: PeerInfo,
    ) -> io::Result<tokio::task::JoinHandle<()>> {
        let listener = TcpListener::bind(address).await?;
        let manager = self.clone();

        Ok(tokio::spawn(async move {
            loop {
                let Ok((mut stream, _)) = listener.accept().await else { continue; };
                let manager = manager.clone();
                let local = local.clone();

                tokio::spawn(async move {
                    let Ok(NetworkMessage::Hello(peer)) = read_message(&mut stream).await else {
                        return;
                    };
                    manager.add_peer(peer).await;
                    let _ = write_message(&mut stream, &NetworkMessage::Hello(local)).await;
                });
            }
        }))
    }
}

impl Default for PeerManager {
    fn default() -> Self { Self::new() }
}
