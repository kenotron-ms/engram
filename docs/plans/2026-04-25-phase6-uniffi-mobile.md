# Engram — Phase 6: UniFFI Mobile Bindings

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Expose `engram-core` to iOS (Swift), Android (Kotlin), and Python via UniFFI — same Rust code, type-safe generated bindings, no FFI boilerplate by hand. Mobile apps can derive keys, encrypt/decrypt, and read/write the SQLCipher memory store natively.

**Architecture:** A UDL interface file declares the public API. `build.rs` runs `uniffi::generate_scaffolding` from it at compile time. `ffi.rs` provides flat wrapper functions and an `Arc<Mutex<MemoryStore>>` object handle — the FFI-friendly surface the scaffolding calls into. `uniffi-bindgen generate` produces Swift/Kotlin/Python source files checked into `bindings/`.

**Tech Stack:** uniffi 0.28 (`build` feature), cdylib + staticlib crate targets, `uniffi-bindgen` binary (via `uniffi::uniffi_bindgen_main()`)

---

## Pre-flight check

Before any task, confirm the existing test suite is green:

```bash
cd ~/workspace/ms/engram
cargo test -p engram-core
```

Expected: all crypto, store, and vault tests pass.

---

### Task 1: Add `EngramKey::from_bytes` + Cargo.toml + `build.rs`

**Files:**
- Modify: `crates/engram-core/src/crypto.rs`
- Modify: `crates/engram-core/Cargo.toml`
- Create: `crates/engram-core/build.rs`

---

**Step 1: Write the failing test**

In `crates/engram-core/src/crypto.rs`, add this test inside the existing `#[cfg(test)] mod tests { ... }` block:

```rust
#[test]
fn test_from_bytes_round_trips_with_as_bytes() {
    let original_bytes = [42u8; 32];
    let key = EngramKey::from_bytes(original_bytes);
    assert_eq!(
        key.as_bytes(),
        &original_bytes,
        "from_bytes → as_bytes must round-trip"
    );
}
```

**Step 2: Run to verify it fails**

```bash
cd ~/workspace/ms/engram
cargo test -p engram-core test_from_bytes_round_trips_with_as_bytes
```

Expected: compile error — `no method named 'from_bytes' found for struct 'EngramKey'`

**Step 3: Implement `EngramKey::from_bytes`**

In `crates/engram-core/src/crypto.rs`, inside `impl EngramKey`, add this method after `as_bytes`:

```rust
/// Construct an `EngramKey` from raw 32-byte key material.
///
/// Used by the FFI layer to round-trip key bytes across the language boundary.
pub fn from_bytes(bytes: [u8; 32]) -> Self {
    EngramKey(bytes)
}
```

**Step 4: Run test to verify it passes**

```bash
cargo test -p engram-core test_from_bytes_round_trips_with_as_bytes
```

Expected: PASS

**Step 5: Update `crates/engram-core/Cargo.toml`**

Replace the full contents of `Cargo.toml` with:

```toml
[package]
name = "engram-core"
version = "0.1.0"
edition = "2021"

[lib]
# lib     = normal Rust library (used by other crates in this workspace)
# cdylib  = dynamic library for UniFFI on macOS/Linux/Android
# staticlib = static library for UniFFI on iOS
crate-type = ["lib", "cdylib", "staticlib"]

[dependencies]
chacha20poly1305 = "0.10"
argon2 = "0.5"
keyring = "2"
rusqlite = { version = "0.31", features = ["bundled-sqlcipher"] }
rand = "0.8"
thiserror = "2"
serde = { version = "1", features = ["derive"] }
uuid = { version = "1", features = ["v4"] }
hex = "0.4"
uniffi = { version = "0.28", features = ["build"] }

[build-dependencies]
uniffi = { version = "0.28", features = ["build"] }

[dev-dependencies]
tempfile = "3"
```

**Step 6: Create `crates/engram-core/build.rs`**

Create the file at `crates/engram-core/build.rs` (alongside `Cargo.toml`, **not** inside `src/`):

```rust
fn main() {
    uniffi::generate_scaffolding("src/engram_core.udl").unwrap();
}
```

**Step 7: Verify the build now errors on the missing UDL**

```bash
cargo build -p engram-core 2>&1 | head -10
```

Expected: an error about `src/engram_core.udl` not found. This proves `build.rs` is executing. (The UDL is created in Task 2.)

**Step 8: Run all existing tests**

```bash
cargo test -p engram-core
```

Expected: all existing tests still pass (the test harness does not execute `build.rs` scaffolding generation when the UDL is absent — if it fails, skip ahead and create the UDL stub now, then return here).

**Step 9: Commit**

```bash
cd ~/workspace/ms/engram
git add crates/engram-core/src/crypto.rs \
        crates/engram-core/Cargo.toml \
        crates/engram-core/build.rs
git commit -m "feat(ffi): add EngramKey::from_bytes, cdylib/staticlib targets, uniffi build dep"
```

---

### Task 2: Create `src/engram_core.udl`

**Files:**
- Create: `crates/engram-core/src/engram_core.udl`

---

**Step 1: Create the UDL file**

Create `crates/engram-core/src/engram_core.udl` with exactly this content:

