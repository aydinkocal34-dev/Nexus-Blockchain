use ed25519_dalek::{Signature, Verifier, VerifyingKey};
use serde::de::Error as DeError;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use sha2::{Digest, Sha256};

fn serialize_signature<S>(
    signature: &Option<[u8; 64]>,
    serializer: S,
) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    match signature {
        None => serializer.serialize_none(),
        Some(bytes) => serializer.serialize_some(bytes.as_slice()),
    }
}

fn deserialize_signature<'de, D>(deserializer: D) -> Result<Option<[u8; 64]>, D::Error>
where
    D: Deserializer<'de>,
{
    let value: Option<Vec<u8>> = Option::deserialize(deserializer)?;

    match value {
        None => Ok(None),
        Some(bytes) if bytes.len() == 64 => {
            let mut signature = [0u8; 64];
            signature.copy_from_slice(&bytes);
            Ok(Some(signature))
        }
        Some(bytes) => Err(D::Error::custom(format!(
            "signature must be exactly 64 bytes, got {}",
            bytes.len()
        ))),
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Transaction {
    pub id: String,
    pub sender: String,
    pub recipient: String,
    pub amount: u64,
    pub nonce: u64,
    pub public_key: Option<[u8; 32]>,
    #[serde(
        serialize_with = "serialize_signature",
        deserialize_with = "deserialize_signature"
    )]
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
            amount: amount,
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
