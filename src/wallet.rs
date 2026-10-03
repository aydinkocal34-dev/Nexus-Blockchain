use ed25519_dalek::{Signature, Signer, SigningKey, Verifier, VerifyingKey};
use rand::rngs::OsRng;
use sha2::{Digest, Sha256};

#[derive(Debug)]
pub struct Wallet {
    signing_key: SigningKey,
}

impl Wallet {
    pub fn new() -> Self {
        Self {
            signing_key: SigningKey::generate(&mut OsRng),
        }
    }

    pub fn address(&self) -> String {
        address_from_public_key(self.verifying_key().as_bytes())
    }

    pub fn public_key_bytes(&self) -> [u8; 32] {
        self.verifying_key().to_bytes()
    }

    pub fn sign(&self, message: &[u8]) -> [u8; 64] {
        self.signing_key.sign(message).to_bytes()
    }

    pub fn verify(&self, message: &[u8], signature: &[u8; 64]) -> bool {
        self.verifying_key()
            .verify(message, &Signature::from_bytes(signature))
            .is_ok()
    }

    fn verifying_key(&self) -> VerifyingKey {
        self.signing_key.verifying_key()
    }
}

impl Default for Wallet {
    fn default() -> Self {
        Self::new()
    }
}

pub fn address_from_public_key(public_key: &[u8; 32]) -> String {
    let digest = Sha256::digest(public_key);
    hex_encode(&digest[..20])
}

fn hex_encode(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}