```udl
namespace engram_core {
    // Key derivation
    [Throws=EngramError]
    sequence<u8> derive_key(string password, sequence<u8> salt);

    sequence<u8> generate_salt();

    // Encryption / decryption
    [Throws=EngramError]
    sequence<u8> encrypt_bytes(sequence<u8> key_bytes, sequence<u8> plaintext);

    [Throws=EngramError]
    sequence<u8> decrypt_bytes(sequence<u8> key_bytes, sequence<u8> ciphertext);

    // Vault operations
    [Throws=EngramError]
    string vault_read(string vault_path, string relative_path);

    [Throws=EngramError]
    void vault_write(string vault_path, string relative_path, string content);

    [Throws=EngramError]
    sequence<string> vault_list_markdown(string vault_path);
};

[Error]
enum EngramError {
    "Crypto",
    "Vault",
    "Store",
    "InvalidInput",
};

interface MemoryStoreHandle {
    [Throws=EngramError]
    constructor(string db_path, sequence<u8> key_bytes);

    [Throws=EngramError]
    void insert_memory(string entity, string attribute, string value, string? source);

    [Throws=EngramError]
    MemoryRecord? get_memory(string id);

    [Throws=EngramError]
    sequence<MemoryRecord> find_by_entity(string entity);

    [Throws=EngramError]
    u64 record_count();
};

dictionary MemoryRecord {
    string id;
    string entity;
    string attribute;
    string value;
    string? source;
    i64 created_at;
    i64 updated_at;
};
```

**Step 2: Verify the build script runs without UDL errors**

```bash
cd ~/workspace/ms/engram
cargo build -p engram-core 2>&1 | head -30
```

Expected: the build script parses the UDL successfully. There will still be errors about missing Rust types and functions — that is fine. There should be **no** errors mentioning UDL syntax or "failed to parse UDL".

> **If you see "No such file or directory":** Verify the file is at `crates/engram-core/src/engram_core.udl` (not in the crate root). The path in `build.rs` is `"src/engram_core.udl"` relative to `crates/engram-core/`.

> **Note:** `lib.rs` is not wired up to include the scaffolding yet — that happens in Task 6. Tasks 3–5 build `ffi.rs` incrementally and run tests directly, without the full UniFFI compile chain active.

**Step 3: Commit**

```bash
git add crates/engram-core/src/engram_core.udl
git commit -m "feat(ffi): add UniFFI UDL interface definition"
```

---

### Task 3: `ffi.rs` — crypto FFI wrappers

**Files:**
- Create: `crates/engram-core/src/ffi.rs`
- Modify: `crates/engram-core/src/lib.rs` (add `pub mod ffi;`)

---

**Step 1: Create `crates/engram-core/src/ffi.rs`**

Create this file with the crypto wrappers, stubs for vault and store, and the test module:

