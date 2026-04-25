# Engram — Phase 3: Sync Backends + Auth

> **For agentic workers:** REQUIRED SUB-SKILL: Use `superpowers:subagent-driven-development` (recommended) or `superpowers:executing-plans` to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build `engram-sync` — a new Rust crate providing encrypted sync to four cloud storage backends (S3-compatible, Azure Blob, Google Cloud Storage, OneDrive via Microsoft Graph). Add `engram auth` and `engram sync` CLI commands. The backend never sees plaintext — all encryption via XChaCha20-Poly1305 before bytes leave the device.

**Architecture:** New `engram-sync` crate defines a `SyncBackend` async trait. Four implementations: S3/Azure/GCS via the `object_store` crate (single trait, pluggable transports), OneDrive via Microsoft Graph REST API (reqwest). `AuthStore` manages credentials in the platform keychain. CLI adds `engram auth add|list|remove` and `engram sync`. Integration tests verify the full encrypt-push-pull-decrypt round-trip using `LocalFileSystem` as a stand-in for real backends.

**Tech Stack:** object_store 0.11 (aws/azure/gcp features), reqwest 0.12 (json + blocking), oauth2 4, open 5, rpassword 7, keyring 2, bytes 1, futures 0.3 (StreamExt for stream collection), async-trait 0.1, tokio 1 (full), serde/serde_json 1

---

## Codebase Orientation

Before starting, understand these existing patterns:

- **`crates/engram-core/src/crypto.rs`** — `EngramKey::derive(password: &[u8], salt: &[u8; 16])`, `encrypt(key, plaintext)`, `decrypt(key, data)`. Note: `derive` takes `&[u8]`, not `&str`.
- **`crates/engram-core/src/vault.rs`** — `Vault::new(path)`, `vault.read("relative/path.md") -> Result<String>`, `vault.write("path.md", "content")`, `vault.list_markdown() -> Result<Vec<String>>`.
- **`crates/engram-cli/src/main.rs`** — clap derive pattern. Package `name = "engram"` so binary commands use `-p engram`.
- **`keyring = "2"` API** — uses `delete_password()` (not `delete_credential()`). Match `crypto.rs` pattern.
- **Keychain tests** — `crypto.rs` marks real-keychain tests `#[ignore = "requires interactive keychain access; run with --include-ignored in a GUI session"]`. Follow this convention.

---

## File Structure Being Created

```
crates/engram-sync/
├── Cargo.toml
└── src/
    ├── lib.rs         ← pub mod exports + re-exports
    ├── backend.rs     ← SyncBackend trait + SyncError enum
    ├── encrypt.rs     ← encrypt_for_sync() / decrypt_from_sync()
    ├── s3.rs          ← S3Backend (object_store aws feature)
    ├── azure.rs       ← AzureBackend (object_store azure feature)
    ├── gcs.rs         ← GcsBackend (object_store gcp feature)
    ├── onedrive.rs    ← OneDriveBackend via Microsoft Graph REST API
    └── auth.rs        ← AuthStore for platform keychain credential storage
tests/
    └── integration_test.rs
```

Also modified:
- `Cargo.toml` (workspace root) — add `"crates/engram-sync"` to members
- `crates/engram-cli/Cargo.toml` — add engram-sync, rpassword, reqwest, tokio deps
- `crates/engram-cli/src/main.rs` — add Auth + Sync subcommands

---

## Task 1: Initialize the engram-sync crate

**Files:**
- Modify: `Cargo.toml` (workspace root)
- Create: `crates/engram-sync/Cargo.toml`
- Create: `crates/engram-sync/src/backend.rs`
- Create: `crates/engram-sync/src/lib.rs`
- Create: `crates/engram-sync/src/encrypt.rs` (stub)
- Create: `crates/engram-sync/src/s3.rs` (stub)
- Create: `crates/engram-sync/src/azure.rs` (stub)
- Create: `crates/engram-sync/src/gcs.rs` (stub)
- Create: `crates/engram-sync/src/onedrive.rs` (stub)
- Create: `crates/engram-sync/src/auth.rs` (stub)

**Step 1: Add engram-sync to workspace members**

Open `Cargo.toml` at the workspace root. The current content is:

```toml
[workspace]
    members = [
        "crates/engram-core",
        "crates/engram-cli",
    ]
    resolver = "2"
```

Add `"crates/engram-sync"` to the members list. Result:

```toml
[workspace]
    members = [
        "crates/engram-core",
        "crates/engram-cli",
        "crates/engram-sync",
    ]
    resolver = "2"
```

**Step 2: Create `crates/engram-sync/Cargo.toml`**

```bash
mkdir -p crates/engram-sync/src
```

```toml
[package]
name = "engram-sync"
version = "0.1.0"
edition = "2021"

[dependencies]
engram-core = { path = "../engram-core" }
object_store = { version = "0.11", features = ["aws", "azure", "gcp"] }
oauth2 = "4"
reqwest = { version = "0.12", features = ["json"] }
tokio = { version = "1", features = ["full"] }
serde = { version = "1", features = ["derive"] }
serde_json = "1"
thiserror = "2"
base64 = { version = "0.22", features = ["std"] }
open = "5"
keyring = "2"
bytes = "1"
async-trait = "0.1"
url = "2"
futures = "0.3"

[dev-dependencies]
tempfile = "3"
tokio = { version = "1", features = ["full", "test-util"] }
```

**Step 3: Create `crates/engram-sync/src/backend.rs`**

```rust
// crates/engram-sync/src/backend.rs

use async_trait::async_trait;
use bytes::Bytes;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum SyncError {
    #[error("Backend error: {0}")]
    Backend(String),
    #[error("Auth error: {0}")]
    Auth(String),
    #[error("Not found: {0}")]
    NotFound(String),
    #[error("Encryption error: {0}")]
    Encryption(String),
    #[error("Network error: {0}")]
    Network(#[from] reqwest::Error),
}

#[async_trait]
pub trait SyncBackend: Send + Sync {
    async fn push(&self, path: &str, data: Bytes) -> Result<(), SyncError>;
    async fn pull(&self, path: &str) -> Result<Bytes, SyncError>;
    async fn list(&self, prefix: &str) -> Result<Vec<String>, SyncError>;
    async fn delete(&self, path: &str) -> Result<(), SyncError>;
    fn backend_name(&self) -> &'static str;
}
```

**Step 4: Create stub files for the remaining modules**

`crates/engram-sync/src/encrypt.rs`:
```rust
// Stub — implemented in Task 2
```

`crates/engram-sync/src/s3.rs`:
```rust
// Stub — implemented in Task 3
```

`crates/engram-sync/src/azure.rs`:
```rust
// Stub — implemented in Task 4
```

`crates/engram-sync/src/gcs.rs`:
```rust
// Stub — implemented in Task 5
```

`crates/engram-sync/src/onedrive.rs`:
```rust
// Stub — implemented in Task 6
```

`crates/engram-sync/src/auth.rs`:
```rust
// Stub — implemented in Task 7
```

**Step 5: Create `crates/engram-sync/src/lib.rs`**

```rust
// crates/engram-sync/src/lib.rs

pub mod auth;
pub mod backend;
pub mod encrypt;
pub mod s3;
pub mod azure;
pub mod gcs;
pub mod onedrive;

pub use backend::{SyncBackend, SyncError};
pub use bytes::Bytes;
```

**Step 6: Verify the crate compiles**

Run:
```bash
cargo build -p engram-sync
```

Expected: compiles with zero errors (warnings about empty stub files are fine).

**Step 7: Commit**

```bash
git add Cargo.toml crates/engram-sync/ && \
git commit -m "chore(sync): initialize engram-sync crate with SyncBackend trait"
```

---

## Task 2: Sync encryption wrapper

**Files:**
- Modify: `crates/engram-sync/src/encrypt.rs`

**Step 1: Write the failing tests first**

Replace the stub in `crates/engram-sync/src/encrypt.rs` with the tests only (no implementation yet):

