//! Cryptographic utilities for the Solace Protocol

use crate::error::{CryptoError, Result};
use ed25519_dalek::{Signature as Ed25519Signature, Signer, SigningKey, Verifier, VerifyingKey};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

/// Key pair for signing and verification
#[derive(Debug)]
pub struct KeyPair {
    signing_key: SigningKey,
    verifying_key: VerifyingKey,
}

impl KeyPair {
    /// Generate a new random key pair
    pub fn generate() -> Result<Self> {
        let mut rng = rand::thread_rng();
        let signing_key = SigningKey::generate(&mut rng);
        let verifying_key = signing_key.verifying_key();
        
        Ok(Self {
            signing_key,
            verifying_key,
        })
    }

    /// Get the public verifying key
    pub fn verifying_key(&self) -> &VerifyingKey {
        &self.verifying_key
    }

    /// Sign a message
    pub fn sign(&self, message: &[u8]) -> Signature {
        let sig = self.signing_key.sign(message);
        Signature(sig)
    }
}

/// Digital signature wrapper
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Signature(Ed25519Signature);

impl Signature {
    /// Verify the signature against a message and public key
    pub fn verify(&self, message: &[u8], public_key: &VerifyingKey) -> Result<()> {
        public_key.verify(message, &self.0)
            .map_err(|_| CryptoError::SignatureVerificationFailed.into())
    }

    /// Convert to bytes
    pub fn to_bytes(&self) -> [u8; 64] {
        self.0.to_bytes()
    }

    /// Create from bytes
    pub fn from_bytes(bytes: &[u8; 64]) -> Result<Self> {
        Ed25519Signature::from_bytes(bytes)
            .map(Signature)
            .map_err(|_| CryptoError::InvalidKeyFormat.into())
    }
}

#[derive(Debug, thiserror::Error)]
pub enum SignatureError {
    #[error("Invalid signature format")]
    InvalidFormat,
    #[error("Signature verification failed")]
    VerificationFailed,
}

/// Hash a message using SHA-256
pub fn hash_message(data: &[u8]) -> Result<[u8; 32]> {
    let mut hasher = Sha256::new();
    hasher.update(data);
    Ok(hasher.finalize().into())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_keypair_generation() {
        let keypair = KeyPair::generate().unwrap();
        let message = b"test message";
        
        let signature = keypair.sign(message);
        assert!(signature.verify(message, keypair.verifying_key()).is_ok());
    }

    #[test]
    fn test_hash_message() {
        let data = b"test data";
        let hash1 = hash_message(data).unwrap();
        let hash2 = hash_message(data).unwrap();
        assert_eq!(hash1, hash2);
    }
} 