```rust
// crates/engram-core/src/ffi.rs
//
// Flat, FFI-friendly wrappers over engram-core.
// No generics, no lifetimes — UniFFI requires concrete, owned types.
//
// NOTE: `uniffi::Error`, `uniffi::Record`, and `uniffi::Object` derives are
// present here but only activate after `uniffi::include_scaffolding!` is added
// to lib.rs in Task 6. If the build fails before Task 6, temporarily remove
// those derives, run tests, then restore them.

use crate::crypto::{decrypt, encrypt, generate_salt as gen_salt, EngramKey};
use crate::store::{Memory, MemoryStore};
use crate::vault::Vault;
use std::path::Path;
use std::sync::{Arc, Mutex};

// ─── Error type ──────────────────────────────────────────────────────────────

#[derive(Debug, thiserror::Error, uniffi::Error)]
pub enum EngramError {
    #[error("Crypto error: {0}")]
    Crypto(String),
    #[error("Vault error: {0}")]
    Vault(String),
    #[error("Store error: {0}")]
    Store(String),
    #[error("Invalid input: {0}")]
    InvalidInput(String),
}

// ─── Private helper ───────────────────────────────────────────────────────────

fn bytes_to_key(key_bytes: Vec<u8>) -> Result<EngramKey, EngramError> {
    let arr: [u8; 32] = key_bytes
        .try_into()
        .map_err(|_| EngramError::InvalidInput("Key must be exactly 32 bytes".into()))?;
    Ok(EngramKey::from_bytes(arr))
}

// ─── Crypto wrappers ─────────────────────────────────────────────────────────

pub fn derive_key(password: String, salt: Vec<u8>) -> Result<Vec<u8>, EngramError> {
    let salt_arr: [u8; 16] = salt
        .try_into()
        .map_err(|_| EngramError::InvalidInput("Salt must be exactly 16 bytes".into()))?;
    let key = EngramKey::derive(password.as_bytes(), &salt_arr)
        .map_err(|e| EngramError::Crypto(e.to_string()))?;
    Ok(key.as_bytes().to_vec())
}

pub fn generate_salt() -> Vec<u8> {
    gen_salt().to_vec()
}

pub fn encrypt_bytes(key_bytes: Vec<u8>, plaintext: Vec<u8>) -> Result<Vec<u8>, EngramError> {
    let key = bytes_to_key(key_bytes)?;
    encrypt(&key, &plaintext).map_err(|e| EngramError::Crypto(e.to_string()))
}

pub fn decrypt_bytes(key_bytes: Vec<u8>, ciphertext: Vec<u8>) -> Result<Vec<u8>, EngramError> {
    let key = bytes_to_key(key_bytes)?;
    decrypt(&key, &ciphertext).map_err(|e| EngramError::Crypto(e.to_string()))
}

// ─── Vault wrappers (stubs — implemented in Task 4) ──────────────────────────

pub fn vault_read(vault_path: String, relative_path: String) -> Result<String, EngramError> {
    todo!("vault_read — implemented in Task 4")
}

pub fn vault_write(
    vault_path: String,
    relative_path: String,
    content: String,
) -> Result<(), EngramError> {
    todo!("vault_write — implemented in Task 4")
}

pub fn vault_list_markdown(vault_path: String) -> Result<Vec<String>, EngramError> {
    todo!("vault_list_markdown — implemented in Task 4")
}

// ─── MemoryRecord + MemoryStoreHandle (stubs — implemented in Task 5) ────────

#[derive(Debug, Clone, uniffi::Record)]
pub struct MemoryRecord {
    pub id: String,
    pub entity: String,
    pub attribute: String,
    pub value: String,
    pub source: Option<String>,
    pub created_at: i64,
    pub updated_at: i64,
}

#[derive(uniffi::Object)]
pub struct MemoryStoreHandle {
    inner: Mutex<MemoryStore>,
}

#[uniffi::export]
impl MemoryStoreHandle {
    #[uniffi::constructor]
    pub fn new(db_path: String, key_bytes: Vec<u8>) -> Result<Arc<Self>, EngramError> {
        todo!("MemoryStoreHandle::new — implemented in Task 5")
    }

    pub fn insert_memory(
        &self,
        entity: String,
        attribute: String,
        value: String,
        source: Option<String>,
    ) -> Result<(), EngramError> {
        todo!("insert_memory — implemented in Task 5")
    }

    pub fn get_memory(&self, id: String) -> Result<Option<MemoryRecord>, EngramError> {
        todo!("get_memory — implemented in Task 5")
    }

    pub fn find_by_entity(&self, entity: String) -> Result<Vec<MemoryRecord>, EngramError> {
        todo!("find_by_entity — implemented in Task 5")
    }

    pub fn record_count(&self) -> Result<u64, EngramError> {
        todo!("record_count — implemented in Task 5")
    }
}

// ─── Tests ───────────────────────────────────────────────────────────────────

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
        let key_bytes = derive_key("test-password".to_string(), salt).unwrap();
        assert_eq!(key_bytes.len(), 32);

        let plaintext = b"hello ffi layer".to_vec();
        let ciphertext = encrypt_bytes(key_bytes.clone(), plaintext.clone()).unwrap();
        let decrypted = decrypt_bytes(key_bytes, ciphertext).unwrap();
        assert_eq!(decrypted, plaintext);
    }

    #[test]
    fn test_derive_key_wrong_salt_length_returns_invalid_input() {
        let bad_salt = vec![0u8; 10]; // must be 16
        let result = derive_key("password".to_string(), bad_salt);
        assert!(
            matches!(result, Err(EngramError::InvalidInput(_))),
            "expected InvalidInput, got: {:?}",
            result
        );
    }

    #[test]
    fn test_encrypt_bytes_wrong_key_length_returns_invalid_input() {
        let bad_key = vec![0u8; 10]; // must be 32
        let result = encrypt_bytes(bad_key, b"plaintext".to_vec());
        assert!(
            matches!(result, Err(EngramError::InvalidInput(_))),
            "expected InvalidInput for short key"
        );
    }

    #[test]
    fn test_decrypt_bytes_wrong_key_length_returns_invalid_input() {
        let bad_key = vec![0u8; 10];
        let result = decrypt_bytes(bad_key, b"bogus".to_vec());
        assert!(matches!(result, Err(EngramError::InvalidInput(_))));
    }
}
```

**Step 2: Add `ffi.rs` to `lib.rs`**

In `crates/engram-core/src/lib.rs`, add:

```rust
pub mod ffi;
```

The file should now read:

```rust
// engram-core: personal memory infrastructure

pub mod crypto;
pub mod ffi;
pub mod store;
pub mod vault;
```

**Step 3: Run the failing tests before implementation exists**

The crypto functions ARE implemented in Step 1 above. Run immediately:

```bash
cd ~/workspace/ms/engram
cargo test -p engram-core ffi::tests
```

Expected: 5 tests pass. (The todo! stubs in vault/store are not exercised yet.)

> **If the build fails with `uniffi::Error` / `uniffi::Record` / `uniffi::Object` not found:** The `uniffi` crate needs to be a runtime dependency (it is, from Task 1). If it still fails, temporarily remove the three uniffi derives from `EngramError`, `MemoryRecord`, and `MemoryStoreHandle`. Run tests. Restore the derives in Task 6.

**Step 4: Commit**

```bash
git add crates/engram-core/src/ffi.rs crates/engram-core/src/lib.rs
git commit -m "feat(ffi): add crypto FFI wrappers (derive_key, generate_salt, encrypt_bytes, decrypt_bytes)"
```

---

### Task 4: `ffi.rs` — vault FFI wrappers

**Files:**
- Modify: `crates/engram-core/src/ffi.rs` (replace vault stubs with real code + add tests)

---

**Step 1: Replace the three vault `todo!()` stubs in `ffi.rs`**