```rust
// crates/engram-sync/src/encrypt.rs

#[cfg(test)]
mod tests {
    use engram_core::crypto::EngramKey;
    use super::*;

    fn test_key() -> EngramKey {
        // EngramKey::derive takes &[u8] (byte slice), not &str
        EngramKey::derive(b"sync-test-password", &[0u8; 16]).unwrap()
    }

    #[test]
    fn test_encrypt_decrypt_roundtrip() {
        let key = test_key();
        let plaintext = b"# Sofia.md\n\nSofia is vegetarian.";
        let encrypted = encrypt_for_sync(&key, plaintext).unwrap();
        let decrypted = decrypt_from_sync(&key, &encrypted).unwrap();
        assert_eq!(decrypted, plaintext);
    }

    #[test]
    fn test_encrypted_has_magic_prefix() {
        let key = test_key();
        let encrypted = encrypt_for_sync(&key, b"hello").unwrap();
        assert!(encrypted.starts_with(MAGIC));
    }

    #[test]
    fn test_missing_magic_returns_error() {
        let key = test_key();
        let result = decrypt_from_sync(&key, b"raw-bytes-without-prefix");
        assert!(result.is_err());
    }

    #[test]
    fn test_tampered_ciphertext_returns_error() {
        let key = test_key();
        let mut encrypted = encrypt_for_sync(&key, b"secret").unwrap().to_vec();
        // Flip a bit in the ciphertext region (past the 10-byte magic + 24-byte nonce)
        encrypted[40] ^= 0xFF;
        assert!(decrypt_from_sync(&key, &encrypted).is_err());
    }
}
```

**Step 2: Run tests to confirm compile failure**

Run:
```bash
cargo test -p engram-sync encrypt
```

Expected: compile error — `encrypt_for_sync`, `decrypt_from_sync`, and `MAGIC` are not defined.

**Step 3: Implement `encrypt.rs`**

Replace the file contents with the full implementation:

```rust
// crates/engram-sync/src/encrypt.rs

use bytes::Bytes;
use engram_core::crypto::{decrypt, encrypt, EngramKey};
use crate::backend::SyncError;

pub const MAGIC: &[u8] = b"ENGRAM_V1:";

/// Encrypt plaintext for cloud sync.
///
/// Prepends a `ENGRAM_V1:` magic prefix so encrypted blobs are
/// identifiable without attempting decryption. The actual encryption
/// is delegated to `engram_core::crypto::encrypt` (XChaCha20-Poly1305).
pub fn encrypt_for_sync(key: &EngramKey, plaintext: &[u8]) -> Result<Bytes, SyncError> {
    let ciphertext = encrypt(key, plaintext)
        .map_err(|e| SyncError::Encryption(e.to_string()))?;
    let mut output = Vec::with_capacity(MAGIC.len() + ciphertext.len());
    output.extend_from_slice(MAGIC);
    output.extend_from_slice(&ciphertext);
    Ok(Bytes::from(output))
}

/// Decrypt a blob produced by `encrypt_for_sync`.
///
/// Returns an error if the magic prefix is missing (blob was not produced
/// by this system) or if decryption fails (wrong key or tampered data).
pub fn decrypt_from_sync(key: &EngramKey, data: &[u8]) -> Result<Vec<u8>, SyncError> {
    if !data.starts_with(MAGIC) {
        return Err(SyncError::Encryption(
            "Not an engram-encrypted blob (missing magic prefix)".into(),
        ));
    }
    let ciphertext = &data[MAGIC.len()..];
    decrypt(key, ciphertext)
        .map_err(|e| SyncError::Encryption(e.to_string()))
}

#[cfg(test)]
mod tests {
    use engram_core::crypto::EngramKey;
    use super::*;

    fn test_key() -> EngramKey {
        EngramKey::derive(b"sync-test-password", &[0u8; 16]).unwrap()
    }

    #[test]
    fn test_encrypt_decrypt_roundtrip() {
        let key = test_key();
        let plaintext = b"# Sofia.md\n\nSofia is vegetarian.";
        let encrypted = encrypt_for_sync(&key, plaintext).unwrap();
        let decrypted = decrypt_from_sync(&key, &encrypted).unwrap();
        assert_eq!(decrypted, plaintext);
    }

    #[test]
    fn test_encrypted_has_magic_prefix() {
        let key = test_key();
        let encrypted = encrypt_for_sync(&key, b"hello").unwrap();
        assert!(encrypted.starts_with(MAGIC));
    }

    #[test]
    fn test_missing_magic_returns_error() {
        let key = test_key();
        let result = decrypt_from_sync(&key, b"raw-bytes-without-prefix");
        assert!(result.is_err());
    }

    #[test]
    fn test_tampered_ciphertext_returns_error() {
        let key = test_key();
        let mut encrypted = encrypt_for_sync(&key, b"secret").unwrap().to_vec();
        // Flip a bit in the ciphertext region (past the 10-byte magic + 24-byte nonce)
        encrypted[40] ^= 0xFF;
        assert!(decrypt_from_sync(&key, &encrypted).is_err());
    }
}
```

**Step 4: Run tests to confirm they pass**

Run:
```bash
cargo test -p engram-sync encrypt
```

Expected: `4 tests pass`.

**Step 5: Commit**

```bash
git add crates/engram-sync/src/encrypt.rs && \
git commit -m "feat(sync): encrypt_for_sync / decrypt_from_sync with ENGRAM_V1 magic prefix"
```

---

## Task 3: S3 Backend

**Files:**
- Modify: `crates/engram-sync/src/s3.rs`

**Step 1: Write the failing tests first**

Replace the stub with tests only (no `S3Backend` yet):

```rust
// crates/engram-sync/src/s3.rs

#[cfg(test)]
mod tests {
    use super::*;
    use bytes::Bytes;
    use object_store::local::LocalFileSystem;
    use std::sync::Arc;
    use tempfile::TempDir;

    // LocalFileSystem implements ObjectStore — same trait logic, no real S3 needed in CI.
    fn local_backend(dir: &TempDir) -> S3Backend {
        let store = LocalFileSystem::new_with_prefix(dir.path()).unwrap();
        S3Backend::from_store(Arc::new(store), "test-bucket".to_string())
    }

    #[tokio::test]
    async fn test_push_and_pull_bytes() {
        let dir = TempDir::new().unwrap();
        let backend = local_backend(&dir);
        let data = Bytes::from(b"encrypted-payload".to_vec());
        backend.push("vault/test.md", data.clone()).await.unwrap();
        let retrieved = backend.pull("vault/test.md").await.unwrap();
        assert_eq!(retrieved, data);
    }

    #[tokio::test]
    async fn test_list_returns_pushed_paths() {
        let dir = TempDir::new().unwrap();
        let backend = local_backend(&dir);
        backend.push("vault/People/Sofia.md", Bytes::from("a")).await.unwrap();
        backend.push("vault/People/Chris.md", Bytes::from("b")).await.unwrap();
        let paths = backend.list("vault/People").await.unwrap();
        assert_eq!(paths.len(), 2);
    }

    #[tokio::test]
    async fn test_delete_removes_object() {
        let dir = TempDir::new().unwrap();
        let backend = local_backend(&dir);
        backend.push("vault/temp.md", Bytes::from("x")).await.unwrap();
        backend.delete("vault/temp.md").await.unwrap();
        assert!(backend.pull("vault/temp.md").await.is_err());
    }

    #[test]
    fn test_backend_name() {
        let dir = TempDir::new().unwrap();
        let backend = local_backend(&dir);
        assert_eq!(backend.backend_name(), "s3");
    }
}
```

**Step 2: Run tests to confirm compile failure**

Run:
```bash
cargo test -p engram-sync s3
```

Expected: compile error — `S3Backend` not defined.

**Step 3: Implement `s3.rs`**

Replace the file contents with the full implementation + the tests from Step 1:

