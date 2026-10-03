use ed25519_dalek::{Signature, Verifier, VerifyingKey};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Transaction {
    pub id: String,
    pub sender: String,
    pub recipient: String,
    pub amount: u64,
    pub nonce: u64,
    pub public_key: Option<[u8; 32]>,
    pub signature: Option<[u8; 64]>,
}

impl Transaction {
    pub fn new(
        id: impl Into<String>,
        sender: impl Into<String>,
        recipient: impl Into<String>,
        amount: u64,
        nonce: u64,
    ) -> Self {
        Self {
            id: id.into(),
            sender: sender.into(),
            recipient: recipient.into(),
            amount,
            nonce,
            public_key: None,
            signature: None,
        }
    }

    pub fn signing_bytes(&self) -> Vec<u8> {
        serde_json::to_vec(&(
            &self.id,
            &self.sender,
            &self.recipient,
            self.amount,
            self.nonce,
            &self.public_key,
        ))
        .expect("transaction signing payload must serialize")
    }

    pub fn id_hash(&self) -> String {
        let digest = Sha256::digest(self.signing_bytes());
        digest.iter().map(|byte| format!("{byte:02x}")).collect()
    }

    pub fn verify_signature(&self) -> bool {
        let (Some(public_key), Some(signature)) = (self.public_key, self.signature) else {
            return false;
        };

        let Ok(verifying_key) = VerifyingKey::from_bytes(&public_key) else {
            return false;
        };

        verifying_key
            .verify(&self.signing_bytes(), &Signature::from_bytes(&signature))
            .is_ok()
    }

    pub fn with_signature(mut self, public_key: [u8; 32], signature: [u8; 64]) -> Self {
        self.public_key = Some(public_key);
        self.signature = Some(signature);
        self
    }
}
