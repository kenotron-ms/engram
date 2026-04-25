// crates/engram-sync/src/auth.rs
//
// Credential storage for sync backends via platform keychain.
//
// Service name:  "engram-sync"
// Username:      "<backend>:<key_name>"   e.g. "s3:access_key", "onedrive:access_token"
//
// Uses keyring 2.x API — note: delete_password() not delete_credential().

use keyring::Entry;
use thiserror::Error;

const SERVICE: &str = "engram-sync";

#[derive(Debug, Error)]
pub enum AuthError {
    #[error("Credential not found for {backend}:{key}")]
    NotFound { backend: String, key: String },
    #[error("Keyring error: {0}")]
    Keyring(String),
}

pub struct AuthStore;

impl AuthStore {
    /// Store a credential in the platform keychain.
    pub fn store(backend: &str, key: &str, value: &str) -> Result<(), AuthError> {
        let username = format!("{}:{}", backend, key);
        let entry = Entry::new(SERVICE, &username)
            .map_err(|e| AuthError::Keyring(e.to_string()))?;
        // Delete any pre-existing entry to avoid "already exists" errors on some platforms.
        entry.delete_password().ok();
        entry
            .set_password(value)
            .map_err(|e| AuthError::Keyring(e.to_string()))
    }

    /// Retrieve a credential from the platform keychain.
    pub fn retrieve(backend: &str, key: &str) -> Result<String, AuthError> {
        let username = format!("{}:{}", backend, key);
        Entry::new(SERVICE, &username)
            .map_err(|e| AuthError::Keyring(e.to_string()))?
            .get_password()
            .map_err(|_| AuthError::NotFound {
                backend: backend.to_string(),
                key: key.to_string(),
            })
    }

    /// Delete a credential from the platform keychain.
    pub fn delete(backend: &str, key: &str) -> Result<(), AuthError> {
        let username = format!("{}:{}", backend, key);
        Entry::new(SERVICE, &username)
            .map_err(|e| AuthError::Keyring(e.to_string()))?
            .delete_password()
            .map_err(|e| AuthError::Keyring(e.to_string()))
    }

    /// Returns true only if all `required_keys` are present in the keychain for `backend`.
    pub fn is_configured(backend: &str, required_keys: &[&str]) -> bool {
        required_keys
            .iter()
            .all(|key| Self::retrieve(backend, key).is_ok())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[ignore = "requires interactive keychain access; run with --include-ignored in a GUI session"]
    fn test_store_and_retrieve() {
        AuthStore::store("test-backend", "test-key", "test-value").unwrap();
        let val = AuthStore::retrieve("test-backend", "test-key").unwrap();
        assert_eq!(val, "test-value");
        AuthStore::delete("test-backend", "test-key").unwrap();
    }

    #[test]
    fn test_retrieve_missing_returns_error() {
        // No keychain write — safe to run in CI.
        // Uses a key name that is astronomically unlikely to exist.
        assert!(AuthStore::retrieve("test-backend-ci", "key-that-does-not-exist-xyz").is_err());
    }

    #[test]
    #[ignore = "requires interactive keychain access; run with --include-ignored in a GUI session"]
    fn test_is_configured_all_present() {
        AuthStore::store("s3", "ci-access-key", "key1").unwrap();
        AuthStore::store("s3", "ci-secret-key", "key2").unwrap();
        assert!(AuthStore::is_configured("s3", &["ci-access-key", "ci-secret-key"]));
        AuthStore::delete("s3", "ci-access-key").unwrap();
        AuthStore::delete("s3", "ci-secret-key").unwrap();
    }

    #[test]
    fn test_is_configured_partial_returns_false() {
        // No keychain write — safe to run in CI.
        assert!(!AuthStore::is_configured("s3", &["key-missing-xyz", "another-missing-xyz"]));
    }
}