```rust
// crates/engram-sync/src/s3.rs

use async_trait::async_trait;
use bytes::Bytes;
use futures::StreamExt;
use object_store::{aws::AmazonS3Builder, path::Path, ObjectStore};
use std::sync::Arc;
use crate::backend::{SyncBackend, SyncError};

pub struct S3Backend {
    pub(crate) store: Arc<dyn ObjectStore>,
    pub(crate) bucket: String,
}

impl S3Backend {
    /// Create an S3Backend connected to a real S3-compatible endpoint.
    pub fn new(
        endpoint: &str,
        bucket: &str,
        access_key: &str,
        secret_key: &str,
    ) -> Result<Self, SyncError> {
        let store = AmazonS3Builder::new()
            .with_endpoint(endpoint)
            .with_bucket_name(bucket)
            .with_access_key_id(access_key)
            .with_secret_access_key(secret_key)
            .with_allow_http(true)
            .build()
            .map_err(|e| SyncError::Backend(e.to_string()))?;
        Ok(Self {
            store: Arc::new(store),
            bucket: bucket.to_string(),
        })
    }

    /// Create an S3Backend from an existing ObjectStore. Used in tests and
    /// the integration test harness where LocalFileSystem stands in for S3.
    pub fn from_store(store: Arc<dyn ObjectStore>, bucket: String) -> Self {
        Self { store, bucket }
    }
}

#[async_trait]
impl SyncBackend for S3Backend {
    async fn push(&self, path: &str, data: Bytes) -> Result<(), SyncError> {
        self.store
            .put(&Path::from(path), data.into())
            .await
            .map_err(|e| SyncError::Backend(e.to_string()))?;
        Ok(())
    }

    async fn pull(&self, path: &str) -> Result<Bytes, SyncError> {
        let result = self.store
            .get(&Path::from(path))
            .await
            .map_err(|e| {
                let msg = e.to_string();
                if msg.contains("404") || msg.contains("not found") || msg.contains("No such file") {
                    SyncError::NotFound(path.to_string())
                } else {
                    SyncError::Backend(msg)
                }
            })?;
        let bytes = result
            .bytes()
            .await
            .map_err(|e| SyncError::Backend(e.to_string()))?;
        Ok(bytes)
    }

    async fn list(&self, prefix: &str) -> Result<Vec<String>, SyncError> {
        let prefix_path = Path::from(prefix);
        let results: Vec<_> = self.store
            .list(Some(&prefix_path))
            .collect()
            .await;
        results
            .into_iter()
            .map(|r| {
                r.map(|meta| meta.location.to_string())
                    .map_err(|e| SyncError::Backend(e.to_string()))
            })
            .collect()
    }

    async fn delete(&self, path: &str) -> Result<(), SyncError> {
        self.store
            .delete(&Path::from(path))
            .await
            .map_err(|e| SyncError::Backend(e.to_string()))?;
        Ok(())
    }

    fn backend_name(&self) -> &'static str {
        "s3"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bytes::Bytes;
    use object_store::local::LocalFileSystem;
    use std::sync::Arc;
    use tempfile::TempDir;

    fn local_backend(dir: &TempDir) -> S3Backend {
        let store = LocalFileSystem::new_with_prefix(dir.path()).unwrap();
        S3Backend::from_store(Arc::new(store), "test-bucket".to_string())
    }

    #[tokio::test]
    async fn test_push_and_pull_bytes() {
        let dir = TempDir::new().unwrap();
        let backend = local_backend(&dir);
        let data = Bytes::from(b"encrypted-payload".to_vec());
        backend.push("vault/test.md", data.clone()).await.unwrap();
        let retrieved = backend.pull("vault/test.md").await.unwrap();
        assert_eq!(retrieved, data);
    }

    #[tokio::test]
    async fn test_list_returns_pushed_paths() {
        let dir = TempDir::new().unwrap();
        let backend = local_backend(&dir);
        backend.push("vault/People/Sofia.md", Bytes::from("a")).await.unwrap();
        backend.push("vault/People/Chris.md", Bytes::from("b")).await.unwrap();
        let paths = backend.list("vault/People").await.unwrap();
        assert_eq!(paths.len(), 2);
    }

    #[tokio::test]
    async fn test_delete_removes_object() {
        let dir = TempDir::new().unwrap();
        let backend = local_backend(&dir);
        backend.push("vault/temp.md", Bytes::from("x")).await.unwrap();
        backend.delete("vault/temp.md").await.unwrap();
        assert!(backend.pull("vault/temp.md").await.is_err());
    }

    #[test]
    fn test_backend_name() {
        let dir = TempDir::new().unwrap();
        let backend = local_backend(&dir);
        assert_eq!(backend.backend_name(), "s3");
    }
}
```

**Step 4: Run tests to confirm they pass**

Run:
```bash
cargo test -p engram-sync s3
```

Expected: `4 tests pass`.

**Step 5: Commit**

```bash
git add crates/engram-sync/src/s3.rs && \
git commit -m "feat(sync): S3Backend via object_store with push/pull/list/delete"
```

---

## Task 4: Azure Blob Backend

**Files:**
- Modify: `crates/engram-sync/src/azure.rs`

**Step 1: Write the failing tests first**

Replace the stub with tests only (no `AzureBackend` yet):

```rust
// crates/engram-sync/src/azure.rs

#[cfg(test)]
mod tests {
    use super::*;
    use bytes::Bytes;
    use object_store::local::LocalFileSystem;
    use std::sync::Arc;
    use tempfile::TempDir;

    fn local_backend(dir: &TempDir) -> AzureBackend {
        let store = LocalFileSystem::new_with_prefix(dir.path()).unwrap();
        AzureBackend { store: Arc::new(store) }
    }

    #[tokio::test]
    async fn test_push_and_pull_bytes() {
        let dir = TempDir::new().unwrap();
        let backend = local_backend(&dir);
        let data = Bytes::from(b"azure-payload".to_vec());
        backend.push("vault/test.md", data.clone()).await.unwrap();
        let retrieved = backend.pull("vault/test.md").await.unwrap();
        assert_eq!(retrieved, data);
    }

    #[tokio::test]
    async fn test_list_returns_pushed_paths() {
        let dir = TempDir::new().unwrap();
        let backend = local_backend(&dir);
        backend.push("vault/Work/notes.md", Bytes::from("a")).await.unwrap();
        backend.push("vault/Work/tasks.md", Bytes::from("b")).await.unwrap();
        let paths = backend.list("vault/Work").await.unwrap();
        assert_eq!(paths.len(), 2);
    }

    #[tokio::test]
    async fn test_delete_removes_object() {
        let dir = TempDir::new().unwrap();
        let backend = local_backend(&dir);
        backend.push("vault/temp.md", Bytes::from("x")).await.unwrap();
        backend.delete("vault/temp.md").await.unwrap();
        assert!(backend.pull("vault/temp.md").await.is_err());
    }

    #[test]
    fn test_backend_name() {
        let dir = TempDir::new().unwrap();
        let backend = local_backend(&dir);
        assert_eq!(backend.backend_name(), "azure");
    }
}
```

**Step 2: Run tests to confirm compile failure**

Run:
```bash
cargo test -p engram-sync azure
```

Expected: compile error — `AzureBackend` not defined.

**Step 3: Implement `azure.rs`**

Replace the file with:

```rust
// crates/engram-sync/src/azure.rs

use async_trait::async_trait;
use bytes::Bytes;
use futures::StreamExt;
use object_store::{azure::MicrosoftAzureBuilder, path::Path, ObjectStore};
use std::sync::Arc;
use crate::backend::{SyncBackend, SyncError};

pub struct AzureBackend {
    pub(crate) store: Arc<dyn ObjectStore>,
}

impl AzureBackend {
    pub fn new(
        account: &str,
        access_key: &str,
        container: &str,
    ) -> Result<Self, SyncError> {
        let store = MicrosoftAzureBuilder::new()
            .with_account(account)
            .with_access_key(access_key)
            .with_container_name(container)
            .build()
            .map_err(|e| SyncError::Backend(e.to_string()))?;
        Ok(Self { store: Arc::new(store) })
    }
}

#[async_trait]
impl SyncBackend for AzureBackend {
    async fn push(&self, path: &str, data: Bytes) -> Result<(), SyncError> {
        self.store
            .put(&Path::from(path), data.into())
            .await
            .map_err(|e| SyncError::Backend(e.to_string()))?;
        Ok(())
    }

    async fn pull(&self, path: &str) -> Result<Bytes, SyncError> {
        let result = self.store
            .get(&Path::from(path))
            .await
            .map_err(|e| {
                let msg = e.to_string();
                if msg.contains("404") || msg.contains("not found") || msg.contains("No such file") {
                    SyncError::NotFound(path.to_string())
                } else {
                    SyncError::Backend(msg)
                }
            })?;
        result
            .bytes()
            .await
            .map_err(|e| SyncError::Backend(e.to_string()))
    }

    async fn list(&self, prefix: &str) -> Result<Vec<String>, SyncError> {
        let results: Vec<_> = self.store
            .list(Some(&Path::from(prefix)))
            .collect()
            .await;
        results
            .into_iter()
            .map(|r| {
                r.map(|meta| meta.location.to_string())
                    .map_err(|e| SyncError::Backend(e.to_string()))
            })
            .collect()
    }

    async fn delete(&self, path: &str) -> Result<(), SyncError> {
        self.store
            .delete(&Path::from(path))
            .await
            .map_err(|e| SyncError::Backend(e.to_string()))?;
        Ok(())
    }

    fn backend_name(&self) -> &'static str {
        "azure"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bytes::Bytes;
    use object_store::local::LocalFileSystem;
    use std::sync::Arc;
    use tempfile::TempDir;

    fn local_backend(dir: &TempDir) -> AzureBackend {
        let store = LocalFileSystem::new_with_prefix(dir.path()).unwrap();
        AzureBackend { store: Arc::new(store) }
    }

    #[tokio::test]
    async fn test_push_and_pull_bytes() {
        let dir = TempDir::new().unwrap();
        let backend = local_backend(&dir);
        let data = Bytes::from(b"azure-payload".to_vec());
        backend.push("vault/test.md", data.clone()).await.unwrap();
        let retrieved = backend.pull("vault/test.md").await.unwrap();
        assert_eq!(retrieved, data);
    }

    #[tokio::test]
    async fn test_list_returns_pushed_paths() {
        let dir = TempDir::new().unwrap();
        let backend = local_backend(&dir);
        backend.push("vault/Work/notes.md", Bytes::from("a")).await.unwrap();
        backend.push("vault/Work/tasks.md", Bytes::from("b")).await.unwrap();
        let paths = backend.list("vault/Work").await.unwrap();
        assert_eq!(paths.len(), 2);
    }

    #[tokio::test]
    async fn test_delete_removes_object() {
        let dir = TempDir::new().unwrap();
        let backend = local_backend(&dir);
        backend.push("vault/temp.md", Bytes::from("x")).await.unwrap();
        backend.delete("vault/temp.md").await.unwrap();
        assert!(backend.pull("vault/temp.md").await.is_err());
    }

    #[test]
    fn test_backend_name() {
        let dir = TempDir::new().unwrap();
        let backend = local_backend(&dir);
        assert_eq!(backend.backend_name(), "azure");
    }
}
```

