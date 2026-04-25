// crypto.rs — Key derivation and encryption primitives

use argon2::{Algorithm, Argon2, Params, Version};
use rand::RngCore;
use thiserror::Error;

/// Errors produced by cryptographic operations.
#[derive(Debug, Error)]
pub enum CryptoError {
    #[error("key derivation failed: {0}")]
    Derivation(String),

    #[error("encryption failed: {0}")]
    Encryption(String),

    #[error("decryption failed: {0}")]
    Decryption(String),

    #[error("keyring error: {0}")]
    Keyring(String),
}

/// A 32-byte derived encryption key.
#[derive(Debug, PartialEq, Eq)]
pub struct EngramKey([u8; 32]);

/// Generate a cryptographically random 16-byte salt.
pub fn generate_salt() -> [u8; 16] {
    let mut salt = [0u8; 16];
    rand::thread_rng().fill_bytes(&mut salt);
    salt
}

impl EngramKey {
    /// Derive an `EngramKey` from a password and salt using Argon2id.
    ///
    /// Parameters: 64 MiB memory, 3 iterations, 1 thread, 32-byte output.
    pub fn derive(password: &[u8], salt: &[u8; 16]) -> Result<Self, CryptoError> {
        // 64 MiB = 65536 KiB (m_cost is in kibibytes)
        let params = Params::new(65536, 3, 1, Some(32))
            .map_err(|e| CryptoError::Derivation(e.to_string()))?;

        let argon2 = Argon2::new(Algorithm::Argon2id, Version::V0x13, params);

        let mut key_bytes = [0u8; 32];
        argon2
            .hash_password_into(password, salt, &mut key_bytes)
            .map_err(|e| CryptoError::Derivation(e.to_string()))?;

        Ok(EngramKey(key_bytes))
    }

    /// Return a reference to the raw 32-byte key material.
    pub fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_same_password_and_salt_produces_same_key() {
        let password = b"correct horse battery staple";
        let salt = [0u8; 16];
        let key1 = EngramKey::derive(password, &salt).expect("derive failed");
        let key2 = EngramKey::derive(password, &salt).expect("derive failed");
        assert_eq!(key1, key2, "same inputs must produce same key");
    }

    #[test]
    fn test_different_passwords_produce_different_keys() {
        let salt = [1u8; 16];
        let key1 = EngramKey::derive(b"password_one", &salt).expect("derive failed");
        let key2 = EngramKey::derive(b"password_two", &salt).expect("derive failed");
        assert_ne!(key1, key2, "different passwords must produce different keys");
    }

    #[test]
    fn test_different_salts_produce_different_keys() {
        let password = b"same_password";
        let salt1 = [2u8; 16];
        let salt2 = [3u8; 16];
        let key1 = EngramKey::derive(password, &salt1).expect("derive failed");
        let key2 = EngramKey::derive(password, &salt2).expect("derive failed");
        assert_ne!(key1, key2, "different salts must produce different keys");
    }

    #[test]
    fn test_generate_salt_produces_16_bytes() {
        let salt = generate_salt();
        assert_eq!(salt.len(), 16, "salt must be exactly 16 bytes");
    }
}