Find and replace each stub:

```rust
// REPLACE this:
pub fn vault_read(vault_path: String, relative_path: String) -> Result<String, EngramError> {
    todo!("vault_read — implemented in Task 4")
}

pub fn vault_write(
    vault_path: String,
    relative_path: String,
    content: String,
) -> Result<(), EngramError> {
    todo!("vault_write — implemented in Task 4")
}

pub fn vault_list_markdown(vault_path: String) -> Result<Vec<String>, EngramError> {
    todo!("vault_list_markdown — implemented in Task 4")
}

// WITH this:
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
```

**Step 2: Add vault tests to `mod tests` in `ffi.rs`**

Add `use tempfile::TempDir;` at the top of the `mod tests` block, then add these three tests:

```rust
// Add at the top of mod tests:
use tempfile::TempDir;

// Add these tests:
#[test]
fn test_vault_write_then_read() {
    let dir = TempDir::new().unwrap();
    let vault_path = dir.path().to_str().unwrap().to_string();

    vault_write(vault_path.clone(), "note.md".to_string(), "hello vault".to_string()).unwrap();
    let content = vault_read(vault_path, "note.md".to_string()).unwrap();
    assert_eq!(content, "hello vault");
}

#[test]
fn test_vault_list_markdown_returns_only_md_files() {
    let dir = TempDir::new().unwrap();
    let vault_path = dir.path().to_str().unwrap().to_string();

    vault_write(vault_path.clone(), "a.md".to_string(), "first".to_string()).unwrap();
    vault_write(vault_path.clone(), "b.md".to_string(), "second".to_string()).unwrap();
    vault_write(vault_path.clone(), "image.png".to_string(), "not md".to_string()).unwrap();

    let files = vault_list_markdown(vault_path).unwrap();
    assert!(files.contains(&"a.md".to_string()), "a.md must be listed");
    assert!(files.contains(&"b.md".to_string()), "b.md must be listed");
    assert!(
        !files.contains(&"image.png".to_string()),
        "image.png must NOT be listed"
    );
}

#[test]
fn test_vault_read_missing_file_returns_vault_error() {
    let dir = TempDir::new().unwrap();
    let vault_path = dir.path().to_str().unwrap().to_string();

    let result = vault_read(vault_path, "nonexistent.md".to_string());
    assert!(
        matches!(result, Err(EngramError::Vault(_))),
        "expected Vault error for missing file, got: {:?}",
        result
    );
}
```

**Step 3: Run to verify all ffi tests pass**

```bash
cd ~/workspace/ms/engram
cargo test -p engram-core ffi::tests
```

Expected: 8 tests pass (5 crypto + 3 vault).

**Step 4: Commit**

```bash
git add crates/engram-core/src/ffi.rs
git commit -m "feat(ffi): add vault FFI wrappers (vault_read, vault_write, vault_list_markdown)"
```

---

### Task 5: `ffi.rs` — `MemoryStoreHandle` implementation

**Files:**
- Modify: `crates/engram-core/src/ffi.rs` (replace store stubs with real code + add tests)

---

**Step 1: Add the `memory_to_record` helper after the vault wrappers**

In `ffi.rs`, add this private helper function before the `MemoryRecord` struct:

```rust
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
```

**Step 2: Replace the `MemoryStoreHandle` `todo!()` stubs with real implementations**

```rust
// REPLACE the entire #[uniffi::export] impl block with:
#[uniffi::export]
impl MemoryStoreHandle {
    /// Open (or create) an encrypted SQLite memory store at `db_path`.
    ///
    /// `key_bytes` must be exactly 32 bytes — the output of `derive_key`.
    #[uniffi::constructor]
    pub fn new(db_path: String, key_bytes: Vec<u8>) -> Result<Arc<Self>, EngramError> {
        let key = bytes_to_key(key_bytes)?;
        let store = MemoryStore::open(Path::new(&db_path), &key)
            .map_err(|e| EngramError::Store(e.to_string()))?;
        Ok(Arc::new(Self {
            inner: Mutex::new(store),
        }))
    }

    /// Insert an atomic memory record into the store.
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

    /// Fetch a memory by its UUID. Returns `None` if no record exists.
    pub fn get_memory(&self, id: String) -> Result<Option<MemoryRecord>, EngramError> {
        self.inner
            .lock()
            .unwrap()
            .get(&id)
            .map(|opt| opt.map(memory_to_record))
            .map_err(|e| EngramError::Store(e.to_string()))
    }

    /// Return all memories for `entity`, ordered newest-first.
    pub fn find_by_entity(&self, entity: String) -> Result<Vec<MemoryRecord>, EngramError> {
        self.inner
            .lock()
            .unwrap()
            .find_by_entity(&entity)
            .map(|v| v.into_iter().map(memory_to_record).collect())
            .map_err(|e| EngramError::Store(e.to_string()))
    }

    /// Return the total number of rows in the memory store.
    pub fn record_count(&self) -> Result<u64, EngramError> {
        self.inner
            .lock()
            .unwrap()
            .record_count()
            .map_err(|e| EngramError::Store(e.to_string()))
    }
}
```

**Step 3: Add `MemoryStoreHandle` tests to `mod tests` in `ffi.rs`**

Add this helper and four tests inside the existing `mod tests { ... }` block:

```rust
fn make_test_store() -> (Arc<MemoryStoreHandle>, TempDir) {
    let dir = TempDir::new().unwrap();
    let db_path = dir.path().join("test.db").to_str().unwrap().to_string();
    let key_bytes = vec![0u8; 32]; // 32-byte zero key — valid for tests
    let handle = MemoryStoreHandle::new(db_path, key_bytes).unwrap();
    (handle, dir)
}

#[test]
fn test_store_insert_get_find_count() {
    let (store, _dir) = make_test_store();

    store
        .insert_memory(
            "Sofia".to_string(),
            "dietary".to_string(),
            "vegetarian".to_string(),
            Some("2026-04-14 transcript".to_string()),
        )
        .unwrap();

    assert_eq!(store.record_count().unwrap(), 1);

    let records = store.find_by_entity("Sofia".to_string()).unwrap();
    assert_eq!(records.len(), 1);
    assert_eq!(records[0].entity, "Sofia");
    assert_eq!(records[0].value, "vegetarian");
    assert_eq!(
        records[0].source,
        Some("2026-04-14 transcript".to_string())
    );

    let got = store.get_memory(records[0].id.clone()).unwrap();
    assert!(got.is_some(), "get_memory should return the inserted record");
    assert_eq!(got.unwrap().attribute, "dietary");
}

#[test]
fn test_store_get_missing_returns_none() {
    let (store, _dir) = make_test_store();
    let got = store.get_memory("nonexistent-uuid".to_string()).unwrap();
    assert!(got.is_none());
}

#[test]
fn test_store_wrong_key_length_returns_invalid_input() {
    let dir = TempDir::new().unwrap();
    let db_path = dir.path().join("test.db").to_str().unwrap().to_string();
    let bad_key = vec![0u8; 10]; // must be 32

    let result = MemoryStoreHandle::new(db_path, bad_key);
    assert!(
        matches!(result, Err(EngramError::InvalidInput(_))),
        "expected InvalidInput for short key, got: {:?}",
        result
    );
}

#[test]
fn test_store_concurrent_inserts_do_not_panic() {
    let (store, _dir) = make_test_store();

    // store is Arc<MemoryStoreHandle> — clone the Arc for each thread
    let h1 = {
        let s = Arc::clone(&store);
        std::thread::spawn(move || {
            s.insert_memory(
                "ThreadA".to_string(),
                "attr".to_string(),
                "val1".to_string(),
                None,
            )
            .unwrap();
        })
    };

    let h2 = {
        let s = Arc::clone(&store);
        std::thread::spawn(move || {
            s.insert_memory(
                "ThreadB".to_string(),
                "attr".to_string(),
                "val2".to_string(),
                None,
            )
            .unwrap();
        })
    };

    h1.join().expect("thread A panicked");
    h2.join().expect("thread B panicked");

    assert_eq!(
        store.record_count().unwrap(),
        2,
        "both concurrent inserts must persist"
    );
}
```

**Step 4: Run to verify all ffi tests pass**

```bash
cd ~/workspace/ms/engram
cargo test -p engram-core ffi::tests
```

Expected: 12 tests pass (5 crypto + 3 vault + 4 store).

**Step 5: Run the full test suite**

```bash
cargo test -p engram-core
```

Expected: all tests pass — crypto, store, vault, and ffi modules.

**Step 6: Commit**

```bash
git add crates/engram-core/src/ffi.rs
git commit -m "feat(ffi): add MemoryStoreHandle FFI object with Arc<Mutex<MemoryStore>> thread safety"
```

---

### Task 6: Wire `lib.rs` + full UniFFI compile + all tests green

**Files:**
- Modify: `crates/engram-core/src/lib.rs`
- Verify: `crates/engram-core/src/ffi.rs` has all three uniffi derives in place

---

**Step 1: Verify the three uniffi derives are present in `ffi.rs`**

Check that these exact derives appear:

| Location in `ffi.rs` | Required derive |
|---|---|
| `pub enum EngramError` | `#[derive(Debug, thiserror::Error, uniffi::Error)]` |
| `pub struct MemoryRecord` | `#[derive(Debug, Clone, uniffi::Record)]` |
| `pub struct MemoryStoreHandle` | `#[derive(uniffi::Object)]` |
| `impl MemoryStoreHandle` | `#[uniffi::export]` on the impl block |

If any are missing (because you deferred them earlier), add them now.

**Step 2: Replace the full contents of `crates/engram-core/src/lib.rs`**

```rust
// engram-core: personal memory infrastructure

// UniFFI scaffolding — generated from src/engram_core.udl by build.rs.
// Must appear before module declarations that use uniffi derives.
uniffi::include_scaffolding!("engram_core");

pub mod crypto;
pub mod ffi;
pub mod store;
pub mod vault;

// Re-export the FFI surface to the crate root.
// The generated scaffolding resolves free functions and types by their
// unqualified names; this pub use makes them visible at crate root.
pub use ffi::{
    decrypt_bytes, derive_key, encrypt_bytes, generate_salt, vault_list_markdown, vault_read,
    vault_write, EngramError, MemoryRecord, MemoryStoreHandle,
};
```

**Step 3: Build the crate**

```bash
cd ~/workspace/ms/engram
cargo build -p engram-core
```