**Step 4: Run tests to confirm they pass**

Run:
```bash
cargo test -p engram-sync azure
```

Expected: `4 tests pass`.

**Step 5: Commit**

```bash
git add crates/engram-sync/src/azure.rs && \
git commit -m "feat(sync): AzureBackend via object_store azure feature"
```

---

## Task 5: Google Cloud Storage Backend

**Files:**
- Modify: `crates/engram-sync/src/gcs.rs`

**Step 1: Write the failing tests first**

Replace the stub with tests only (no `GcsBackend` yet). Same pattern as Task 4 — use `LocalFileSystem` as the store:

```rust
// crates/engram-sync/src/gcs.rs

#[cfg(test)]
mod tests {
    use super::*;
    use bytes::Bytes;
    use object_store::local::LocalFileSystem;
    use std::sync::Arc;
    use tempfile::TempDir;

    fn local_backend(dir: &TempDir) -> GcsBackend {
        let store = LocalFileSystem::new_with_prefix(dir.path()).unwrap();
        GcsBackend { store: Arc::new(store) }
    }

    #[tokio::test]
    async fn test_push_and_pull_bytes() {
        let dir = TempDir::new().unwrap();
        let backend = local_backend(&dir);
        let data = Bytes::from(b"gcs-payload".to_vec());
        backend.push("vault/test.md", data.clone()).await.unwrap();
        let retrieved = backend.pull("vault/test.md").await.unwrap();
        assert_eq!(retrieved, data);
    }

    #[tokio::test]
    async fn test_list_returns_pushed_paths() {
        let dir = TempDir::new().unwrap();
        let backend = local_backend(&dir);
        backend.push("vault/Personal/diary.md", Bytes::from("a")).await.unwrap();
        backend.push("vault/Personal/goals.md", Bytes::from("b")).await.unwrap();
        let paths = backend.list("vault/Personal").await.unwrap();
        assert_eq!(paths.len(), 2);
    }

    #[tokio::test]
    async fn test_delete_removes_object() {
        let dir = TempDir::new().unwrap();
        let backend = local_backend(&dir);
        backend.push("vault/temp.md", Bytes::from("x")).await.unwrap();
        backend.delete("vault/temp.md").await.unwrap();
        assert!(backend.pull("vault/temp.md").await.is_err());
    }

    #[test]
    fn test_backend_name() {
        let dir = TempDir::new().unwrap();
        let backend = local_backend(&dir);
        assert_eq!(backend.backend_name(), "gcs");
    }
}
```

**Step 2: Run tests to confirm compile failure**

Run:
```bash
cargo test -p engram-sync gcs
```

Expected: compile error — `GcsBackend` not defined.

**Step 3: Implement `gcs.rs`**

Replace the file with:

```rust
// crates/engram-sync/src/gcs.rs

use async_trait::async_trait;
use bytes::Bytes;
use futures::StreamExt;
use object_store::{gcp::GoogleCloudStorageBuilder, path::Path, ObjectStore};
use std::sync::Arc;
use crate::backend::{SyncBackend, SyncError};

pub struct GcsBackend {
    pub(crate) store: Arc<dyn ObjectStore>,
}

impl GcsBackend {
    pub fn new(
        bucket: &str,
        service_account_key_path: &str,
    ) -> Result<Self, SyncError> {
        let store = GoogleCloudStorageBuilder::new()
            .with_bucket_name(bucket)
            .with_service_account_path(service_account_key_path)
            .build()
            .map_err(|e| SyncError::Backend(e.to_string()))?;
        Ok(Self { store: Arc::new(store) })
    }
}

#[async_trait]
impl SyncBackend for GcsBackend {
    async fn push(&self, path: &str, data: Bytes) -> Result<(), SyncError> {
        self.store
            .put(&Path::from(path), data.into())
            .await
            .map_err(|e| SyncError::Backend(e.to_string()))?;
        Ok(())
    }

    async fn pull(&self, path: &str) -> Result<Bytes, SyncError> {
        let result = self.store
            .get(&Path::from(path))
            .await
            .map_err(|e| {
                let msg = e.to_string();
                if msg.contains("404") || msg.contains("not found") || msg.contains("No such file") {
                    SyncError::NotFound(path.to_string())
                } else {
                    SyncError::Backend(msg)
                }
            })?;
        result
            .bytes()
            .await
            .map_err(|e| SyncError::Backend(e.to_string()))
    }

    async fn list(&self, prefix: &str) -> Result<Vec<String>, SyncError> {
        let results: Vec<_> = self.store
            .list(Some(&Path::from(prefix)))
            .collect()
            .await;
        results
            .into_iter()
            .map(|r| {
                r.map(|meta| meta.location.to_string())
                    .map_err(|e| SyncError::Backend(e.to_string()))
            })
            .collect()
    }

    async fn delete(&self, path: &str) -> Result<(), SyncError> {
        self.store
            .delete(&Path::from(path))
            .await
            .map_err(|e| SyncError::Backend(e.to_string()))?;
        Ok(())
    }

    fn backend_name(&self) -> &'static str {
        "gcs"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bytes::Bytes;
    use object_store::local::LocalFileSystem;
    use std::sync::Arc;
    use tempfile::TempDir;

    fn local_backend(dir: &TempDir) -> GcsBackend {
        let store = LocalFileSystem::new_with_prefix(dir.path()).unwrap();
        GcsBackend { store: Arc::new(store) }
    }

    #[tokio::test]
    async fn test_push_and_pull_bytes() {
        let dir = TempDir::new().unwrap();
        let backend = local_backend(&dir);
        let data = Bytes::from(b"gcs-payload".to_vec());
        backend.push("vault/test.md", data.clone()).await.unwrap();
        let retrieved = backend.pull("vault/test.md").await.unwrap();
        assert_eq!(retrieved, data);
    }

    #[tokio::test]
    async fn test_list_returns_pushed_paths() {
        let dir = TempDir::new().unwrap();
        let backend = local_backend(&dir);
        backend.push("vault/Personal/diary.md", Bytes::from("a")).await.unwrap();
        backend.push("vault/Personal/goals.md", Bytes::from("b")).await.unwrap();
        let paths = backend.list("vault/Personal").await.unwrap();
        assert_eq!(paths.len(), 2);
    }

    #[tokio::test]
    async fn test_delete_removes_object() {
        let dir = TempDir::new().unwrap();
        let backend = local_backend(&dir);
        backend.push("vault/temp.md", Bytes::from("x")).await.unwrap();
        backend.delete("vault/temp.md").await.unwrap();
        assert!(backend.pull("vault/temp.md").await.is_err());
    }

    #[test]
    fn test_backend_name() {
        let dir = TempDir::new().unwrap();
        let backend = local_backend(&dir);
        assert_eq!(backend.backend_name(), "gcs");
    }
}
```

**Step 4: Run tests to confirm they pass**

Run:
```bash
cargo test -p engram-sync gcs
```

Expected: `4 tests pass`.

**Step 5: Commit**

```bash
git add crates/engram-sync/src/gcs.rs && \
git commit -m "feat(sync): GcsBackend via object_store gcp feature"
```

---

## Task 6: OneDrive Backend

**Files:**
- Modify: `crates/engram-sync/src/onedrive.rs`

**Step 1: Write the failing tests first**

Replace the stub with tests only (no `OneDriveBackend` yet):

