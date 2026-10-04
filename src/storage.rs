use crate::blockchain::Blockchain;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone)]
pub struct BlockchainStorage {
    path: PathBuf,
}

impl BlockchainStorage {
    pub fn new(path: impl Into<PathBuf>) -> Self { Self { path: path.into() } }

    pub fn save(&self, blockchain: &Blockchain) -> io::Result<()> {
        blockchain.validate_full().map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
        let bytes = serde_json::to_vec(blockchain)
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
        let parent = self.path.parent().unwrap_or_else(|| Path::new("."));
        fs::create_dir_all(parent)?;
        let tmp = self.path.with_extension("tmp");
        fs::write(&tmp, bytes)?;
        fs::rename(tmp, &self.path)?;
        Ok(())
    }

    pub fn load(&self) -> io::Result<Blockchain> {
        let bytes = fs::read(&self.path)?;
        let blockchain: Blockchain = serde_json::from_slice(&bytes)
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
        blockchain.validate_full().map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
        Ok(blockchain)
    }

    pub fn path(&self) -> &Path { &self.path }
}