Expected: clean build with no errors. This is the first time the full UniFFI scaffolding compiles together with `ffi.rs`.

> **Common errors at this step:**
>
> - `cannot find type 'EngramError' in this scope` → Verify the `pub use ffi::EngramError;` line is in `lib.rs`.
> - `the trait 'uniffi::LiftReturn<UniFfiTag>' is not implemented` → A type is missing a uniffi derive. Check all three types in `ffi.rs`.
> - `function 'vault_read' has wrong number of parameters` → The UDL signature in `engram_core.udl` doesn't match the Rust function signature in `ffi.rs`. Compare them carefully — UDL `string` maps to Rust `String`, `sequence<u8>` to `Vec<u8>`, `string?` to `Option<String>`.
> - `conflicting implementations` of a uniffi trait → The `uniffi::include_scaffolding!` may be duplicating something. Ensure `uniffi::include_scaffolding!` appears exactly once, at the top of `lib.rs`, before any `mod` declarations.

**Step 4: Run the complete test suite**

```bash
cargo test -p engram-core
```

Expected: ALL tests pass — crypto, store, vault, and all 12 ffi tests.

**Step 5: Commit**

```bash
git add crates/engram-core/src/lib.rs crates/engram-core/src/ffi.rs
git commit -m "feat(ffi): wire UniFFI scaffolding in lib.rs, re-export FFI surface to crate root"
```

---

### Task 7: Generate Swift, Kotlin, and Python bindings

**Files:**
- Create: `crates/engram-core/src/bin/uniffi-bindgen.rs`
- Modify: `crates/engram-core/Cargo.toml` (add `[[bin]]` target)
- Create: `crates/engram-core/bindings/swift/` (generated)
- Create: `crates/engram-core/bindings/kotlin/` (generated)
- Create: `crates/engram-core/bindings/python/` (generated)

---

**Step 1: Create the `uniffi-bindgen` binary entry point**

Create `crates/engram-core/src/bin/uniffi-bindgen.rs`:

```rust
fn main() {
    uniffi::uniffi_bindgen_main()
}
```

**Step 2: Register the binary in `Cargo.toml`**

In `crates/engram-core/Cargo.toml`, add this block after `[build-dependencies]`:

```toml
[[bin]]
name = "uniffi-bindgen"
path = "src/bin/uniffi-bindgen.rs"
```

**Step 3: Create the bindings output directories**

```bash
cd ~/workspace/ms/engram/crates/engram-core
mkdir -p bindings/swift bindings/kotlin bindings/python
```

**Step 4: Generate Swift bindings**

```bash
cd ~/workspace/ms/engram
cargo run -p engram-core --bin uniffi-bindgen -- generate \
  crates/engram-core/src/engram_core.udl \
  --language swift \
  --out-dir crates/engram-core/bindings/swift/
```

Expected: exits 0.

Verify the output files exist:

```bash
ls crates/engram-core/bindings/swift/
```

Expected: `engram_core.swift`, `engram_coreFFI.h`, `engram_coreFFI.modulemap`

**Step 5: Generate Kotlin bindings**

```bash
cargo run -p engram-core --bin uniffi-bindgen -- generate \
  crates/engram-core/src/engram_core.udl \
  --language kotlin \
  --out-dir crates/engram-core/bindings/kotlin/
```

Expected: exits 0.

```bash
ls crates/engram-core/bindings/kotlin/uniffi/engram_core/
```

Expected: `engram_core.kt`

**Step 6: Generate Python bindings**

```bash
cargo run -p engram-core --bin uniffi-bindgen -- generate \
  crates/engram-core/src/engram_core.udl \
  --language python \
  --out-dir crates/engram-core/bindings/python/
```

Expected: exits 0.

```bash
ls crates/engram-core/bindings/python/
```

Expected: `engram_core.py`

**Step 7: Syntax-check the generated Swift (macOS only)**

```bash
swiftc -parse crates/engram-core/bindings/swift/engram_core.swift 2>&1
```

Expected: no errors. Warnings about `Foundation` or `ffi` module are acceptable. If `swiftc` is not installed, skip this step.

**Step 8: Commit**

```bash
git add crates/engram-core/src/bin/ \
        crates/engram-core/Cargo.toml \
        crates/engram-core/bindings/
git commit -m "feat(ffi): generate Swift, Kotlin, Python bindings via uniffi-bindgen"
```

---

### Task 8: Usage examples + README update

**Files:**
- Create: `crates/engram-core/bindings/SWIFT_USAGE.md`
- Create: `crates/engram-core/bindings/KOTLIN_USAGE.md`
- Modify: `README.md`

---

**Step 1: Create `crates/engram-core/bindings/SWIFT_USAGE.md`**

````markdown
# Swift / iOS Usage — EngramCore

> Generated bindings are in `bindings/swift/`. Copy `engram_core.swift` and
> `engram_coreFFI.h` into your Xcode project and link against `libengram_core.a`
> (build with `cargo build --target aarch64-apple-ios --release`).

## Key derivation and encryption