```rust
// crates/engram-sync/src/onedrive.rs

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_item_url_construction() {
        let backend = OneDriveBackend::new("token", "/Apps/Engram/vault");
        let url = backend.item_url("People/Sofia.md");
        assert_eq!(
            url,
            "https://graph.microsoft.com/v1.0/me/drive/root:/Apps/Engram/vault/People/Sofia.md:/content"
        );
    }

    #[test]
    fn test_item_url_no_double_slash_when_folder_has_trailing_slash() {
        let backend = OneDriveBackend::new("token", "/Apps/Engram/vault/");
        let url = backend.item_url("Tasks.md");
        assert_eq!(
            url,
            "https://graph.microsoft.com/v1.0/me/drive/root:/Apps/Engram/vault/Tasks.md:/content"
        );
    }

    #[test]
    fn test_backend_name() {
        let backend = OneDriveBackend::new("token", "/Apps/Engram/vault");
        assert_eq!(backend.backend_name(), "onedrive");
    }

    #[tokio::test]
    #[ignore = "requires real OneDrive access token — set ONEDRIVE_TOKEN env var"]
    async fn test_push_pull_real() {
        use bytes::Bytes;
        let token = std::env::var("ONEDRIVE_TOKEN").expect("ONEDRIVE_TOKEN not set");
        let backend = OneDriveBackend::new(&token, "/Apps/Engram/test");
        backend.push("test.md", Bytes::from("hello")).await.unwrap();
        let pulled = backend.pull("test.md").await.unwrap();
        assert_eq!(pulled.as_ref(), b"hello");
        backend.delete("test.md").await.unwrap();
    }
}
```

**Step 2: Run tests to confirm compile failure**

Run:
```bash
cargo test -p engram-sync onedrive
```

Expected: compile error — `OneDriveBackend` not defined.

**Step 3: Implement `onedrive.rs`**

Replace the file with:

```rust
// crates/engram-sync/src/onedrive.rs

use async_trait::async_trait;
use bytes::Bytes;
use reqwest::{Client, StatusCode};
use crate::backend::{SyncBackend, SyncError};

pub struct OneDriveBackend {
    client: Client,
    access_token: String,
    folder: String, // e.g. "/Apps/Engram/vault"
}

impl OneDriveBackend {
    pub fn new(access_token: &str, folder: &str) -> Self {
        Self {
            client: Client::new(),
            access_token: access_token.to_string(),
            folder: folder.to_string(),
        }
    }

    /// Build the Graph API URL for file content operations.
    pub(crate) fn item_url(&self, path: &str) -> String {
        let full_path = format!(
            "{}/{}",
            self.folder.trim_end_matches('/'),
            path
        );
        format!(
            "https://graph.microsoft.com/v1.0/me/drive/root:{}:/content",
            full_path
        )
    }

    fn auth_header(&self) -> String {
        format!("Bearer {}", self.access_token)
    }
}

#[async_trait]
impl SyncBackend for OneDriveBackend {
    async fn push(&self, path: &str, data: Bytes) -> Result<(), SyncError> {
        let url = self.item_url(path);
        let response = self.client
            .put(&url)
            .header("Authorization", self.auth_header())
            .header("Content-Type", "application/octet-stream")
            .body(data)
            .send()
            .await?;
        if !response.status().is_success() {
            return Err(SyncError::Backend(format!(
                "OneDrive push failed: {}",
                response.status()
            )));
        }
        Ok(())
    }

    async fn pull(&self, path: &str) -> Result<Bytes, SyncError> {
        let url = self.item_url(path);
        let response = self.client
            .get(&url)
            .header("Authorization", self.auth_header())
            .send()
            .await?;
        if response.status() == StatusCode::NOT_FOUND {
            return Err(SyncError::NotFound(path.to_string()));
        }
        if !response.status().is_success() {
            return Err(SyncError::Backend(format!(
                "OneDrive pull failed: {}",
                response.status()
            )));
        }
        Ok(response.bytes().await?)
    }

    async fn list(&self, prefix: &str) -> Result<Vec<String>, SyncError> {
        let folder_path = format!(
            "{}/{}",
            self.folder.trim_end_matches('/'),
            prefix
        );
        let url = format!(
            "https://graph.microsoft.com/v1.0/me/drive/root:{}:/children",
            folder_path
        );
        let response = self.client
            .get(&url)
            .header("Authorization", self.auth_header())
            .send()
            .await?;
        if !response.status().is_success() {
            return Err(SyncError::Backend(format!(
                "OneDrive list failed: {}",
                response.status()
            )));
        }
        let json: serde_json::Value = response
            .json()
            .await
            .map_err(|e| SyncError::Backend(e.to_string()))?;
        let names = json["value"]
            .as_array()
            .unwrap_or(&vec![])
            .iter()
            .filter_map(|item| {
                item["name"].as_str().map(|s| format!("{}/{}", prefix, s))
            })
            .collect();
        Ok(names)
    }

    async fn delete(&self, path: &str) -> Result<(), SyncError> {
        // Use the items endpoint for delete (the content endpoint doesn't support DELETE)
        let folder_path = format!(
            "{}/{}",
            self.folder.trim_end_matches('/'),
            path
        );
        let url = format!(
            "https://graph.microsoft.com/v1.0/me/drive/root:{}",
            folder_path
        );
        let response = self.client
            .delete(&url)
            .header("Authorization", self.auth_header())
            .send()
            .await?;
        if !response.status().is_success() && response.status() != StatusCode::NOT_FOUND {
            return Err(SyncError::Backend(format!(
                "OneDrive delete failed: {}",
                response.status()
            )));
        }
        Ok(())
    }

    fn backend_name(&self) -> &'static str {
        "onedrive"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_item_url_construction() {
        let backend = OneDriveBackend::new("token", "/Apps/Engram/vault");
        let url = backend.item_url("People/Sofia.md");
        assert_eq!(
            url,
            "https://graph.microsoft.com/v1.0/me/drive/root:/Apps/Engram/vault/People/Sofia.md:/content"
        );
    }

    #[test]
    fn test_item_url_no_double_slash_when_folder_has_trailing_slash() {
        let backend = OneDriveBackend::new("token", "/Apps/Engram/vault/");
        let url = backend.item_url("Tasks.md");
        assert_eq!(
            url,
            "https://graph.microsoft.com/v1.0/me/drive/root:/Apps/Engram/vault/Tasks.md:/content"
        );
    }

    #[test]
    fn test_backend_name() {
        let backend = OneDriveBackend::new("token", "/Apps/Engram/vault");
        assert_eq!(backend.backend_name(), "onedrive");
    }

    #[tokio::test]
    #[ignore = "requires real OneDrive access token — set ONEDRIVE_TOKEN env var"]
    async fn test_push_pull_real() {
        use bytes::Bytes;
        let token = std::env::var("ONEDRIVE_TOKEN").expect("ONEDRIVE_TOKEN not set");
        let backend = OneDriveBackend::new(&token, "/Apps/Engram/test");
        backend.push("test.md", Bytes::from("hello")).await.unwrap();
        let pulled = backend.pull("test.md").await.unwrap();
        assert_eq!(pulled.as_ref(), b"hello");
        backend.delete("test.md").await.unwrap();
    }
}
```

**Step 4: Run tests to confirm they pass (non-ignored only)**

Run:
```bash
cargo test -p engram-sync onedrive
```

Expected: `3 tests pass` (2 unit tests + 0 ignored visible, 1 `#[ignore]` shown as ignored).

**Step 5: Commit**

```bash
git add crates/engram-sync/src/onedrive.rs && \
git commit -m "feat(sync): OneDriveBackend via Microsoft Graph REST API"
```

---

## Task 7: Auth Credential Storage

**Files:**
- Modify: `crates/engram-sync/src/auth.rs`

**Step 1: Write the failing tests first**

Replace the stub with tests only. Follow the project convention from `crypto.rs`: mark real-keychain tests as `#[ignore]`.

```rust
// crates/engram-sync/src/auth.rs

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
```

**Step 2: Run tests to confirm compile failure**

Run:
```bash
cargo test -p engram-sync auth
```

Expected: compile error — `AuthStore` not defined.

**Step 3: Implement `auth.rs`**

Replace the file with:

```rust
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
        assert!(!AuthStore::is_configured("s3", &["key-missing-xyz", "another-missing-xyz"]));
    }
}
```

**Step 4: Add `pub mod auth;` to `lib.rs` if not already present**

Open `crates/engram-sync/src/lib.rs`. Confirm it already has `pub mod auth;` (it was added in Task 1). If missing, add it.

**Step 5: Run tests to confirm they pass**

Run:
```bash
cargo test -p engram-sync auth
```

Expected: `2 tests pass` (the non-`#[ignore]` ones), `2 ignored`.

**Step 6: Commit**

