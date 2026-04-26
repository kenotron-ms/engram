// ffi.rs — UniFFI-compatible wrappers for the FFI boundary
//
// This module exposes crypto, vault, and store operations across the FFI
// boundary using types that are compatible with UniFFI's code generation.
//
// NOTE: uniffi derive macros (#[derive(uniffi::Error)], #[derive(uniffi::Record)],
// #[derive(uniffi::Object)], #[uniffi::export]) are temporarily removed because
// the UniFfiTag scaffolding type is generated in Task 6.  They will be restored
// when the full UniFFI scaffolding is wired into lib.rs.

use std::sync::Mutex;
use thiserror::Error;

use crate::crypto::{decrypt, encrypt, generate_salt as crypto_generate_salt, EngramKey};
use crate::store::MemoryStore;

/// Errors exposed across the FFI boundary.
#[derive(Debug, Error)]
pub enum EngramError {
    #[error("crypto error: {0}")]
    Crypto(String),

    #[error("vault error: {0}")]
    Vault(String),

    #[error("store error: {0}")]
    Store(String),

    #[error("invalid input: {0}")]
    InvalidInput(String),
}

// ── helpers ──────────────────────────────────────────────────────────────────

/// Convert a byte vector to an EngramKey, returning InvalidInput if wrong length.
fn bytes_to_key(bytes: Vec<u8>) -> Result<EngramKey, EngramError> {
    let arr: [u8; 32] = bytes.try_into().map_err(|_| {
        EngramError::InvalidInput("key must be exactly 32 bytes".to_string())
    })?;
    Ok(EngramKey::from_bytes(arr))
}

// ── crypto wrappers ───────────────────────────────────────────────────────────

/// Derive a 32-byte key from `password` and `salt`.
///
/// `salt` must be exactly 16 bytes; otherwise `InvalidInput` is returned.
pub fn derive_key(password: String, salt: Vec<u8>) -> Result<Vec<u8>, EngramError> {
    if salt.len() != 16 {
        return Err(EngramError::InvalidInput(format!(
            "salt must be exactly 16 bytes, got {}",
            salt.len()
        )));
    }
    let salt_arr: [u8; 16] = salt.try_into().unwrap();
    let key = EngramKey::derive(password.as_bytes(), &salt_arr)
        .map_err(|e| EngramError::Crypto(e.to_string()))?;
    Ok(key.as_bytes().to_vec())
}

/// Generate a fresh 16-byte random salt.
pub fn generate_salt() -> Vec<u8> {
    crypto_generate_salt().to_vec()
}

/// Encrypt `plaintext` with the 32-byte `key_bytes`.
///
/// Returns `InvalidInput` if `key_bytes` is not exactly 32 bytes.
pub fn encrypt_bytes(key_bytes: Vec<u8>, plaintext: Vec<u8>) -> Result<Vec<u8>, EngramError> {
    let key = bytes_to_key(key_bytes)?;
    encrypt(&key, &plaintext).map_err(|e| EngramError::Crypto(e.to_string()))
}

/// Decrypt `ciphertext` with the 32-byte `key_bytes`.
///
/// Returns `InvalidInput` if `key_bytes` is not exactly 32 bytes.
pub fn decrypt_bytes(key_bytes: Vec<u8>, ciphertext: Vec<u8>) -> Result<Vec<u8>, EngramError> {
    let key = bytes_to_key(key_bytes)?;
    decrypt(&key, &ciphertext).map_err(|e| EngramError::Crypto(e.to_string()))
}

// ── vault stubs ───────────────────────────────────────────────────────────────

pub fn vault_read(_vault_path: String, _relative_path: String) -> Result<String, EngramError> {
    todo!("vault_read stub")
}

pub fn vault_write(
    _vault_path: String,
    _relative_path: String,
    _content: String,
) -> Result<(), EngramError> {
    todo!("vault_write stub")
}

pub fn vault_list_markdown(_vault_path: String) -> Result<Vec<String>, EngramError> {
    todo!("vault_list_markdown stub")
}

// ── store types & handle ─────────────────────────────────────────────────────

/// An FFI-safe representation of a memory record.
///
/// NOTE: #[derive(uniffi::Record)] will be restored in Task 6.
#[derive(Debug, Clone)]
pub struct MemoryRecord {
    pub id: String,
    pub entity: String,
    pub attribute: String,
    pub value: String,
    pub source: Option<String>,
    pub created_at: i64,
    pub updated_at: i64,
}

/// A thread-safe, opaque handle to the encrypted memory store.
///
/// NOTE: #[derive(uniffi::Object)] and #[uniffi::export] will be restored in Task 6.
pub struct MemoryStoreHandle {
    inner: Mutex<MemoryStore>,
}

impl MemoryStoreHandle {
    pub fn new(_db_path: String, _key_bytes: Vec<u8>) -> Result<Self, EngramError> {
        todo!("MemoryStoreHandle::new stub")
    }

    pub fn insert_memory(
        &self,
        _entity: String,
        _attribute: String,
        _value: String,
        _source: Option<String>,
    ) -> Result<(), EngramError> {
        todo!("insert_memory stub")
    }

    pub fn get_memory(&self, _id: String) -> Result<Option<MemoryRecord>, EngramError> {
        todo!("get_memory stub")
    }

    pub fn find_by_entity(&self, _entity: String) -> Result<Vec<MemoryRecord>, EngramError> {
        todo!("find_by_entity stub")
    }

    pub fn record_count(&self) -> Result<u64, EngramError> {
        todo!("record_count stub")
    }
}

// ── tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ffi_generate_salt_is_16_bytes() {
        let salt = generate_salt();
        assert_eq!(salt.len(), 16, "generate_salt must return exactly 16 bytes");
    }

    #[test]
    fn test_ffi_encrypt_decrypt_roundtrip() {
        let salt = generate_salt();
        let key_bytes =
            derive_key("test_password".to_string(), salt).expect("derive_key failed");
        let plaintext = b"hello, ffi!".to_vec();
        let ciphertext =
            encrypt_bytes(key_bytes.clone(), plaintext.clone()).expect("encrypt_bytes failed");
        let decrypted = decrypt_bytes(key_bytes, ciphertext).expect("decrypt_bytes failed");
        assert_eq!(decrypted, plaintext, "decrypted must equal original plaintext");
    }

    #[test]
    fn test_derive_key_wrong_salt_length_returns_invalid_input() {
        let bad_salt = vec![1u8; 8]; // wrong length
        let result = derive_key("password".to_string(), bad_salt);
        assert!(
            matches!(result, Err(EngramError::InvalidInput(_))),
            "wrong salt length must return InvalidInput, got: {:?}",
            result
        );
    }

    #[test]
    fn test_encrypt_bytes_wrong_key_length_returns_invalid_input() {
        let bad_key = vec![1u8; 16]; // wrong length (need 32)
        let result = encrypt_bytes(bad_key, b"plaintext".to_vec());
        assert!(
            matches!(result, Err(EngramError::InvalidInput(_))),
            "wrong key length must return InvalidInput, got: {:?}",
            result
        );
    }

    #[test]
    fn test_decrypt_bytes_wrong_key_length_returns_invalid_input() {
        let bad_key = vec![1u8; 16]; // wrong length (need 32)
        let result = decrypt_bytes(bad_key, b"fake_ciphertext".to_vec());
        assert!(
            matches!(result, Err(EngramError::InvalidInput(_))),
            "wrong key length must return InvalidInput, got: {:?}",
            result
        );
    }
}
