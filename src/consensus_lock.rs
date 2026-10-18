use crate::consensus::{Vote, VoteDecision};
use serde::{Deserialize, Serialize};
use std::fs::{self, File, OpenOptions};
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

static TEMP_SEQUENCE: AtomicU64 = AtomicU64::new(0);

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
struct PersistedVote {
    height: u64,
    block_hash: String,
    decision: VoteDecision,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
struct PersistedLock {
    format_version: u32,
    chain_id: u64,
    votes: Vec<PersistedVote>,
}

/// Durable local signing history. A vote is recorded and synced before its
/// signature may be emitted. Corrupt or cross-chain state fails closed.
#[derive(Debug, Clone)]
pub struct ConsensusSignLock {
    path: PathBuf,
    chain_id: u64,
    votes: Vec<PersistedVote>,
}

impl ConsensusSignLock {
    pub fn open(path: impl Into<PathBuf>, chain_id: u64) -> io::Result<Self> {
        let path = path.into();
        let persisted = match fs::read(&path) {
            Ok(bytes) => {
                let persisted: PersistedLock = serde_json::from_slice(&bytes)
                    .map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error))?;
                if persisted.format_version != 1 {
                    return Err(io::Error::new(
                        io::ErrorKind::InvalidData,
                        "unsupported consensus lock format",
                    ));
                }
                if persisted.chain_id != chain_id {
                    return Err(io::Error::new(
                        io::ErrorKind::InvalidData,
                        "consensus lock chain ID mismatch",
                    ));
                }
                persisted
            }
            Err(error) if error.kind() == io::ErrorKind::NotFound => PersistedLock {
                format_version: 1,
                chain_id,
                votes: Vec::new(),
            },
            Err(error) => return Err(error),
        };

        Ok(Self {
            path,
            chain_id,
            votes: persisted.votes,
        })
    }

    /// Persist a local vote before signing it. Repeating the exact same vote
    /// is idempotent; conflicting votes at a height and height rollback fail.
    pub fn reserve_vote(&mut self, vote: &Vote) -> Result<(), &'static str> {
        if let Some(existing) = self.votes.iter().find(|entry| entry.height == vote.height) {
            if existing.block_hash == vote.block_hash && existing.decision == vote.decision {
                return Ok(());
            }
            return Err("conflicting vote already reserved at this height");
        }

        if self.votes.iter().any(|entry| entry.height > vote.height) {
            return Err("cannot reserve a vote below the highest signed height");
        }

        let entry = PersistedVote {
            height: vote.height,
            block_hash: vote.block_hash.clone(),
            decision: vote.decision,
        };
        let mut next_votes = self.votes.clone();
        next_votes.push(entry);
        next_votes.sort_by_key(|entry| entry.height);
        self.persist(&next_votes)
            .map_err(|_| "failed to persist consensus signing lock")?;
        self.votes = next_votes;
        Ok(())
    }

    fn persist(&self, votes: &[PersistedVote]) -> io::Result<()> {
        let bytes = serde_json::to_vec(&PersistedLock {
            format_version: 1,
            chain_id: self.chain_id,
            votes: votes.to_vec(),
        })
        .map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error))?;
        let parent = self.path.parent().unwrap_or_else(|| Path::new("."));
        fs::create_dir_all(parent)?;

        let sequence = TEMP_SEQUENCE.fetch_add(1, Ordering::Relaxed);
        let file_name = self
            .path
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("consensus-lock");
        let tmp = parent.join(format!(
            ".{file_name}.{}.{}.tmp",
            std::process::id(),
            sequence
        ));

        let write_result = (|| {
            let mut file = OpenOptions::new().write(true).create_new(true).open(&tmp)?;
            file.write_all(&bytes)?;
            file.sync_all()?;
            drop(file);
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
    use super::ConsensusSignLock;
    use crate::consensus::{Vote, VoteDecision};
    use std::fs;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn temp_path(label: &str) -> std::path::PathBuf {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        std::env::temp_dir().join(format!("nexus-{label}-{}-{nonce}.json", std::process::id()))
    }

    fn vote(height: u64, block_hash: &str) -> Vote {
        Vote {
            height,
            block_hash: block_hash.into(),
            voter: "validator-a".into(),
            decision: VoteDecision::Commit,
        }
    }

    #[test]
    fn conflicting_vote_is_rejected_after_reopening_storage() {
        let path = temp_path("sign-lock-restart");
        {
            let mut lock = ConsensusSignLock::open(&path, 42).unwrap();
            lock.reserve_vote(&vote(10, "block-a")).unwrap();
        }

        let mut restarted = ConsensusSignLock::open(&path, 42).unwrap();
        assert_eq!(
            restarted.reserve_vote(&vote(10, "block-b")),
            Err("conflicting vote already reserved at this height")
        );
        assert!(restarted.reserve_vote(&vote(10, "block-a")).is_ok());
        let _ = fs::remove_file(path);
    }

    #[test]
    fn older_height_is_rejected_after_newer_vote() {
        let path = temp_path("sign-lock-rollback");
        let mut lock = ConsensusSignLock::open(&path, 42).unwrap();
        lock.reserve_vote(&vote(11, "block-b")).unwrap();
        assert_eq!(
            lock.reserve_vote(&vote(10, "block-a")),
            Err("cannot reserve a vote below the highest signed height")
        );
        let _ = fs::remove_file(path);
    }

    #[test]
    fn corrupt_or_wrong_chain_lock_fails_closed() {
        let path = temp_path("sign-lock-corrupt");
        fs::write(&path, b"not-json").unwrap();
        assert!(ConsensusSignLock::open(&path, 42).is_err());
        fs::write(
            &path,
            serde_json::to_vec(&super::PersistedLock {
                format_version: 1,
                chain_id: 99,
                votes: Vec::new(),
            })
            .unwrap(),
        )
        .unwrap();
        assert!(ConsensusSignLock::open(&path, 42).is_err());
        let _ = fs::remove_file(path);
    }
}