```bash
git add crates/engram-sync/src/auth.rs && \
git commit -m "feat(sync): AuthStore for backend credential management in platform keychain"
```

---

## Task 8: CLI — `engram auth add s3`

**Files:**
- Modify: `crates/engram-cli/Cargo.toml`
- Modify: `crates/engram-cli/src/main.rs`

**Step 1: Update `crates/engram-cli/Cargo.toml`**

Add these three new dependencies under `[dependencies]`:

```toml
engram-sync = { path = "../engram-sync" }
rpassword = "7"
tokio = { version = "1", features = ["full"] }
```

The `[dependencies]` block should now read:

```toml
[dependencies]
engram-core = { path = "../engram-core" }
engram-sync = { path = "../engram-sync" }
clap = { version = "4", features = ["derive"] }
directories = "5"
thiserror = "2"
anyhow = "1"
rpassword = "7"
tokio = { version = "1", features = ["full"] }
```

**Step 2: Add auth subcommand enums to `main.rs`**

Open `crates/engram-cli/src/main.rs`. Add the following enums **before** `fn main()`:

```rust
#[derive(Subcommand)]
enum AuthCommands {
    /// Configure a sync backend (stores credentials in keychain)
    Add {
        #[command(subcommand)]
        backend: BackendCommands,
    },
    /// List configured sync backends
    List,
    /// Remove a backend's credentials from the keychain
    Remove { backend: String },
}

#[derive(Subcommand)]
enum BackendCommands {
    /// S3-compatible storage (AWS S3, Cloudflare R2, MinIO, Backblaze B2)
    S3 {
        #[arg(long)]
        endpoint: String,
        #[arg(long)]
        bucket: String,
        /// If omitted, prompts interactively
        #[arg(long)]
        access_key: Option<String>,
        /// If omitted, prompts securely (no echo)
        #[arg(long)]
        secret_key: Option<String>,
    },
    /// Microsoft OneDrive (OAuth2 browser flow)
    Onedrive {
        #[arg(long, default_value = "/Apps/Engram/vault")]
        folder: String,
    },
    /// Azure Blob Storage
    Azure {
        #[arg(long)]
        account: String,
        #[arg(long)]
        container: String,
    },
    /// Google Cloud Storage
    Gdrive {
        #[arg(long)]
        bucket: String,
        #[arg(long)]
        key_file: String,
    },
}
```

**Step 3: Add `Auth` and `Sync` variants to the `Commands` enum**

The existing `Commands` enum only has `Status`. Change it to:

```rust
#[derive(Subcommand)]
enum Commands {
    /// Print vault state, memory store stats, and keyring status
    Status,
    /// Manage sync backend authentication
    Auth {
        #[command(subcommand)]
        command: AuthCommands,
    },
    /// Sync vault with configured backend
    Sync {
        /// Force a specific backend (s3, onedrive, azure, gcs)
        #[arg(long)]
        backend: Option<String>,
    },
}
```

**Step 4: Implement `run_auth_add_s3`**

Add this function to `main.rs`:

```rust
fn run_auth_add_s3(
    endpoint: &str,
    bucket: &str,
    access_key: Option<&str>,
    secret_key: Option<&str>,
) {
    use engram_sync::auth::AuthStore;
    use std::io::{self, Write};

    let ak = access_key
        .map(|s| s.to_string())
        .unwrap_or_else(|| {
            print!("Access key ID: ");
            io::stdout().flush().unwrap();
            let mut input = String::new();
            io::stdin().read_line(&mut input).unwrap();
            input.trim().to_string()
        });

    let sk = secret_key
        .map(|s| s.to_string())
        .unwrap_or_else(|| {
            rpassword::prompt_password("Secret access key: ").unwrap_or_default()
        });

    AuthStore::store("s3", "access_key", &ak).unwrap();
    AuthStore::store("s3", "secret_key", &sk).unwrap();
    AuthStore::store("s3", "endpoint", endpoint).unwrap();
    AuthStore::store("s3", "bucket", bucket).unwrap();

    println!("✓ S3 backend configured");
    println!("  Endpoint: {}", endpoint);
    println!("  Bucket:   {}", bucket);
}
```

**Step 5: Wire up the new commands in `main()`**

Replace the existing `match cli.command` block with:

```rust
fn main() {
    let cli = Cli::parse();
    match cli.command {
        Commands::Status => run_status(),
        Commands::Auth { command } => match command {
            AuthCommands::Add { backend } => match backend {
                BackendCommands::S3 { endpoint, bucket, access_key, secret_key } => {
                    run_auth_add_s3(
                        &endpoint,
                        &bucket,
                        access_key.as_deref(),
                        secret_key.as_deref(),
                    );
                }
                BackendCommands::Onedrive { folder } => {
                    run_auth_add_onedrive(&folder);
                }
                BackendCommands::Azure { account, container } => {
                    run_auth_add_azure(&account, &container);
                }
                BackendCommands::Gdrive { bucket, key_file } => {
                    run_auth_add_gdrive(&bucket, &key_file);
                }
            },
            AuthCommands::List => run_auth_list(),
            AuthCommands::Remove { backend } => run_auth_remove(&backend),
        },
        Commands::Sync { backend } => run_sync(backend.as_deref()),
    }
}
```

Add stub functions so the file compiles (these are implemented in Tasks 9–11):

```rust
fn run_auth_add_onedrive(_folder: &str) { todo!("implemented in Task 9") }
fn run_auth_add_azure(_account: &str, _container: &str) { todo!("implemented in Task 9") }
fn run_auth_add_gdrive(_bucket: &str, _key_file: &str) { todo!("implemented in Task 9") }
fn run_auth_list() { todo!("implemented in Task 10") }
fn run_auth_remove(_backend: &str) { todo!("implemented in Task 10") }
fn run_sync(_backend: Option<&str>) { todo!("implemented in Task 11") }
```

Also add this `use` at the top of the file:

```rust
use engram_sync; // pulls in the crate so auth module is accessible
```

**Step 6: Verify the binary compiles**

Run:
```bash
cargo build -p engram
```

Expected: compiles successfully (warnings about unused imports are fine).

**Step 7: Smoke-test the command**

Run:
```bash
cargo run -p engram -- auth add s3 \
  --endpoint https://r2.example.com \
  --bucket my-vault \
  --access-key test-ak \
  --secret-key test-sk
```

Expected output:
```
✓ S3 backend configured
  Endpoint: https://r2.example.com
  Bucket:   my-vault
```

**Step 8: Commit**

```bash
git add crates/engram-cli/ && \
git commit -m "feat(cli): engram auth add s3 with interactive credential prompt"
```

---

## Task 9: CLI — `engram auth add onedrive/azure/gdrive`

**Files:**
- Modify: `crates/engram-cli/Cargo.toml`
- Modify: `crates/engram-cli/src/main.rs`

**Step 1: Add `reqwest` blocking feature to engram-cli Cargo.toml**

The OneDrive auth flow uses a blocking HTTP call (synchronous token exchange). Update the `reqwest` entry — it doesn't exist yet, so add it:

```toml
reqwest = { version = "0.12", features = ["json", "blocking"] }
serde_json = "1"
```

Add both lines under `[dependencies]` in `crates/engram-cli/Cargo.toml`.

**Step 2: Replace `run_auth_add_onedrive` stub with the real implementation**

Find the line `fn run_auth_add_onedrive(_folder: &str) { todo!("implemented in Task 9") }` and replace it:

```rust
fn run_auth_add_onedrive(folder: &str) {
    use engram_sync::auth::AuthStore;
    use std::io::{self, Write};

    // Microsoft Identity platform — Azure CLI public client ID (public, no secret required)
    let client_id = "04b07795-8ddb-461a-bbee-02f9e1bf7b46";
    let auth_url = format!(
        "https://login.microsoftonline.com/common/oauth2/v2.0/authorize?\
         client_id={}&response_type=code\
         &redirect_uri=https://login.microsoftonline.com/common/oauth2/nativeclient\
         &scope=Files.ReadWrite%20offline_access&response_mode=query",
        client_id
    );

    println!("Opening browser for Microsoft authentication...");
    println!("If browser doesn't open, visit:\n{}", auth_url);
    open::that(&auth_url).ok();

    print!("\nPaste the authorization code from the redirect URL: ");
    io::stdout().flush().unwrap();
    let mut code = String::new();
    io::stdin().read_line(&mut code).unwrap();
    let code = code.trim().to_string();

    // Exchange authorization code for tokens
    let client = reqwest::blocking::Client::new();
    let response = client
        .post("https://login.microsoftonline.com/common/oauth2/v2.0/token")
        .form(&[
            ("client_id", client_id),
            ("grant_type", "authorization_code"),
            ("code", code.as_str()),
            ("redirect_uri", "https://login.microsoftonline.com/common/oauth2/nativeclient"),
            ("scope", "Files.ReadWrite offline_access"),
        ])
        .send()
        .expect("Token exchange request failed");

    let json: serde_json::Value = response.json().expect("Invalid token response");
    let access_token = json["access_token"]
        .as_str()
        .expect("No access_token in response");
    let refresh_token = json["refresh_token"].as_str().unwrap_or("");

    AuthStore::store("onedrive", "access_token", access_token).unwrap();
    AuthStore::store("onedrive", "refresh_token", refresh_token).unwrap();
    AuthStore::store("onedrive", "folder", folder).unwrap();

    println!("✓ OneDrive backend configured");
    println!("  Folder: {}", folder);
}
```

