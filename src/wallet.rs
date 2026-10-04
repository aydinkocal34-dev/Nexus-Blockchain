use argon2::Argon2;
use chacha20poly1305::aead::{Aead, KeyInit};
use chacha20poly1305::{ChaCha20Poly1305, Key, Nonce};
use ed25519_dalek::{Signature, Signer, SigningKey, Verifier, VerifyingKey};
use rand::rngs::OsRng;
use rand::RngCore;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

const KEY_LEN: usize = 32;
const SALT_LEN: usize = 16;
const NONCE_LEN: usize = 12;

#[derive(Debug)]
pub struct Wallet {
    signing_key: SigningKey,
}

#[derive(Debug, Serialize, Deserialize)]
struct EncryptedWallet {
    version: u8,
    salt: Vec<u8>,
    nonce: Vec<u8>,
    ciphertext: Vec<u8>,
}

impl Wallet {
    pub fn new() -> Self {
        Self { signing_key: SigningKey::generate(&mut OsRng) }
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

    pub fn encrypt_keystore(&self, password: &[u8]) -> Result<Vec<u8>, &'static str> {
        if password.is_empty() { return Err("wallet password must not be empty"); }
        let mut salt = [0u8; SALT_LEN];
        let mut nonce = [0u8; NONCE_LEN];
        OsRng.fill_bytes(&mut salt);
        OsRng.fill_bytes(&mut nonce);

        let mut key_bytes = [0u8; KEY_LEN];
        Argon2::default()
            .hash_password_into(password, &salt, &mut key_bytes)
            .map_err(|_| "failed to derive wallet key")?;

        let cipher = ChaCha20Poly1305::new(Key::from_slice(&key_bytes));
        let ciphertext = cipher
            .encrypt(Nonce::from_slice(&nonce), self.signing_key.to_bytes().as_ref())
            .map_err(|_| "failed to encrypt wallet")?;

        serde_json::to_vec(&EncryptedWallet {
            version: 1,
            salt: salt.to_vec(),
            nonce: nonce.to_vec(),
            ciphertext,
        }).map_err(|_| "failed to serialize wallet")
    }

    pub fn from_encrypted_keystore(data: &[u8], password: &[u8]) -> Result<Self, &'static str> {
        let stored: EncryptedWallet = serde_json::from_slice(data).map_err(|_| "invalid wallet file")?;
        if stored.version != 1 || stored.salt.len() != SALT_LEN || stored.nonce.len() != NONCE_LEN {
            return Err("unsupported wallet format");
        }

        let mut key_bytes = [0u8; KEY_LEN];
        Argon2::default()
            .hash_password_into(password, &stored.salt, &mut key_bytes)
            .map_err(|_| "failed to derive wallet key")?;
        let cipher = ChaCha20Poly1305::new(Key::from_slice(&key_bytes));
        let plaintext = cipher
            .decrypt(Nonce::from_slice(&stored.nonce), stored.ciphertext.as_ref())
            .map_err(|_| "invalid wallet password or corrupted wallet")?;
        if plaintext.len() != KEY_LEN { return Err("invalid wallet key"); }

        let mut secret = [0u8; KEY_LEN];
        secret.copy_from_slice(&plaintext);
        Ok(Self { signing_key: SigningKey::from_bytes(&secret) })
    }

    fn verifying_key(&self) -> VerifyingKey {
        self.signing_key.verifying_key()
    }
}

impl Default for Wallet {
    fn default() -> Self { Self::new() }
}

pub fn address_from_public_key(public_key: &[u8; 32]) -> String {
    let digest = Sha256::digest(public_key);
    hex_encode(&digest[..20])
}

fn hex_encode(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}
