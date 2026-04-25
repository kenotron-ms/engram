// crypto.rs — Key derivation and encryption primitives

use argon2::{Algorithm, Argon2, Params, Version};
use chacha20poly1305::{
    aead::{Aead, AeadCore, KeyInit, OsRng},
    XChaCha20Poly1305, XNonce,
};
use hex;
use keyring::Entry;
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
#[derive(PartialEq, Eq)]
pub struct EngramKey([u8; 32]);

impl std::fmt::Debug for EngramKey {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("EngramKey([REDACTED])")
    }
}

/// Generate a cryptographically random 16-byte salt.
pub fn generate_salt() -> [u8; 16] {
    let mut salt = [0u8; 16];
    rand::thread_rng().fill_bytes(&mut salt);
    salt
}

impl EngramKey {
    /// Construct an `EngramKey` directly from 32 bytes of raw key material.
    ///
    /// Used by the FFI layer to round-trip key bytes across the ABI boundary.
    pub fn from_bytes(bytes: [u8; 32]) -> Self {
        EngramKey(bytes)
    }

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

/// Encrypt plaintext using XChaCha20-Poly1305.
///
/// A random 24-byte nonce is generated for each call.
/// Output format: `[24-byte nonce][ciphertext + 16-byte auth tag]`.
pub fn encrypt(key: &EngramKey, plaintext: &[u8]) -> Result<Vec<u8>, CryptoError> {
    let cipher = XChaCha20Poly1305::new_from_slice(key.as_bytes())
        .map_err(|e| CryptoError::Encryption(e.to_string()))?;
    let nonce = XChaCha20Poly1305::generate_nonce(&mut OsRng);
    let ciphertext = cipher
        .encrypt(&nonce, plaintext)
        .map_err(|e| CryptoError::Encryption(e.to_string()))?;
    let mut output = nonce.to_vec();
    output.extend_from_slice(&ciphertext);
    Ok(output)
}

/// Decrypt data produced by [`encrypt`].
///
/// Expects `data` to begin with the 24-byte nonce followed by the ciphertext
/// and 16-byte authentication tag.
pub fn decrypt(key: &EngramKey, data: &[u8]) -> Result<Vec<u8>, CryptoError> {
    if data.len() < 24 {
        return Err(CryptoError::Decryption(
            "data too short to contain nonce".to_string(),
        ));
    }
    let (nonce_bytes, ciphertext) = data.split_at(24);
    let nonce = XNonce::from_slice(nonce_bytes);
    let cipher = XChaCha20Poly1305::new_from_slice(key.as_bytes())
        .map_err(|e| CryptoError::Decryption(e.to_string()))?;
    cipher
        .decrypt(nonce, ciphertext)
        .map_err(|e| CryptoError::Decryption(e.to_string()))
}

/// Manages storage and retrieval of `EngramKey` in the platform keychain.
///
/// Uses the `keyring` crate to interface with macOS Keychain, Windows Credential
/// Manager, or Linux libsecret. Keys are stored as hex-encoded strings.
pub struct KeyStore {
    service: String,
}

impl KeyStore {
    /// Create a new `KeyStore` bound to the given service name.
    pub fn new(service: &str) -> Self {
        Self {
            service: service.to_string(),
        }
    }

    /// Build a keyring `Entry` for this store's service.
    fn entry(&self) -> Result<Entry, CryptoError> {
        Entry::new(&self.service, "key").map_err(|e| CryptoError::Keyring(e.to_string()))
    }

    /// Store an `EngramKey` in the platform keychain as a hex-encoded string.
    ///
    /// If an entry already exists for this service, it is replaced.
    pub fn store(&self, key: &EngramKey) -> Result<(), CryptoError> {
        let hex_key = hex::encode(key.as_bytes());
        let entry = self.entry()?;
        // Delete any pre-existing entry so that set_password doesn't fail with
        // "item already exists" on platforms that do not support upsert.
        entry.delete_password().ok();
        entry
            .set_password(&hex_key)
            .map_err(|e| CryptoError::Keyring(e.to_string()))
    }

    /// Retrieve an `EngramKey` from the platform keychain.
    ///
    /// Returns an error if the key does not exist or cannot be decoded.
    pub fn retrieve(&self) -> Result<EngramKey, CryptoError> {
        let hex_key = self
            .entry()?
            .get_password()
            .map_err(|e| CryptoError::Keyring(e.to_string()))?;
        let bytes = hex::decode(&hex_key).map_err(|e| CryptoError::Keyring(e.to_string()))?;
        if bytes.len() != 32 {
            return Err(CryptoError::Keyring(format!(
                "expected 32 bytes, got {}",
                bytes.len()
            )));
        }
        let mut key_bytes = [0u8; 32];
        key_bytes.copy_from_slice(&bytes);
        Ok(EngramKey(key_bytes))
    }

