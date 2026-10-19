use crate::blockchain::Blockchain;
use std::fs::{self, File, OpenOptions};
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Debug, Clone)]
pub struct BlockchainStorage {
    path: PathBuf,
}

impl BlockchainStorage {
    pub fn new(path: impl Into<PathBuf>) -> Self {
        Self { path: path.into() }
    }

    /// Persist a validated chain using a unique temporary file, file sync,
    /// atomic replacement, and parent-directory sync. If any durability step
    /// fails, return an error rather than claiming the save is durable.
    pub fn save(&self, blockchain: &Blockchain) -> io::Result<()> {
        blockchain
            .validate_full()
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
        let bytes = serde_json::to_vec(blockchain)
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
        let parent = self.parent_dir();
        fs::create_dir_all(parent)?;
        atomic_replace(&self.path, &bytes)?;
        File::open(parent)?.sync_all()?;
        Ok(())
    }

    pub fn load(&self) -> io::Result<Blockchain> {
        let bytes = fs::read(&self.path)?;
        let blockchain: Blockchain = serde_json::from_slice(&bytes)
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
        blockchain
            .validate_full()
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
        Ok(blockchain)
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    fn parent_dir(&self) -> &Path {
        self.path
            .parent()
            .filter(|parent| !parent.as_os_str().is_empty())
            .unwrap_or_else(|| Path::new("."))
    }
}

fn atomic_replace(path: &Path, bytes: &[u8]) -> io::Result<()> {
    let parent = path
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."));
    let file_name = path.file_name().unwrap_or_default().to_string_lossy();
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|error| io::Error::new(io::ErrorKind::Other, error))?
        .as_nanos();
    let temporary_path = parent.join(format!(
        ".{file_name}.{}.{}.tmp",
        std::process::id(),
        nonce
    ));

    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&temporary_path)?;
    let result = (|| {
        file.write_all(bytes)?;
        file.sync_all()?;
        drop(file);
        fs::rename(&temporary_path, path)?;
        Ok(())
    })();
    if result.is_err() {
        let _ = fs::remove_file(&temporary_path);
    }
    result
}

#[cfg(test)]
mod tests {
    use super::BlockchainStorage;
    use crate::Blockchain;
    use std::fs;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn temp_path() -> std::path::PathBuf {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        std::env::temp_dir().join(format!("nexus-chain-storage-{}-{nonce}.json", std::process::id()))
    }

    #[test]
    fn save_load_round_trip_validates_chain() {
        let path = temp_path();
        let storage = BlockchainStorage::new(&path);
        let chain = Blockchain::new(1);
        storage.save(&chain).unwrap();
        let loaded = storage.load().unwrap();
        assert_eq!(loaded.latest_block().hash, chain.latest_block().hash);
        let _ = fs::remove_file(path);
    }

    #[test]
    fn corrupt_primary_fails_closed_instead_of_silently_loading_old_backup() {
        let path = temp_path();
        let storage = BlockchainStorage::new(&path);
        fs::write(&path, b"not-json").unwrap();
        assert!(storage.load().is_err());
        let _ = fs::remove_file(path);
    }
}