**Step 3: Replace `run_auth_add_azure` stub**

```rust
fn run_auth_add_azure(account: &str, container: &str) {
    use engram_sync::auth::AuthStore;
    use std::io::{self, Write};

    print!("Azure Storage access key: ");
    io::stdout().flush().unwrap();
    let ak = rpassword::prompt_password("Access key: ").unwrap_or_default();

    AuthStore::store("azure", "account", account).unwrap();
    AuthStore::store("azure", "container", container).unwrap();
    AuthStore::store("azure", "access_key", &ak).unwrap();

    println!("✓ Azure backend configured");
    println!("  Account:   {}", account);
    println!("  Container: {}", container);
}
```

**Step 4: Replace `run_auth_add_gdrive` stub**

```rust
fn run_auth_add_gdrive(bucket: &str, key_file: &str) {
    use engram_sync::auth::AuthStore;

    AuthStore::store("gcs", "bucket", bucket).unwrap();
    AuthStore::store("gcs", "key_file", key_file).unwrap();

    println!("✓ GCS backend configured");
    println!("  Bucket:   {}", bucket);
    println!("  Key file: {}", key_file);
}
```

**Step 5: Verify the binary compiles**

Run:
```bash
cargo build -p engram
```

Expected: compiles successfully.

**Step 6: Smoke-test OneDrive flow starts correctly**

Run:
```bash
cargo run -p engram -- auth add onedrive
```

Expected: prints "Opening browser for Microsoft authentication...", opens browser (or shows URL), then waits for input. Press `Ctrl+C` to cancel.

**Step 7: Commit**

```bash
git add crates/engram-cli/Cargo.toml crates/engram-cli/src/main.rs && \
git commit -m "feat(cli): engram auth add onedrive/azure/gdrive with OAuth2 browser flow"
```

---

## Task 10: CLI — `engram auth list` and `engram auth remove`

**Files:**
- Modify: `crates/engram-cli/src/main.rs`

**Step 1: Replace the `run_auth_list` stub**

Find `fn run_auth_list() { todo!("implemented in Task 10") }` and replace:

```rust
fn run_auth_list() {
    use engram_sync::auth::AuthStore;

    // (backend_name, required_keys_for_is_configured, display_keys_non_sensitive)
    let backends: &[(&str, &[&str], &[&str])] = &[
        ("s3",       &["access_key", "secret_key", "endpoint", "bucket"], &["endpoint", "bucket"]),
        ("onedrive", &["access_token", "folder"],                         &["folder"]),
        ("azure",    &["account", "container"],                           &["account", "container"]),
        ("gcs",      &["bucket", "key_file"],                             &["bucket", "key_file"]),
    ];

    println!("{}", "─".repeat(41));
    println!("Configured sync backends:");
    println!();

    let mut any_configured = false;
    for (backend, required, display_keys) in backends {
        if AuthStore::is_configured(backend, required) {
            let details = display_keys
                .iter()
                .filter_map(|k| {
                    AuthStore::retrieve(backend, k)
                        .ok()
                        .map(|v| format!("{}={}", k, v))
                })
                .collect::<Vec<_>>()
                .join(", ");
            println!("  ✓ {} ({})", backend, details);
            any_configured = true;
        }
    }

    if !any_configured {
        println!("  No backends configured.");
        println!();
        println!("  Run: engram auth add s3|onedrive|azure|gdrive");
    }
    println!();
}
```

**Step 2: Replace the `run_auth_remove` stub**

Find `fn run_auth_remove(_backend: &str) { todo!("implemented in Task 10") }` and replace:

```rust
fn run_auth_remove(backend: &str) {
    use engram_sync::auth::AuthStore;
    use std::collections::HashMap;

    let keys_by_backend: HashMap<&str, &[&str]> = [
        ("s3",       ["access_key", "secret_key", "endpoint", "bucket"].as_slice()),
        ("onedrive", ["access_token", "refresh_token", "folder"].as_slice()),
        ("azure",    ["account", "access_key", "container"].as_slice()),
        ("gcs",      ["bucket", "key_file"].as_slice()),
    ]
    .into_iter()
    .collect();

    match keys_by_backend.get(backend) {
        None => {
            eprintln!("Unknown backend: {}. Valid options: s3, onedrive, azure, gcs", backend);
            std::process::exit(1);
        }
        Some(keys) => {
            let removed = keys
                .iter()
                .filter(|k| AuthStore::delete(backend, k).is_ok())
                .count();
            if removed > 0 {
                println!("✓ Removed {} backend credentials", backend);
            } else {
                println!("No credentials found for {}", backend);
            }
        }
    }
}
```

**Step 3: Verify the binary compiles**

Run:
```bash
cargo build -p engram
```

Expected: compiles successfully.

**Step 4: Manual end-to-end test of list and remove**

Run the sequence in order:

```bash
# Should show "No backends configured"
cargo run -p engram -- auth list

# Configure an S3 backend
cargo run -p engram -- auth add s3 \
  --endpoint https://test.example.com \
  --bucket test-vault \
  --access-key ak123 \
  --secret-key sk456

# Should now show S3 is configured
cargo run -p engram -- auth list

# Remove it
cargo run -p engram -- auth remove s3

# Should show "No backends configured" again
cargo run -p engram -- auth list
```

Expected: each command produces the correct output as described above.

**Step 5: Commit**

```bash
git add crates/engram-cli/src/main.rs && \
git commit -m "feat(cli): engram auth list and engram auth remove"
```

---

## Task 11: CLI — `engram sync`

**Files:**
- Modify: `crates/engram-cli/src/main.rs`

**Step 1: Replace the `run_sync` stub**

Find `fn run_sync(_backend: Option<&str>) { todo!("implemented in Task 11") }` and replace with:

```rust
fn run_sync(backend_name: Option<&str>) {
    use engram_core::{crypto::KeyStore, vault::Vault};
    use engram_sync::{
        auth::AuthStore,
        backend::SyncBackend,
        encrypt::encrypt_for_sync,
        onedrive::OneDriveBackend,
        s3::S3Backend,
    };

    let vault_path = default_vault_path();
    let vault = Vault::new(&vault_path);
    let key_store = KeyStore::new("engram");

    let key = match key_store.retrieve() {
        Ok(k) => k,
        Err(_) => {
            eprintln!("No vault key found. Run: engram init");
            std::process::exit(1);
        }
    };

    // Determine which backend to use: explicit arg → first configured → error
    let effective_backend = backend_name.unwrap_or_else(|| {
        if AuthStore::is_configured("s3", &["access_key", "secret_key", "endpoint", "bucket"]) {
            "s3"
        } else if AuthStore::is_configured("onedrive", &["access_token", "folder"]) {
            "onedrive"
        } else if AuthStore::is_configured("azure", &["account", "container", "access_key"]) {
            "azure"
        } else if AuthStore::is_configured("gcs", &["bucket", "key_file"]) {
            "gcs"
        } else {
            eprintln!("No sync backend configured. Run: engram auth add s3|onedrive|azure|gdrive");
            std::process::exit(1);
        }
    });

    let backend: Box<dyn SyncBackend> = match effective_backend {
        "s3" => {
            let endpoint = AuthStore::retrieve("s3", "endpoint").unwrap();
            let bucket   = AuthStore::retrieve("s3", "bucket").unwrap();
            let ak       = AuthStore::retrieve("s3", "access_key").unwrap();
            let sk       = AuthStore::retrieve("s3", "secret_key").unwrap();
            Box::new(S3Backend::new(&endpoint, &bucket, &ak, &sk).unwrap())
        }
        "onedrive" => {
            let token  = AuthStore::retrieve("onedrive", "access_token").unwrap();
            let folder = AuthStore::retrieve("onedrive", "folder").unwrap();
            Box::new(OneDriveBackend::new(&token, &folder))
        }
        other => {
            eprintln!("Backend '{}' is not yet supported in engram sync. Use: s3, onedrive", other);
            std::process::exit(1);
        }
    };

    let files = match vault.list_markdown() {
        Ok(f) => f,
        Err(e) => {
            eprintln!("Failed to list vault files: {}", e);
            std::process::exit(1);
        }
    };

    println!("Syncing {} files via {} ...", files.len(), effective_backend);

    let runtime = tokio::runtime::Runtime::new().unwrap();
    let mut success = 0usize;
    let mut errors = 0usize;

    for relative_path in &files {
        let content = match vault.read(relative_path) {
            Ok(c) => c,
            Err(e) => {
                eprintln!("  ✗ {}: {}", relative_path, e);
                errors += 1;
                continue;
            }
        };
        let encrypted = match encrypt_for_sync(&key, content.as_bytes()) {
            Ok(e) => e,
            Err(e) => {
                eprintln!("  ✗ {}: encryption failed — {}", relative_path, e);
                errors += 1;
                continue;
            }
        };
        match runtime.block_on(backend.push(relative_path, encrypted)) {
            Ok(_) => {
                success += 1;
            }
            Err(e) => {
                eprintln!("  ✗ {}: {}", relative_path, e);
                errors += 1;
            }
        }
    }

    println!("{}", "─".repeat(41));
    println!("Pushed:  {} files", success);
    if errors > 0 {
        eprintln!("Errors:  {} files", errors);
        std::process::exit(1);
    }
}
```

