use std::io;
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};

use serde::{Deserialize, Serialize};

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
where
    W: AsyncWrite + Unpin,
{
    let payload = serde_json::to_vec(message)
        .map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error))?;

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
where
    R: AsyncRead + Unpin,
{
    let length = reader.read_u32().await? as usize;

    if length > MAX_FRAME_SIZE {
        return Err(io::Error::new(io::ErrorKind::InvalidData, "message too large"));
    }

    let mut payload = vec![0u8; length];
    reader.read_exact(&mut payload).await?;

    serde_json::from_slice(&payload)
        .map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error))
}