```swift
import Foundation

// 1. Generate a random salt and derive a 32-byte key from the user's password.
//    The salt is not secret — store it alongside the encrypted data.
let salt = generateSalt()
let keyBytes = try deriveKey(password: "user-passphrase", salt: salt)

// 2. Encrypt arbitrary bytes
let plaintext: [UInt8] = Array("Hello, engram!".utf8)
let ciphertext = try encryptBytes(keyBytes: keyBytes, plaintext: plaintext)

// 3. Decrypt
let decrypted = try decryptBytes(keyBytes: keyBytes, ciphertext: ciphertext)
assert(decrypted == plaintext)
```

## Vault operations

```swift
let vaultPath = "\(NSHomeDirectory())/.engram/vault"

// Write a markdown file into the vault (creates parent directories automatically)
try vaultWrite(
    vaultPath: vaultPath,
    relativePath: "People/Sofia.md",
    content: "# Sofia\n\n- dietary: vegetarian"
)

// Read it back
let content = try vaultRead(vaultPath: vaultPath, relativePath: "People/Sofia.md")

// List all .md files recursively
let files: [String] = try vaultListMarkdown(vaultPath: vaultPath)
```

## Memory store

```swift
let dbPath = "\(NSHomeDirectory())/.engram/memory.db"

// Open (or create) the encrypted SQLite memory store
let store = try MemoryStoreHandle(dbPath: dbPath, keyBytes: keyBytes)

// Insert an atomic fact
try store.insertMemory(
    entity: "Sofia",
    attribute: "dietary",
    value: "vegetarian",
    source: "2026-04-14 transcript"
)

// Query all facts for an entity (newest first)
let memories: [MemoryRecord] = try store.findByEntity(entity: "Sofia")
for m in memories {
    print("\(m.entity) — \(m.attribute): \(m.value)")
}

// Fetch by UUID
if let record = try store.getMemory(id: memories[0].id) {
    print("Found: \(record.value)")
}

print("Total records: \(try store.recordCount())")
```
````

**Step 2: Create `crates/engram-core/bindings/KOTLIN_USAGE.md`**

````markdown
# Kotlin / Android Usage — EngramCore

> Generated bindings are in `bindings/kotlin/`. Copy the `uniffi/engram_core/`
> directory into your Android project's `src/main/java/` tree and include
> `libengram_core.so` in `src/main/jniLibs/arm64-v8a/`
> (build with `cargo build --target aarch64-linux-android --release`).

## Key derivation and encryption

```kotlin
import uniffi.engram_core.*

// Derive a 32-byte key from the user's password
val salt = generateSalt()
val keyBytes = deriveKey("user-passphrase", salt)

// Encrypt
val plaintext = "Hello, engram!".toByteArray().toList()
val ciphertext = encryptBytes(keyBytes, plaintext)

// Decrypt
val decrypted = decryptBytes(keyBytes, ciphertext)
assert(decrypted == plaintext)
```

## Vault operations

```kotlin
val vaultPath = "${System.getProperty("user.home")}/.engram/vault"

// Write a markdown file (parent directories created automatically)
vaultWrite(vaultPath, "People/Sofia.md", "# Sofia\n\n- dietary: vegetarian")

// Read it back
val content: String = vaultRead(vaultPath, "People/Sofia.md")

// List all .md files recursively
val files: List<String> = vaultListMarkdown(vaultPath)
```

## Memory store

```kotlin
val dbPath = "${System.getProperty("user.home")}/.engram/memory.db"

// Open (or create) the encrypted SQLite memory store
val store = MemoryStoreHandle(dbPath, keyBytes)

// Insert an atomic fact
store.insertMemory("Sofia", "dietary", "vegetarian", "2026-04-14 transcript")

// Query all facts for an entity (newest first)
val memories: List<MemoryRecord> = store.findByEntity("Sofia")
memories.forEach { println("${it.entity} — ${it.attribute}: ${it.value}") }

// Fetch by UUID
val record: MemoryRecord? = store.getMemory(memories[0].id)
println("Source: ${record?.source}")

println("Total: ${store.recordCount()}")
```
````

**Step 3: Add "Mobile Integration (UniFFI)" section to `README.md`**

Open `~/workspace/ms/engram/README.md`. Append the following section **before** any trailing blank lines at the end of the file:

```markdown
---

## Mobile Integration (UniFFI)

`engram-core` exposes its full API to iOS, Android, and Python via
[UniFFI](https://mozilla.github.io/uniffi-rs/). The same Rust code that runs
on your desktop compiles natively to all five platforms.

### Supported targets

| Platform | Target triple | Output |
|---|---|---|
| macOS | `aarch64-apple-darwin` | `libengram_core.dylib` |
| iOS | `aarch64-apple-ios` | `libengram_core.a` (staticlib) |
| Android (arm64) | `aarch64-linux-android` | `libengram_core.so` |
| Windows | `x86_64-pc-windows-msvc` | `engram_core.dll` |
| Linux | `x86_64-unknown-linux-gnu` | `libengram_core.so` |

### Generate language bindings

```bash
# Swift (iOS/macOS)
cargo run -p engram-core --bin uniffi-bindgen -- generate \
  crates/engram-core/src/engram_core.udl \
  --language swift \
  --out-dir crates/engram-core/bindings/swift/

# Kotlin (Android)
cargo run -p engram-core --bin uniffi-bindgen -- generate \
  crates/engram-core/src/engram_core.udl \
  --language kotlin \
  --out-dir crates/engram-core/bindings/kotlin/