**Step 2: Verify the binary compiles**

Run:
```bash
cargo build -p engram
```

Expected: compiles successfully.

**Step 3: Smoke-test error path (no backend configured)**

First remove any S3 credentials if you added them in earlier tasks:
```bash
cargo run -p engram -- auth remove s3
```

Then:
```bash
cargo run -p engram -- sync
```

Expected:
```
No sync backend configured. Run: engram auth add s3|onedrive|azure|gdrive
```

**Step 4: Commit**

```bash
git add crates/engram-cli/src/main.rs && \
git commit -m "feat(cli): engram sync — encrypt all vault files and push to configured backend"
```

---

## Task 12: Integration Tests — Full Sync Round-Trip

**Files:**
- Modify: `crates/engram-sync/src/s3.rs` — confirm `pub(crate)` fields (already done in Task 3)
- Create: `crates/engram-sync/tests/integration_test.rs`

**Step 1: Create the integration test file**

```bash
mkdir -p crates/engram-sync/tests
```

Create `crates/engram-sync/tests/integration_test.rs`:

```rust
// Integration tests for engram-sync.
//
// These tests use LocalFileSystem as a stand-in for real cloud backends.
// The ObjectStore trait is identical; only the transport changes in production.
// No cloud credentials are required to run these tests.

use bytes::Bytes;
use engram_core::crypto::EngramKey;
use engram_core::vault::Vault;
use engram_sync::{
    encrypt::{decrypt_from_sync, encrypt_for_sync},
    s3::S3Backend,
};
use object_store::local::LocalFileSystem;
use std::sync::Arc;
use tempfile::TempDir;

fn test_key_a() -> EngramKey {
    // EngramKey::derive takes &[u8], not &str
    EngramKey::derive(b"integration-test-sync-key-a", &[1u8; 16]).unwrap()
}

fn test_key_b() -> EngramKey {
    EngramKey::derive(b"integration-test-sync-key-b", &[2u8; 16]).unwrap()
}

fn make_backend(dir: &TempDir) -> S3Backend {
    let store = LocalFileSystem::new_with_prefix(dir.path()).unwrap();
    S3Backend::from_store(Arc::new(store), "integration-test".to_string())
}

#[tokio::test]
async fn test_encrypt_push_pull_decrypt_roundtrip() {
    let store_dir = TempDir::new().unwrap();
    let vault_dir = TempDir::new().unwrap();

    // Write a markdown file to the vault
    let vault = Vault::new(vault_dir.path());
    vault
        .write(
            "People/Sofia.md",
            "Sofia is vegetarian.\nSofia lives in Seattle.",
        )
        .unwrap();

    let key = test_key_a();
    let backend = make_backend(&store_dir);

    // Encrypt and push
    let content = vault.read("People/Sofia.md").unwrap();
    let encrypted = encrypt_for_sync(&key, content.as_bytes()).unwrap();
    backend.push("People/Sofia.md", encrypted).await.unwrap();

    // Pull and decrypt
    let pulled = backend.pull("People/Sofia.md").await.unwrap();
    let decrypted = decrypt_from_sync(&key, &pulled).unwrap();
    let decrypted_str = std::str::from_utf8(&decrypted).unwrap();

    assert_eq!(
        decrypted_str,
        "Sofia is vegetarian.\nSofia lives in Seattle."
    );
}

#[tokio::test]
async fn test_wrong_key_cannot_decrypt() {
    let store_dir = TempDir::new().unwrap();
    let backend = make_backend(&store_dir);

    let key_a = test_key_a();
    let key_b = test_key_b();

    // Encrypt with key_a, attempt decrypt with key_b
    let encrypted = encrypt_for_sync(&key_a, b"secret content").unwrap();
    backend.push("secret.md", encrypted).await.unwrap();

    let pulled = backend.pull("secret.md").await.unwrap();
    assert!(
        decrypt_from_sync(&key_b, &pulled).is_err(),
        "Decrypting with the wrong key must fail"
    );
}

#[tokio::test]
async fn test_list_after_multiple_pushes() {
    let dir = TempDir::new().unwrap();
    let backend = make_backend(&dir);
    let key = test_key_a();

    backend
        .push("vault/Work/notes.md", encrypt_for_sync(&key, b"notes").unwrap())
        .await
        .unwrap();
    backend
        .push("vault/People/Sofia.md", encrypt_for_sync(&key, b"sofia").unwrap())
        .await
        .unwrap();
    backend
        .push("vault/Tasks.md", encrypt_for_sync(&key, b"tasks").unwrap())
        .await
        .unwrap();

    let all = backend.list("vault").await.unwrap();
    assert_eq!(all.len(), 3, "Expected 3 objects under vault/, got: {:?}", all);
}

#[tokio::test]
async fn test_encrypted_blob_not_plaintext() {
    let dir = TempDir::new().unwrap();
    let backend = make_backend(&dir);
    let key = test_key_a();

    let plaintext = b"Personal: Sofia is vegetarian.";
    let encrypted = encrypt_for_sync(&key, plaintext).unwrap();
    backend.push("People/Sofia.md", encrypted).await.unwrap();

    let stored = backend.pull("People/Sofia.md").await.unwrap();

    // The stored bytes must NOT contain the plaintext
    assert!(
        !stored.windows(plaintext.len()).any(|w| w == plaintext),
        "Stored blob must not contain plaintext"
    );
    // But must contain the ENGRAM_V1 magic prefix
    assert!(
        stored.starts_with(b"ENGRAM_V1:"),
        "Stored blob must start with ENGRAM_V1: magic prefix"
    );
}
```

**Step 2: Run the integration tests**

Run:
```bash
cargo test -p engram-sync --test integration_test
```

Expected: `4 tests pass`.

**Step 3: Run the full workspace test suite**

Run:
```bash
cargo test --workspace
```

Expected: all tests pass, 0 failures.

**Step 4: Run clippy across the workspace**

Run:
```bash
cargo clippy --workspace --all-targets -- -D warnings
```

Expected: 0 warnings treated as errors. If clippy catches issues, fix them before proceeding.

**Step 5: Run rustfmt check**

Run:
```bash
cargo fmt --all -- --check
```

If there are formatting issues, fix them with:
```bash
cargo fmt --all
```

Then confirm clean:
```bash
cargo fmt --all -- --check
```

Expected: exits with code 0 (no output).

**Step 6: Final commit**

```bash
git add crates/engram-sync/tests/ crates/engram-sync/src/s3.rs && \
git commit -m "test(sync): integration tests verifying encrypt-push-pull-decrypt round-trip"
```

---

## Done

After all 12 tasks are complete, verify the full feature set:

```bash
# All tests green
cargo test --workspace

# Help text for new commands
cargo run -p engram -- auth --help
cargo run -p engram -- auth add --help
cargo run -p engram -- auth add s3 --help
cargo run -p engram -- sync --help
```

The implementation is complete when:
- `cargo test --workspace` — all pass, 0 failures
- `cargo clippy --workspace --all-targets -- -D warnings` — clean
- `cargo fmt --all -- --check` — clean
- `engram auth add s3 --endpoint ... --bucket ... --access-key ... --secret-key ...` — stores in keychain and prints confirmation
- `engram auth list` — shows configured backends
- `engram auth remove s3` — clears credentials
- `engram sync` — encrypts vault files and pushes to configured backend
