// ffi.rs — UniFFI-compatible wrappers for the FFI boundary
//
// This module exposes crypto, vault, and store operations across the FFI
// boundary using types that are compatible with UniFFI's code generation.

use std::path::Path;
use std::sync::Mutex;

use crate::crypto::{decrypt, encrypt, generate_salt as crypto_generate_salt, EngramKey};
use crate::store::{Memory, MemoryStore};
use crate::vault::Vault;

/// Errors exposed across the FFI boundary.
/// uniffi::Error semantics are provided by the UDL definition ([Error] enum EngramError).
#[derive(Debug, thiserror::Error)]
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
    let arr: [u8; 32] = bytes
        .try_into()
        .map_err(|_| EngramError::InvalidInput("key must be exactly 32 bytes".to_string()))?;
    Ok(EngramKey::from_bytes(arr))
}

/// Convert a store `Memory` into an FFI-safe `MemoryRecord`.
fn memory_to_record(m: Memory) -> MemoryRecord {
    MemoryRecord {
        id: m.id,
        entity: m.entity,
        attribute: m.attribute,
        value: m.value,
        source: m.source,
        created_at: m.created_at,
        updated_at: m.updated_at,
    }
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

// ── vault wrappers ───────────────────────────────────────────────────────────────

pub fn vault_read(vault_path: String, relative_path: String) -> Result<String, EngramError> {
    let vault = Vault::new(&vault_path);
    vault
        .read(&relative_path)
        .map_err(|e| EngramError::Vault(e.to_string()))
}

pub fn vault_write(
    vault_path: String,
    relative_path: String,
    content: String,
) -> Result<(), EngramError> {
    let vault = Vault::new(&vault_path);
    vault
        .write(&relative_path, &content)
        .map_err(|e| EngramError::Vault(e.to_string()))
}

pub fn vault_list_markdown(vault_path: String) -> Result<Vec<String>, EngramError> {
    let vault = Vault::new(&vault_path);
    vault
        .list_markdown()
        .map_err(|e| EngramError::Vault(e.to_string()))
}

// ── store types & handle ─────────────────────────────────────────────────────

/// An FFI-safe representation of a memory record.
/// uniffi::Record semantics are provided by the UDL definition (dictionary MemoryRecord).
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
/// uniffi::Object semantics are provided by the UDL definition (interface MemoryStoreHandle).
pub struct MemoryStoreHandle {
    inner: Mutex<MemoryStore>,
}

impl MemoryStoreHandle {
    /// Open (or create) an encrypted memory store at `db_path` using `key_bytes`.
    ///
    /// Returns `InvalidInput` if `key_bytes` is not exactly 32 bytes.
    /// Returns `Store` if the database cannot be opened.
    /// Open (or create) an encrypted memory store at `db_path` using `key_bytes`.
    ///
    /// Returns `InvalidInput` if `key_bytes` is not exactly 32 bytes.
    /// Returns `Store` if the database cannot be opened.
    ///
    /// Note: returns `Arc<Self>` so callers (including tests) can share the handle.
    /// The UDL scaffolding calls this constructor and handles the Arc wrapping at the FFI boundary.
    pub fn new(db_path: String, key_bytes: Vec<u8>) -> Result<Self, EngramError> {
        let key = bytes_to_key(key_bytes)?;
        let store = MemoryStore::open(Path::new(&db_path), &key)
            .map_err(|e| EngramError::Store(e.to_string()))?;
        Ok(Self {
            inner: Mutex::new(store),
        })
    }

    /// Create and insert a new memory record for the given entity/attribute/value triple.
    pub fn insert_memory(
        &self,
        entity: String,
        attribute: String,
        value: String,
        source: Option<String>,
    ) -> Result<(), EngramError> {
        let memory = Memory::new(&entity, &attribute, &value, source.as_deref());
        self.inner
            .lock()
            .unwrap()
            .insert(&memory)
            .map_err(|e| EngramError::Store(e.to_string()))
    }

    /// Retrieve a memory record by id, or `None` if not found.
    pub fn get_memory(&self, id: String) -> Result<Option<MemoryRecord>, EngramError> {
        self.inner
            .lock()
            .unwrap()
            .get(&id)
            .map_err(|e| EngramError::Store(e.to_string()))
            .map(|opt| opt.map(memory_to_record))
    }

    /// Return all memory records associated with `entity`.
    pub fn find_by_entity(&self, entity: String) -> Result<Vec<MemoryRecord>, EngramError> {
        self.inner
            .lock()
            .unwrap()
            .find_by_entity(&entity)
            .map_err(|e| EngramError::Store(e.to_string()))
            .map(|vec| vec.into_iter().map(memory_to_record).collect())
    }

    /// Return the total number of records in the store.
    pub fn record_count(&self) -> Result<u64, EngramError> {
        self.inner
            .lock()
            .unwrap()
            .record_count()
            .map_err(|e| EngramError::Store(e.to_string()))
    }
}

// ── tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_vault_write_then_read() {
        let dir = TempDir::new().unwrap();
        let vault_path = dir.path().to_str().unwrap().to_string();
        vault_write(
            vault_path.clone(),
            "note.md".to_string(),
            "hello vault".to_string(),
        )
        .expect("vault_write failed");
        let content = vault_read(vault_path, "note.md".to_string()).expect("vault_read failed");
        assert_eq!(content, "hello vault");
    }

    #[test]
    fn test_vault_list_markdown_returns_only_md_files() {
        let dir = TempDir::new().unwrap();
        let vault_path = dir.path().to_str().unwrap().to_string();
        vault_write(
            vault_path.clone(),
            "a.md".to_string(),
            "content a".to_string(),
        )
        .unwrap();
        vault_write(
            vault_path.clone(),
            "b.md".to_string(),
            "content b".to_string(),
        )
        .unwrap();
        vault_write(
            vault_path.clone(),
            "image.png".to_string(),
            "binary".to_string(),
        )
        .unwrap();
        let files = vault_list_markdown(vault_path).expect("vault_list_markdown failed");
        assert!(
            files.contains(&"a.md".to_string()),
            "a.md missing from list: {:?}",
            files
        );
        assert!(
            files.contains(&"b.md".to_string()),
            "b.md missing from list: {:?}",
            files
        );
        assert!(
            !files.contains(&"image.png".to_string()),
            "image.png should not appear in markdown list: {:?}",
            files
        );
    }

    #[test]
    fn test_vault_read_missing_file_returns_vault_error() {
        let dir = TempDir::new().unwrap();
        let vault_path = dir.path().to_str().unwrap().to_string();
        let result = vault_read(vault_path, "nonexistent.md".to_string());
        assert!(
            matches!(result, Err(EngramError::Vault(_))),
            "expected Err(EngramError::Vault(_)), got: {:?}",
            result
        );
    }

    #[test]
    fn test_ffi_generate_salt_is_16_bytes() {
        let salt = generate_salt();
        assert_eq!(salt.len(), 16, "generate_salt must return exactly 16 bytes");
    }

    #[test]
    fn test_ffi_encrypt_decrypt_roundtrip() {
        let salt = generate_salt();
        let key_bytes = derive_key("test_password".to_string(), salt).expect("derive_key failed");
        let plaintext = b"hello, ffi!".to_vec();
        let ciphertext =
            encrypt_bytes(key_bytes.clone(), plaintext.clone()).expect("encrypt_bytes failed");
        let decrypted = decrypt_bytes(key_bytes, ciphertext).expect("decrypt_bytes failed");
        assert_eq!(
            decrypted, plaintext,
            "decrypted must equal original plaintext"
        );
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

    // ── store tests ───────────────────────────────────────────────────────────

    fn make_test_store() -> (std::sync::Arc<MemoryStoreHandle>, TempDir) {
        let dir = TempDir::new().unwrap();
        let db_path = dir.path().join("test_ffi.db");
        let key_bytes = vec![0u8; 32];
        let handle = MemoryStoreHandle::new(db_path.to_str().unwrap().to_string(), key_bytes)
            .expect("failed to create test store");
        (std::sync::Arc::new(handle), dir)
    }

    #[test]
    fn test_store_insert_get_find_count() {
        let (handle, _dir) = make_test_store();
        handle
            .insert_memory(
                "Sofia".to_string(),
                "dietary".to_string(),
                "vegetarian".to_string(),
                Some("2026-04-14 transcript".to_string()),
            )
            .expect("insert failed");

        assert_eq!(handle.record_count().expect("count failed"), 1);

        let records = handle
            .find_by_entity("Sofia".to_string())
            .expect("find_by_entity failed");
        assert_eq!(records.len(), 1);
        assert_eq!(records[0].entity, "Sofia");
        assert_eq!(records[0].attribute, "dietary");
        assert_eq!(records[0].value, "vegetarian");
        assert_eq!(records[0].source, Some("2026-04-14 transcript".to_string()));

        let record = handle
            .get_memory(records[0].id.clone())
            .expect("get_memory failed");
        assert!(record.is_some(), "expected get_memory to return a record");
        let record = record.unwrap();
        assert_eq!(record.entity, "Sofia");
        assert_eq!(record.value, "vegetarian");
    }

    #[test]
    fn test_store_get_missing_returns_none() {
        let (handle, _dir) = make_test_store();
        let result = handle
            .get_memory("nonexistent-uuid".to_string())
            .expect("get_memory should not error for missing id");
        assert!(result.is_none(), "expected None for missing id");
    }

    #[test]
    fn test_store_wrong_key_length_returns_invalid_input() {
        let dir = TempDir::new().unwrap();
        let db_path = dir.path().join("test_ffi.db");
        let result = MemoryStoreHandle::new(
            db_path.to_str().unwrap().to_string(),
            vec![0u8; 10], // wrong: 10 bytes instead of 32
        );
        assert!(
            matches!(result, Err(EngramError::InvalidInput(_))),
            "expected Err(EngramError::InvalidInput(_)) for 10-byte key, but got a different result"
        );
    }

    #[test]
    fn test_store_concurrent_inserts_do_not_panic() {
        use std::sync::Arc;
        use std::thread;

        let (handle, _dir) = make_test_store();

        let h1 = Arc::clone(&handle);
        let h2 = Arc::clone(&handle);

        let t1 = thread::spawn(move || {
            h1.insert_memory(
                "ThreadA".to_string(),
                "test".to_string(),
                "valueA".to_string(),
                None,
            )
            .expect("ThreadA insert failed");
        });

        let t2 = thread::spawn(move || {
            h2.insert_memory(
                "ThreadB".to_string(),
                "test".to_string(),
                "valueB".to_string(),
                None,
            )
            .expect("ThreadB insert failed");
        });

        t1.join().expect("thread A panicked");
        t2.join().expect("thread B panicked");

        assert_eq!(handle.record_count().expect("count failed"), 2);
    }
}