# Python
cargo run -p engram-core --bin uniffi-bindgen -- generate \
  crates/engram-core/src/engram_core.udl \
  --language python \
  --out-dir crates/engram-core/bindings/python/
```

Pre-generated bindings are checked in at `crates/engram-core/bindings/`.
See [`SWIFT_USAGE.md`](crates/engram-core/bindings/SWIFT_USAGE.md) and
[`KOTLIN_USAGE.md`](crates/engram-core/bindings/KOTLIN_USAGE.md) for usage examples.

### Exposed API surface

| Symbol | Description |
|---|---|
| `derive_key(password, salt)` | Argon2id KDF → 32-byte key |
| `generate_salt()` | Cryptographically random 16-byte salt |
| `encrypt_bytes(key, plaintext)` | XChaCha20-Poly1305 encrypt |
| `decrypt_bytes(key, ciphertext)` | XChaCha20-Poly1305 decrypt |
| `vault_read(path, rel)` | Read a file from the markdown vault |
| `vault_write(path, rel, content)` | Write a file to the vault (creates dirs) |
| `vault_list_markdown(path)` | List all `.md` files recursively |
| `MemoryStoreHandle` | Thread-safe handle to the SQLCipher memory store |
| `MemoryRecord` | Dictionary: id, entity, attribute, value, source, timestamps |
```

**Step 4: Run the final test suite**

```bash
cd ~/workspace/ms/engram
cargo test -p engram-core
```

Expected: all tests pass (0 failures).

**Step 5: Final commit**

```bash
git add crates/engram-core/bindings/SWIFT_USAGE.md \
        crates/engram-core/bindings/KOTLIN_USAGE.md \
        README.md
git commit -m "feat(ffi): UniFFI bindings for iOS/Android/Python — Swift and Kotlin usage guides, README update"
```

---

## Verification Checklist

After all 8 tasks complete, verify each item:

- [ ] `cargo test -p engram-core` — all tests pass (0 failures, 0 errors)
- [ ] `cargo build -p engram-core` — clean build, no errors
- [ ] `crates/engram-core/src/crypto.rs` — `EngramKey::from_bytes` is `pub`
- [ ] `crates/engram-core/Cargo.toml` — `crate-type = ["lib", "cdylib", "staticlib"]`
- [ ] `crates/engram-core/build.rs` — exists, calls `uniffi::generate_scaffolding`
- [ ] `crates/engram-core/src/engram_core.udl` — exists with namespace, Error, interface, dictionary
- [ ] `crates/engram-core/src/ffi.rs` — `EngramError`, `MemoryRecord`, `MemoryStoreHandle`, 7 free functions, all uniffi derives present
- [ ] `crates/engram-core/src/lib.rs` — `uniffi::include_scaffolding!("engram_core")` + `pub use ffi::...`
- [ ] `crates/engram-core/src/bin/uniffi-bindgen.rs` — exists, calls `uniffi_bindgen_main()`
- [ ] `crates/engram-core/bindings/swift/engram_core.swift` — exists and non-empty
- [ ] `crates/engram-core/bindings/kotlin/uniffi/engram_core/engram_core.kt` — exists
- [ ] `crates/engram-core/bindings/python/engram_core.py` — exists
- [ ] `README.md` — contains "Mobile Integration (UniFFI)" section

---

## Troubleshooting Reference

**`uniffi::Object` / `uniffi::Record` / `uniffi::Error` derive not found**
→ Both `[dependencies]` and `[build-dependencies]` need `uniffi = { version = "0.28", features = ["build"] }`. Verify with `cargo tree -p engram-core | grep uniffi`.

**`uniffi_bindgen_main` function not found**
→ This function is in `uniffi` 0.28. If missing, check `cargo tree -p engram-core | grep uniffi` for the actual version. Ensure `uniffi` is a `[dependencies]` entry (not only `[build-dependencies]`).

**UDL type signature mismatch error at compile time**
→ UDL-to-Rust type mapping: `string` ↔ `String`, `sequence<u8>` ↔ `Vec<u8>`, `string?` ↔ `Option<String>`, `u64` ↔ `u64`, `i64` ↔ `i64`, `void` ↔ `()`. Compare each UDL function against its Rust counterpart in `ffi.rs`.

**`MemoryStore is not Sync` / trait not satisfied**
→ `Arc<Mutex<MemoryStore>>` is `Send + Sync` as long as `MemoryStore: Send`. `rusqlite::Connection` has been `Send` since rusqlite 0.25. If this error appears, verify you're on rusqlite 0.31 (from `Cargo.toml`) and that `MemoryStoreHandle` has `#[derive(uniffi::Object)]`.

**Generated Swift file is empty or bindings directory is missing files**
→ Verify the UDL path passed to `uniffi-bindgen generate` is correct. Use an absolute path if needed:
```bash
cargo run -p engram-core --bin uniffi-bindgen -- generate \
  "$(pwd)/crates/engram-core/src/engram_core.udl" --language swift \
  --out-dir "$(pwd)/crates/engram-core/bindings/swift/"
```

**`conflicting implementations` of a uniffi trait in lib.rs**
→ `uniffi::include_scaffolding!("engram_core")` must appear exactly once, at the very top of `lib.rs`, before any `mod` declarations. Check that you haven't accidentally left an older call or duplicated the macro.
