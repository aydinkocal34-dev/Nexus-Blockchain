use crate::consensus::{Vote, VoteDecision};
use fs2::FileExt;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
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

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq)]
struct SigningAnchor {
    version: u32,
    record_count: usize,
    records_sha256: String,
}

#[derive(Debug)]
pub struct ConsensusSigningStore {
    path: PathBuf,
    anchor_path: PathBuf,
    records: Vec<SigningRecord>,
    poisoned: bool,
    // Keep the OS lock alive for the entire lifetime of the store.
    _lock_file: File,
}

impl ConsensusSigningStore {
    /// Open a signing history with an integrity anchor. A missing, corrupt, or
    /// mismatched anchor fails closed; it is never silently recreated for an
    /// existing history.
    ///
    /// An exclusive OS-level lock is held until this store is dropped. The
    /// sidecar anchor detects restoring only an older signing-history file.
    /// It cannot detect an attacker restoring both files from the same backup;
    /// that requires an independently protected monotonic anchor.
    pub fn open(path: impl Into<PathBuf>) -> io::Result<Self> {
        let path = path.into();
        let parent = path
            .parent()
            .filter(|p| !p.as_os_str().is_empty())
            .unwrap_or_else(|| Path::new("."));
        fs::create_dir_all(parent)?;

        let mut lock_name = path.file_name().unwrap_or_default().to_os_string();
        lock_name.push(".lock");
        let lock_path = parent.join(lock_name);
        let lock_file = OpenOptions::new()
            .create(true)
            .truncate(false)
            .read(true)
            .write(true)
            .open(lock_path)?;
        FileExt::try_lock_exclusive(&lock_file)?;

        let mut anchor_name = path.file_name().unwrap_or_default().to_os_string();
        anchor_name.push(".anchor");
        let anchor_path = parent.join(anchor_name);

        let history_result = fs::read(&path);
        let anchor_result = fs::read(&anchor_path);
        let (records, initialize_empty) = match (history_result, anchor_result) {
            (Err(history_error), Err(anchor_error))
                if history_error.kind() == io::ErrorKind::NotFound
                    && anchor_error.kind() == io::ErrorKind::NotFound =>
            {
                (Vec::new(), true)
            }
            (Ok(history_bytes), Ok(anchor_bytes)) => {
                let records: Vec<SigningRecord> = serde_json::from_slice(&history_bytes)
                    .map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error))?;
                let anchor: SigningAnchor = serde_json::from_slice(&anchor_bytes)
                    .map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error))?;
                if anchor.version != 1
                    || anchor.record_count != records.len()
                    || anchor.records_sha256 != records_hash(&records)?
                {
                    return Err(io::Error::new(
                        io::ErrorKind::InvalidData,
                        "consensus signing history does not match its integrity anchor",
                    ));
                }
                (records, false)
            }
            (Err(error), _) if error.kind() != io::ErrorKind::NotFound => return Err(error),
            (_, Err(error)) if error.kind() != io::ErrorKind::NotFound => return Err(error),
            _ => {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    "consensus signing history or integrity anchor is missing",
                ));
            }
        };

        let store = Self {
            path,
            anchor_path,
            records,
            poisoned: false,
            _lock_file: lock_file,
        };
        if initialize_empty {
            store.persist(&[])?;
        }
        Ok(store)
    }

    /// Durably reserve a vote before producing or publishing its signature.
    /// A validator may only sign one block/decision per chain and height.
    pub fn reserve_vote(&mut self, chain_id: u64, vote: &Vote) -> Result<(), &'static str> {
        if self.poisoned {
            return Err("consensus signing store is in an unsafe state");
        }
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
        if self.persist(&updated).is_err() {
            // A failure may occur after the history rename but before the
            // anchor rename. Never sign or reserve again from stale memory.
            self.poisoned = true;
            return Err("failed to persist consensus signing lock");
        }
        self.records = updated;
        Ok(())
    }

    fn persist(&self, records: &[SigningRecord]) -> io::Result<()> {
        let parent = self
            .path
            .parent()
            .filter(|p| !p.as_os_str().is_empty())
            .unwrap_or_else(|| Path::new("."));
        fs::create_dir_all(parent)?;
        let bytes = serde_json::to_vec(records)
            .map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error))?;
        let anchor = SigningAnchor {
            version: 1,
            record_count: records.len(),
            records_sha256: records_hash(records)?,
        };
        let anchor_bytes = serde_json::to_vec(&anchor)
            .map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error))?;

        // Commit history first, then its anchor. If interrupted between the
        // two renames, startup detects the mismatch and refuses to sign.
        atomic_replace(&self.path, &bytes)?;
        File::open(parent)?.sync_all()?;
        atomic_replace(&self.anchor_path, &anchor_bytes)?;
        File::open(parent)?.sync_all()?;
        Ok(())
    }
}

fn records_hash(records: &[SigningRecord]) -> io::Result<String> {
    let bytes = serde_json::to_vec(records)
        .map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error))?;
    let digest = Sha256::digest(bytes);
    Ok(digest.iter().map(|byte| format!("{byte:02x}")).collect())
}

fn atomic_replace(path: &Path, bytes: &[u8]) -> io::Result<()> {
    let parent = path
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."));
    let file_name = path.file_name().unwrap_or_default().to_string_lossy();
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(io::Error::other)?
        .as_nanos();
    let tmp = parent.join(format!(".{file_name}.{}.{}.tmp", std::process::id(), nonce));
    let mut file = OpenOptions::new().write(true).create_new(true).open(&tmp)?;
    let write_result = (|| {
        file.write_all(bytes)?;
        file.sync_all()?;
        fs::rename(&tmp, path)?;
        Ok(())
    })();
    if write_result.is_err() {
        let _ = fs::remove_file(&tmp);
    }
    write_result
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

    fn sidecar(path: &std::path::Path, suffix: &str) -> std::path::PathBuf {
        let mut name = path.file_name().unwrap_or_default().to_os_string();
        name.push(suffix);
        path.parent().unwrap().join(name)
    }

    fn cleanup(path: &std::path::Path) {
        let _ = std::fs::remove_file(path);
        let _ = std::fs::remove_file(sidecar(path, ".lock"));
        let _ = std::fs::remove_file(sidecar(path, ".anchor"));
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
    fn restoring_only_an_old_history_file_fails_closed() {
        let path = path();
        let mut store = ConsensusSigningStore::open(&path).unwrap();
        let old_history = std::fs::read(&path).unwrap();
        store.reserve_vote(7, &vote("block-a")).unwrap();
        std::fs::write(&path, old_history).unwrap();
        drop(store);

        let error = ConsensusSigningStore::open(&path).unwrap_err();
        assert_eq!(error.kind(), ErrorKind::InvalidData);
        cleanup(&path);
    }

    #[test]
    fn corrupt_history_fails_closed() {
        let path = path();
        {
            let _store = ConsensusSigningStore::open(&path).unwrap();
        }
        std::fs::write(&path, b"not-json").unwrap();
        assert!(ConsensusSigningStore::open(&path).is_err());
        cleanup(&path);
    }
}