    /// Delete the stored key from the platform keychain.
    pub fn delete(&self) -> Result<(), CryptoError> {
        self.entry()?
            .delete_password()
            .map_err(|e| CryptoError::Keyring(e.to_string()))
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
        assert_ne!(
            key1, key2,
            "different passwords must produce different keys"
        );
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

    #[test]
    fn test_encrypt_decrypt_roundtrip() {
        let salt = [0u8; 16];
        let key = EngramKey::derive(b"test_password", &salt).expect("derive failed");
        let plaintext = b"hello, engram!";
        let ciphertext = encrypt(&key, plaintext).expect("encrypt failed");
        let decrypted = decrypt(&key, &ciphertext).expect("decrypt failed");
        assert_eq!(
            decrypted, plaintext,
            "decrypted must equal original plaintext"
        );
    }

    #[test]
    fn test_encrypt_output_longer_than_input() {
        let salt = [0u8; 16];
        let key = EngramKey::derive(b"test_password", &salt).expect("derive failed");
        let plaintext = b"hello";
        let ciphertext = encrypt(&key, plaintext).expect("encrypt failed");
        assert_eq!(
            ciphertext.len(),
            plaintext.len() + 24 + 16,
            "ciphertext must be plaintext + 24-byte nonce + 16-byte auth tag"
        );
    }

    #[test]
    fn test_tampered_ciphertext_fails_decryption() {
        let salt = [0u8; 16];
        let key = EngramKey::derive(b"test_password", &salt).expect("derive failed");
        let plaintext = b"hello, engram!";
        let mut ciphertext = encrypt(&key, plaintext).expect("encrypt failed");
        // Flip a bit at index 30 (within the ciphertext+auth region)
        ciphertext[30] ^= 0x01;
        let result = decrypt(&key, &ciphertext);
        assert!(result.is_err(), "tampered ciphertext must fail decryption");
    }

    #[test]
    fn test_different_nonces_each_encrypt() {
        let salt = [0u8; 16];
        let key = EngramKey::derive(b"test_password", &salt).expect("derive failed");
        let plaintext = b"same message";
        let ct1 = encrypt(&key, plaintext).expect("encrypt failed");
        let ct2 = encrypt(&key, plaintext).expect("encrypt failed");
        // First 24 bytes are the nonce
        assert_ne!(
            &ct1[..24],
            &ct2[..24],
            "each encrypt must generate a unique nonce"
        );
    }

    #[test]
    #[ignore = "requires interactive keychain access; run with --include-ignored in a GUI session"]
    fn test_store_and_retrieve_key() {
        let salt = [42u8; 16];
        let key = EngramKey::derive(b"test_password", &salt).expect("derive failed");
        let store = KeyStore::new("engram-test-suite");
        store.store(&key).expect("store failed");
        let retrieved = store.retrieve().expect("retrieve failed");
        assert_eq!(
            key.as_bytes(),
            retrieved.as_bytes(),
            "retrieved key must equal stored key"
        );
        store.delete().ok(); // cleanup
    }

    #[test]
    fn test_retrieve_missing_key_returns_error() {
        let store = KeyStore::new("engram-test-nonexistent-9999");
        store.delete().ok(); // ensure clean state
        let result = store.retrieve();
        assert!(
            result.is_err(),
            "retrieving a missing key must return an error"
        );
    }

    #[test]
    fn test_from_bytes_round_trips_with_as_bytes() {
        let original_bytes = [42u8; 32];
        let key = EngramKey::from_bytes(original_bytes);
        assert_eq!(key.as_bytes(), &original_bytes);
    }

    #[test]
    #[ignore = "requires interactive keychain access; run with --include-ignored in a GUI session"]
    fn test_delete_key_then_retrieve_fails() {
        let salt = [42u8; 16];
        let key = EngramKey::derive(b"test_password", &salt).expect("derive failed");
        let store = KeyStore::new("engram-test-delete-verify");
        store.store(&key).expect("store failed");
        store.delete().expect("delete failed");
        let result = store.retrieve();
        assert!(
            result.is_err(),
            "retrieving after delete must return an error"
        );
    }
}
