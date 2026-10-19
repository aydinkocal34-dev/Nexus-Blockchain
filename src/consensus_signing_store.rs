use crate::consensus::{Vote, VoteDecision};
use fs2::FileExt;
use serde::{Deserialize, Serialize};
use std::fs::{self, File, OpenOptions};
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
struct SigningRecord {
    chain_id: u64,
    height: u64,
    block_hash: String,
    decision: VoteDecision,
}

#[derive(Debug)]
pub struct ConsensusSigningStore {
    path: PathBuf,
    records: Vec<SigningRecord>,
    // Keep the OS lock alive for the entire lifetime of the store.
    _lock_file: File,
}

impl ConsensusSigningStore {
    /// Open an existing signing history or create an empty one. A corrupt
    /// history is an error: callers must fail closed rather than sign again.
    ///
    /// An exclusive OS-level lock is held until this store is dropped. A
    /// second process using the same path fails to open the store instead of
    /// independently making signing decisions from stale in-memory history.
    pub fn open(path: impl Into<PathBuf>) -> io::Result<Self> {
        let path = path.into();
        let parent = path.parent().unwrap_or_else(|| Path::new("."));
        fs::create_dir_all(parent)?;

        let mut lock_name = path.file_name().unwrap_or_default().to_os_string();
        lock_name.push(".lock");
        let lock_path = parent.join(lock_name);
        let lock_file = OpenOptions::new()
            .create(true)
            .read(true)
            .write(true)
            .open(lock_path)?;
        FileExt::try_lock_exclusive(&lock_file)?;

        let records = match fs::read(&path) {
            Ok(bytes) => serde_json::from_slice(&bytes)
                .map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error))?,
            Err(error) if error.kind() == io::ErrorKind::NotFound => Vec::new(),
            Err(error) => return Err(error),
        };
        Ok(Self {
            path,
            records,
            _lock_file: lock_file,
        })
    }

    /// Durably reserve a vote before producing or publishing its signature.
    /// A validator may only sign one block/decision per chain and height.
    pub fn reserve_vote(&mut self, chain_id: u64, vote: &Vote) -> Result<(), &'static str> {
        if let Some(existing) = self
            .records
            .iter()
            .find(|record| record.chain_id == chain_id && record.height == vote.height)
        {
            if existing.block_hash == vote.block_hash && existing.decision == vote.decision {
                return Ok(());
            }
            return Err("refusing conflicting consensus signature for chain and height");
        }

        let mut updated = self.records.clone();
        updated.push(SigningRecord {
            chain_id,
            height: vote.height,
            block_hash: vote.block_hash.clone(),
            decision: vote.decision,
        });
        self.persist(&updated)
            .map_err(|_| "failed to persist consensus signing lock")?;
        self.records = updated;
        Ok(())
    }

    fn persist(&self, records: &[SigningRecord]) -> io::Result<()> {
        let parent = self.path.parent().unwrap_or_else(|| Path::new("."));
        fs::create_dir_all(parent)?;
        let bytes = serde_json::to_vec(records)
            .map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error))?;
        let file_name = self.path.file_name().unwrap_or_default().to_string_lossy();
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(io::Error::other)?
            .as_nanos();
        let tmp = parent.join(format!(".{file_name}.{}.{}.tmp", std::process::id(), nonce));
        let mut file = OpenOptions::new().write(true).create_new(true).open(&tmp)?;
        let write_result = (|| {
            file.write_all(&bytes)?;
            file.sync_all()?;
            fs::rename(&tmp, &self.path)?;
            File::open(parent)?.sync_all()?;
            Ok(())
        })();
        if write_result.is_err() {
            let _ = fs::remove_file(&tmp);
        }
        write_result
    }
}

#[cfg(test)]
mod tests {
    use super::ConsensusSigningStore;
    use crate::consensus::{Vote, VoteDecision};
    use std::io::ErrorKind;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn path() -> std::path::PathBuf {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        std::env::temp_dir().join(format!(
            "nexus-sign-lock-{}-{nonce}.json",
            std::process::id()
        ))
    }

    fn cleanup(path: &std::path::Path) {
        let _ = std::fs::remove_file(path);
        let mut lock_name = path.file_name().unwrap_or_default().to_os_string();
        lock_name.push(".lock");
        let _ = std::fs::remove_file(path.parent().unwrap().join(lock_name));
    }

    fn vote(hash: &str) -> Vote {
        Vote {
            height: 50,
            block_hash: hash.into(),
            voter: "validator-a".into(),
            decision: VoteDecision::Commit,
        }
    }

    #[test]
    fn conflicting_vote_is_rejected_after_reopen() {
        let path = path();
        {
            let mut store = ConsensusSigningStore::open(&path).unwrap();
            store.reserve_vote(7, &vote("block-a")).unwrap();
        }
        let mut reopened = ConsensusSigningStore::open(&path).unwrap();
        assert_eq!(
            reopened.reserve_vote(7, &vote("block-b")),
            Err("refusing conflicting consensus signature for chain and height")
        );
        drop(reopened);
        cleanup(&path);
    }

    #[test]
    fn same_vote_is_idempotent_and_other_chain_is_independent() {
        let path = path();
        let mut store = ConsensusSigningStore::open(&path).unwrap();
        store.reserve_vote(7, &vote("block-a")).unwrap();
        store.reserve_vote(7, &vote("block-a")).unwrap();
        store.reserve_vote(8, &vote("block-b")).unwrap();
        drop(store);
        cleanup(&path);
    }

    #[test]
    fn second_open_is_rejected_while_exclusive_lock_is_held() {
        let path = path();
        let store = ConsensusSigningStore::open(&path).unwrap();
        let error = match ConsensusSigningStore::open(&path) {
            Ok(_) => panic!("a second store unexpectedly acquired the lock"),
            Err(error) => error,
        };
        assert_eq!(error.kind(), ErrorKind::WouldBlock);
        drop(store);

        let reopened = ConsensusSigningStore::open(&path).unwrap();
        drop(reopened);
        cleanup(&path);
    }

    #[test]
    fn corrupt_history_fails_closed() {
        let path = path();
        std::fs::write(&path, b"not-json").unwrap();
        assert!(ConsensusSigningStore::open(&path).is_err());
        cleanup(&path);
    }
}
