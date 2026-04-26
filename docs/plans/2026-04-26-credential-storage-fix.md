# Engram — Credential Storage Fix (No More Keychain Panic)

> **Execution:** Use the subagent-driven-development workflow to implement this plan.

**Goal:** Fix the "User interaction is not allowed" keychain panic by moving to a passphrase-derived vault key (never stored, works everywhere) and storing backend credentials in `~/.engram/config.toml` with 0600 permissions — the same approach rclone uses.

**Architecture:** Vault key derived via Argon2id(passphrase + stored salt) — three-tier resolution: `ENGRAM_VAULT_KEY` env var → `ENGRAM_VAULT_PASSPHRASE` env var → interactive prompt. Backend credentials (S3 access key, OAuth tokens) written to `~/.engram/config.toml` (0600) under `[vaults.<name>.sync]`. OS keychain completely removed from the primary flow. `engram init` becomes a real command that generates a salt, accepts a passphrase, and never stores the key.

**Tech Stack:** Rust, toml 0.8 (already present), base64 0.22 (needs adding to engram-cli), rpassword 7 (already present), Argon2id via argon2 crate (already in engram-core), generate_salt() (already in engram-core::crypto)

---

## Before You Start: Understand the Codebase

### What exists now

- `crates/engram-core/src/config.rs` — `EngramConfig { vaults: BTreeMap<String, VaultEntry> }`. No `key` field yet.
- `crates/engram-core/src/crypto.rs` — `KeyStore` writes to OS keychain. `EngramKey::derive(password, salt)` exists. `generate_salt() -> [u8; 16]` exists. Both are already `pub`.
- `crates/engram-cli/src/main.rs` — No `Init` command/variant. `run_auth_add_s3` calls `AuthStore::store`. `run_sync` calls `AuthStore::retrieve().unwrap()` (lines 672–675). Five functions call `KeyStore::new("engram").retrieve()`: `run_mcp`, `run_sync`, `run_load`, `run_observe`, `run_daemon`. `run_status` and `run_doctor` also call `key_store.retrieve()`.
- `crates/engram-sync/src/auth.rs` — `AuthStore` wraps the OS keychain. After this fix it is no longer called by the CLI (it stays in the codebase but is unused by the new flow).

### Critical facts about signatures

- `EngramConfig::add_vault` takes `(name: String, entry: VaultEntry)` — always pass `"name".to_string()`, not a bare `&str`.
- `VaultEntry` does not derive `Default`. Construct all fields explicitly.
- `base64 = { version = "0.22", features = ["std"] }` is **not** in `engram-cli/Cargo.toml`. It is in `engram-sync`. You must add it to `engram-cli`.
- `resolve_vault_key()` will be a private `fn` in `main.rs`. Its unit tests must live **inline** in `main.rs` inside `#[cfg(test)] mod tests { ... }`, not in `tests/`. The `tests/` integration tests can only call the compiled binary.
- Tests in `config.rs` that set `ENGRAM_CONFIG_PATH` must acquire `env_lock()` (the `OnceLock<Mutex<()>>` already defined in that file's test module). Copy the pattern.
- `run_vault_add` constructs `VaultEntry` with explicit fields — after you add `sync: Option<SyncCredentials>` to `VaultEntry`, you must add `sync: None` to that construction (line 1706 in main.rs).

---

## Task 1: Add `KeyConfig` and `SyncCredentials` to `config.rs`

**Files:**
- Modify: `crates/engram-core/src/config.rs`

### Step 1: Write the failing tests

Add to the `#[cfg(test)] mod tests { ... }` block at the bottom of `config.rs`. Acquire `env_lock()` in any test that sets `ENGRAM_CONFIG_PATH`.

```rust
#[test]
fn test_key_config_roundtrip() {
    let mut config = EngramConfig::default();
    config.key.salt = Some("dGVzdHNhbHQ=".to_string());
    let toml = toml::to_string_pretty(&config).unwrap();
    let back: EngramConfig = toml::from_str(&toml).unwrap();
    assert_eq!(back.key.salt.as_deref(), Some("dGVzdHNhbHQ="));
}

#[test]
fn test_sync_credentials_roundtrip() {
    let mut config = EngramConfig::default();
    config.add_vault(
        "personal".to_string(),
        VaultEntry {
            path: PathBuf::from("/vaults/personal"),
            access: VaultAccess::ReadWrite,
            sync_mode: SyncMode::Auto,
            default: true,
            vault_type: None,
            sync: Some(SyncCredentials {
                backend: "s3".to_string(),
                endpoint: Some("https://r2.example.com".to_string()),
                bucket: Some("my-vault".to_string()),
                access_key: Some("AKIA123".to_string()),
                secret_key: Some("secret456".to_string()),
                ..Default::default()
            }),
        },
    );
    let toml = toml::to_string_pretty(&config).unwrap();
    let back: EngramConfig = toml::from_str(&toml).unwrap();
    let sync = back.vaults["personal"].sync.as_ref().unwrap();
    assert_eq!(sync.backend, "s3");
    assert_eq!(sync.access_key.as_deref(), Some("AKIA123"));
}

#[test]
fn test_config_missing_key_section_uses_default() {
    let toml = r#"
[vaults.personal]
path = "/vaults/personal"
"#;
    let config: EngramConfig = toml::from_str(toml).unwrap();
    assert!(config.key.salt.is_none());
}

#[test]
fn test_sync_field_absent_when_none() {
    // A vault with no sync credentials must not emit a [sync] section.
    let mut config = EngramConfig::default();
    config.add_vault(
        "work".to_string(),
        VaultEntry {
            path: PathBuf::from("/vaults/work"),
            access: VaultAccess::ReadWrite,
            sync_mode: SyncMode::Approval,
            default: false,
            vault_type: None,
            sync: None,
        },
    );
    let toml = toml::to_string_pretty(&config).unwrap();
    assert!(
        !toml.contains("[vaults.work.sync]"),
        "sync section must be absent when None:\n{toml}"
    );
}
```

### Step 2: Run tests to verify they fail

```
cd ~/workspace/ms/engram
cargo test -p engram-core test_key_config_roundtrip test_sync_credentials_roundtrip test_config_missing_key_section_uses_default test_sync_field_absent_when_none 2>&1 | tail -20
```

Expected: compile error — `KeyConfig`, `SyncCredentials`, and `VaultEntry::sync` do not exist yet.

### Step 3: Implement the new types

In `crates/engram-core/src/config.rs`, make the following changes:

**A. Add two new structs** — place them in the `// Structs` section, before `VaultEntry`:

```rust
/// Vault key configuration — stores the Argon2id salt only.
/// The key itself is NEVER stored; it is derived on demand from a passphrase.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct KeyConfig {
    /// Base64-encoded 16-byte random salt. Generated once by `engram init`. Not secret.
    pub salt: Option<String>,
}

/// Backend sync credentials stored in config.toml (not the OS keychain).
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SyncCredentials {
    pub backend: String,
    pub endpoint: Option<String>,
    pub bucket: Option<String>,
    pub access_key: Option<String>,
    pub secret_key: Option<String>,
    pub container: Option<String>,
    pub account: Option<String>,
    pub access_token: Option<String>,
    pub refresh_token: Option<String>,
    pub folder: Option<String>,
}
```

**B. Update `VaultEntry`** — add the `sync` field with skip-serialization-if-None:

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VaultEntry {
    pub path: PathBuf,
    #[serde(default = "default_access")]
    pub access: VaultAccess,
    #[serde(default = "default_sync_mode")]
    pub sync_mode: SyncMode,
    #[serde(default)]
    pub default: bool,
    #[serde(rename = "type")]
    pub vault_type: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sync: Option<SyncCredentials>,
}
```

**C. Update `EngramConfig`** — add the `key` field:

```rust
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct EngramConfig {
    #[serde(default)]
    pub key: KeyConfig,
    #[serde(default)]
    pub vaults: BTreeMap<String, VaultEntry>,
}
```

**D. In `run_vault_add` inside `crates/engram-cli/src/main.rs`** — add `sync: None` to the `VaultEntry` construction at line ~1706 (do this now to prevent the compile error):

```rust
let entry = VaultEntry {
    path: expanded_path,
    access: access_mode,
    sync_mode: sync,
    default,
    vault_type: vault_type.map(|s| s.to_string()),
    sync: None,  // add this line
};
```

### Step 4: Run tests to verify they pass

```
cargo test -p engram-core test_key_config_roundtrip test_sync_credentials_roundtrip test_config_missing_key_section_uses_default test_sync_field_absent_when_none 2>&1 | tail -10
```

Expected: all 4 tests PASS.

Also verify the full engram-core test suite still passes:
```
cargo test -p engram-core 2>&1 | tail -15
```

Expected: all tests PASS.

### Step 5: Commit

```
git add crates/engram-core/src/config.rs crates/engram-cli/src/main.rs
git commit -m "feat(config): add KeyConfig (Argon2id salt storage) and SyncCredentials (backend creds in config file)"
```

---

## Task 2: Set 0600 Permissions on `config.toml` After Save

**Files:**
- Modify: `crates/engram-core/src/config.rs`

### Step 1: Write the failing test

Add to the `#[cfg(test)] mod tests { ... }` block in `config.rs`. This test must acquire `env_lock()`:

```rust
#[cfg(unix)]
#[test]
fn test_save_sets_0600_permissions() {
    use std::os::unix::fs::PermissionsExt;
    use tempfile::tempdir;

    let _guard = env_lock();
    let dir = tempdir().unwrap();
    let config_path = dir.path().join("config.toml");
    std::env::set_var("ENGRAM_CONFIG_PATH", &config_path);

    let config = EngramConfig::default();
    config.save().unwrap();

    let meta = std::fs::metadata(&config_path).unwrap();
    let mode = meta.permissions().mode() & 0o777;
    assert_eq!(mode, 0o600, "config.toml must be 0600, got: {:o}", mode);

    std::env::remove_var("ENGRAM_CONFIG_PATH");
}
```

### Step 2: Run test to verify it fails

```
cargo test -p engram-core test_save_sets_0600_permissions 2>&1 | tail -10
```

Expected: FAIL — permissions will be 0644 (umask default) not 0600.

### Step 3: Update `EngramConfig::save()`

In `crates/engram-core/src/config.rs`, find the `save()` method and add the permission-setting block immediately after the `std::fs::rename` call:

```rust
pub fn save(&self) -> Result<(), ConfigError> {
    let path = Self::config_path();

    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }

    let serialised = toml::to_string(self)?;

    let tmp_path = path.with_extension("toml.tmp");
    std::fs::write(&tmp_path, &serialised)?;
    std::fs::rename(&tmp_path, &path)?;

    // Restrict config.toml to owner-only (0600): it may contain credentials.
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o600))
            .map_err(ConfigError::Io)?;
    }

    Ok(())
}
```

### Step 4: Run test to verify it passes

```
cargo test -p engram-core test_save_sets_0600_permissions 2>&1 | tail -10
```

Expected: PASS.

Full suite still passes:
```
cargo test -p engram-core 2>&1 | tail -10
```

### Step 5: Commit

```
git add crates/engram-core/src/config.rs
git commit -m "feat(config): enforce 0600 permissions on config.toml after save (protects stored credentials)"
```

---

## Task 3: Add `resolve_vault_key()` to `engram-cli`

**Files:**
- Modify: `crates/engram-cli/Cargo.toml` — add `base64`
- Modify: `crates/engram-cli/src/main.rs` — add `resolve_vault_key()` function and inline tests

### Step 1: Write the failing tests

Add these tests to the `#[cfg(test)] mod tests { ... }` block at the bottom of `main.rs`. They are inline unit tests (not in `tests/`) because `resolve_vault_key()` is a private function.

```rust
#[cfg(test)]
mod tests {
    use super::*;
    // ... existing tests ...

    // ── resolve_vault_key tests ──────────────────────────────────────────────

    #[test]
    #[serial]
    fn test_resolve_key_from_vault_key_env_var() {
        use base64::Engine;
        use base64::engine::general_purpose::STANDARD as B64;
        use tempfile::TempDir;

        std::env::remove_var("ENGRAM_VAULT_PASSPHRASE");
        let key_bytes = [42u8; 32];
        let b64 = B64.encode(key_bytes);
        std::env::set_var("ENGRAM_VAULT_KEY", &b64);

        // Point config at an empty dir so no salt is needed for tier 1.
        let dir = TempDir::new().unwrap();
        std::env::set_var("ENGRAM_CONFIG_PATH", dir.path().join("config.toml"));

        let result = resolve_vault_key();
        assert!(result.is_ok(), "tier-1 should succeed: {:?}", result.err());
        assert_eq!(result.unwrap().as_bytes(), &[42u8; 32]);

        std::env::remove_var("ENGRAM_VAULT_KEY");
        std::env::remove_var("ENGRAM_CONFIG_PATH");
    }

    #[test]
    #[serial]
    fn test_resolve_key_from_passphrase_env_var() {
        use base64::Engine;
        use base64::engine::general_purpose::STANDARD as B64;
        use tempfile::TempDir;

        std::env::remove_var("ENGRAM_VAULT_KEY");
        std::env::set_var("ENGRAM_VAULT_PASSPHRASE", "test-passphrase-123");

        let dir = TempDir::new().unwrap();
        let config_path = dir.path().join("config.toml");
        let salt = [0u8; 16];
        let salt_b64 = B64.encode(salt);
        std::fs::write(&config_path, format!("[key]\nsalt = \"{}\"\n", salt_b64)).unwrap();
        std::env::set_var("ENGRAM_CONFIG_PATH", &config_path);

        let r1 = resolve_vault_key();
        let r2 = resolve_vault_key();
        assert!(r1.is_ok(), "tier-2 should succeed: {:?}", r1.err());
        // Same passphrase + same salt = deterministic key.
        assert_eq!(r1.unwrap().as_bytes(), r2.unwrap().as_bytes());

        std::env::remove_var("ENGRAM_VAULT_PASSPHRASE");
        std::env::remove_var("ENGRAM_CONFIG_PATH");
    }

    #[test]
    #[serial]
    fn test_resolve_key_fails_gracefully_when_not_initialized() {
        use tempfile::TempDir;

        std::env::remove_var("ENGRAM_VAULT_KEY");
        std::env::remove_var("ENGRAM_VAULT_PASSPHRASE");

        let dir = TempDir::new().unwrap();
        // Config file with no [key] section → salt is None.
        std::env::set_var("ENGRAM_CONFIG_PATH", dir.path().join("config.toml"));

        let result = resolve_vault_key();
        assert!(result.is_err(), "should return Err when not initialized");
        assert!(
            result.unwrap_err().contains("engram init"),
            "error message must mention 'engram init'"
        );

        std::env::remove_var("ENGRAM_CONFIG_PATH");
    }

    #[test]
    #[serial]
    fn test_resolve_key_invalid_base64_vault_key_env() {
        std::env::remove_var("ENGRAM_VAULT_PASSPHRASE");
        std::env::set_var("ENGRAM_VAULT_KEY", "not_valid_base64!!!");

        let dir = tempfile::TempDir::new().unwrap();
        std::env::set_var("ENGRAM_CONFIG_PATH", dir.path().join("config.toml"));

        let result = resolve_vault_key();
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("base64"));

        std::env::remove_var("ENGRAM_VAULT_KEY");
        std::env::remove_var("ENGRAM_CONFIG_PATH");
    }
}
```

### Step 2: Run tests to verify they fail

```
cargo test -p engram 2>&1 | grep -E "error|test_resolve" | head -20
```

Expected: compile error — `resolve_vault_key`, `base64`, `serial` not found.

### Step 3: Add `base64` to `engram-cli/Cargo.toml`

In `crates/engram-cli/Cargo.toml`, add to `[dependencies]`:
```toml
base64 = { version = "0.22", features = ["std"] }
```

### Step 4: Add `use serial_test::serial;` import and the function to `main.rs`

At the top of `main.rs`, add to the imports:
```rust
use base64::Engine;
use base64::engine::general_purpose::STANDARD as B64;
```

Add the `resolve_vault_key()` function anywhere before `main()` (e.g. after the `default_vectors_path` helper at line ~870):

```rust
/// Resolve the vault encryption key using three-tier fallback.
///
/// Tier 1 — `ENGRAM_VAULT_KEY` env var: pre-derived key, base64-encoded 32 bytes.
///           No config read required. Useful for CI.
/// Tier 2 — `ENGRAM_VAULT_PASSPHRASE` env var: derive key using Argon2id + salt
///           read from `~/.engram/config.toml`. Useful for remote/SSH.
/// Tier 3 — Interactive passphrase prompt via rpassword.
///
/// Never panics. Returns a human-friendly Err on failure.
fn resolve_vault_key() -> Result<engram_core::crypto::EngramKey, String> {
    use engram_core::crypto::EngramKey;

    // Tier 1: pre-derived key as raw base64
    if let Ok(b64) = std::env::var("ENGRAM_VAULT_KEY") {
        let bytes = B64
            .decode(&b64)
            .map_err(|_| "ENGRAM_VAULT_KEY is not valid base64".to_string())?;
        let arr: [u8; 32] = bytes
            .try_into()
            .map_err(|_| "ENGRAM_VAULT_KEY must decode to exactly 32 bytes".to_string())?;
        return Ok(EngramKey::from_bytes(arr));
    }

    // Tiers 2 and 3 need the salt from config.
    let config = EngramConfig::load();
    let salt_b64 = config.key.salt.as_deref().ok_or_else(|| {
        "Vault not initialized. Run: engram init".to_string()
    })?;
    let salt_bytes = B64
        .decode(salt_b64)
        .map_err(|_| "Invalid salt in config (not valid base64)".to_string())?;
    let salt: [u8; 16] = salt_bytes
        .try_into()
        .map_err(|_| "Salt in config must be exactly 16 bytes".to_string())?;

    // Tier 2: passphrase from env var
    if let Ok(passphrase) = std::env::var("ENGRAM_VAULT_PASSPHRASE") {
        return EngramKey::derive(passphrase.as_bytes(), &salt)
            .map_err(|e| format!("Key derivation failed: {e}"));
    }

    // Tier 3: interactive prompt
    let passphrase = rpassword::prompt_password("Vault passphrase: ")
        .map_err(|e| format!("Could not read passphrase: {e}"))?;
    if passphrase.is_empty() {
        return Err("Passphrase cannot be empty".to_string());
    }
    EngramKey::derive(passphrase.as_bytes(), &salt)
        .map_err(|e| format!("Key derivation failed: {e}"))
}
```

In the `#[cfg(test)] mod tests { ... }` block at the bottom of `main.rs`, add `use serial_test::serial;` at the top of that block (it's already in dev-dependencies).

### Step 5: Run tests to verify they pass

```
cargo test -p engram test_resolve_key 2>&1 | tail -15
```

Expected: all 4 tests PASS.

```
cargo build -p engram 2>&1 | tail -5
```

Expected: compiles clean.

### Step 6: Commit

```
git add crates/engram-cli/Cargo.toml crates/engram-cli/src/main.rs
git commit -m "feat(cli): three-tier vault key resolution — ENGRAM_VAULT_KEY → passphrase env var → interactive prompt; removes keychain dependency from key access"
```

---

## Task 4: Add `engram init` Command

**Files:**
- Modify: `crates/engram-cli/src/main.rs`

### Step 1: Write the failing integration test

Create `crates/engram-cli/tests/init_test.rs`:

```rust
// Integration tests for `engram init`.

use assert_cmd::Command;
use std::fs;
use tempfile::TempDir;

fn engram_with_config(config_path: &std::path::Path) -> Command {
    let mut cmd = Command::cargo_bin("engram").unwrap();
    cmd.env("ENGRAM_CONFIG_PATH", config_path)
        .env("ENGRAM_VAULT_PASSPHRASE", "test-init-passphrase")
        .env_remove("ENGRAM_VAULT_KEY");
    cmd
}

#[test]
fn test_init_creates_config_with_salt() {
    let dir = TempDir::new().unwrap();
    let config_path = dir.path().join("config.toml");

    engram_with_config(&config_path)
        .arg("init")
        .assert()
        .success();

    let contents = fs::read_to_string(&config_path).unwrap();
    assert!(contents.contains("salt"), "config must have salt after init:\n{contents}");
    assert!(
        !contents.contains("vault_key") && !contents.contains("password"),
        "config must NOT store the key:\n{contents}"
    );
}

#[test]
fn test_init_is_idempotent() {
    let dir = TempDir::new().unwrap();
    let config_path = dir.path().join("config.toml");

    // First init
    engram_with_config(&config_path).arg("init").assert().success();
    let salt1 = fs::read_to_string(&config_path).unwrap();

    // Second init — should report already initialized and not change the salt.
    let output = engram_with_config(&config_path)
        .arg("init")
        .output()
        .unwrap();
    assert!(output.status.success());
    let salt2 = fs::read_to_string(&config_path).unwrap();
    assert_eq!(salt1, salt2, "salt must not change on second init");
}

#[test]
fn test_init_sets_0600_permissions() {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let dir = TempDir::new().unwrap();
        let config_path = dir.path().join("config.toml");

        engram_with_config(&config_path).arg("init").assert().success();

        let mode = fs::metadata(&config_path).unwrap().permissions().mode() & 0o777;
        assert_eq!(mode, 0o600, "config.toml must be 0600 after init, got {:o}", mode);
    }
}
```

### Step 2: Run test to verify it fails

```
cargo test -p engram init_test 2>&1 | tail -15
```

Expected: FAIL — `engram init` is not a recognized subcommand.

### Step 3: Add `Init` to the CLI

In `crates/engram-cli/src/main.rs`:

**A. Add `Init` variant to the `Commands` enum** (after `Status`):
```rust
/// Initialise the vault: generate salt, prompt for passphrase, write config
Init,
```

**B. Add arm to the `match cli.command` block in `main()`** (after `Commands::Status`):
```rust
Commands::Init => run_init(),
```

**C. Add the `run_init()` function** (place it near `run_auth_add_s3`, e.g. just before it):

```rust
fn run_init() {
    let mut config = EngramConfig::load();

    if config.key.salt.is_some() {
        println!("Vault already initialized.");
        println!("Use ENGRAM_VAULT_PASSPHRASE env var or enter passphrase when prompted.");
        return;
    }

    // Resolve passphrase (env var or interactive prompt with confirmation).
    let passphrase = if let Ok(p) = std::env::var("ENGRAM_VAULT_PASSPHRASE") {
        println!("Using passphrase from ENGRAM_VAULT_PASSPHRASE.");
        p
    } else {
        let p = rpassword::prompt_password("Create vault passphrase: ")
            .expect("Failed to read passphrase");
        let confirm = rpassword::prompt_password("Confirm passphrase: ")
            .expect("Failed to read passphrase");
        if p != confirm {
            eprintln!("Error: passphrases do not match.");
            std::process::exit(1);
        }
        if p.is_empty() {
            eprintln!("Error: passphrase cannot be empty.");
            std::process::exit(1);
        }
        p
    };

    // Generate a random salt and verify key derivation succeeds before saving.
    let salt = engram_core::crypto::generate_salt();
    if let Err(e) = engram_core::crypto::EngramKey::derive(passphrase.as_bytes(), &salt) {
        eprintln!("Key derivation failed: {e}");
        std::process::exit(1);
    }

    config.key.salt = Some(B64.encode(salt));
    if let Err(e) = config.save() {
        eprintln!("Failed to save config: {e}");
        std::process::exit(1);
    }

    println!("Vault initialized.");
    println!("  Config: {}", EngramConfig::config_path().display());
    println!("  Key:    derived from passphrase (never stored)");
    println!();
    println!("Tip: set ENGRAM_VAULT_PASSPHRASE to avoid interactive prompts in scripts.");
}
```

### Step 4: Run tests to verify they pass

```
cargo test -p engram init_test 2>&1 | tail -15
```

Expected: all 3 tests PASS.

### Step 5: Commit

```
git add crates/engram-cli/src/main.rs crates/engram-cli/tests/init_test.rs
git commit -m "feat(cli): add 'engram init' command — generates Argon2id salt, accepts passphrase, never stores the key"
```

---

## Task 5: Rewrite `auth add` Commands to Write to `config.toml`

**Files:**
- Modify: `crates/engram-cli/src/main.rs`

This task replaces four functions (`run_auth_add_s3`, `run_auth_add_onedrive`, `run_auth_add_azure`, `run_auth_add_gdrive`) and also rewrites `run_auth_list` and `run_auth_remove` to work with config instead of the OS keychain.

It also adds an optional `--vault` flag to `BackendCommands::S3`, `Azure`, and `Gdrive` so credentials can be targeted at a specific vault name. (OneDrive already has `folder` for scoping.)

### Step 1: Write the failing integration test

Create `crates/engram-cli/tests/auth_config_test.rs`:

```rust
// Integration tests confirming auth add writes to config.toml, not the OS keychain.

use assert_cmd::Command;
use std::fs;
use tempfile::TempDir;

fn make_config(dir: &TempDir) -> std::path::PathBuf {
    let config_path = dir.path().join("config.toml");
    let vault_dir = dir.path().join("vault");
    fs::create_dir_all(&vault_dir).unwrap();
    fs::write(
        &config_path,
        format!(
            "[vaults.myvault]\npath = \"{}\"\ndefault = true\n",
            vault_dir.display()
        ),
    )
    .unwrap();
    config_path
}

fn engram(config_path: &std::path::Path) -> Command {
    let mut cmd = Command::cargo_bin("engram").unwrap();
    cmd.env("ENGRAM_CONFIG_PATH", config_path)
        .env_remove("ENGRAM_VAULT_KEY")
        .env_remove("ENGRAM_VAULT_PASSPHRASE");
    cmd
}

#[test]
fn test_auth_add_s3_writes_to_config() {
    let dir = TempDir::new().unwrap();
    let config_path = make_config(&dir);

    engram(&config_path)
        .args([
            "auth", "add", "s3",
            "--endpoint", "https://r2.example.com",
            "--bucket", "my-vault",
            "--access-key", "AKIA123",
            "--secret-key", "SECRET456",
        ])
        .assert()
        .success();

    let contents = fs::read_to_string(&config_path).unwrap();
    assert!(contents.contains("AKIA123"), "access_key missing from config");
    assert!(contents.contains("r2.example.com"), "endpoint missing from config");
    assert!(contents.contains("backend = \"s3\""), "backend field missing");
}

#[test]
fn test_auth_add_s3_sets_0600_permissions() {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let dir = TempDir::new().unwrap();
        let config_path = make_config(&dir);

        engram(&config_path)
            .args([
                "auth", "add", "s3",
                "--endpoint", "https://r2.example.com",
                "--bucket", "my-vault",
                "--access-key", "AKIA123",
                "--secret-key", "SECRET456",
            ])
            .assert()
            .success();

        let mode = fs::metadata(&config_path).unwrap().permissions().mode() & 0o777;
        assert_eq!(mode, 0o600, "config.toml must be 0600 after auth add");
    }
}

#[test]
fn test_auth_add_s3_fails_for_unknown_vault() {
    let dir = TempDir::new().unwrap();
    let config_path = dir.path().join("config.toml");
    fs::write(&config_path, "").unwrap(); // empty config — no vaults

    let output = engram(&config_path)
        .args([
            "auth", "add", "s3",
            "--vault", "nonexistent",
            "--endpoint", "https://r2.example.com",
            "--bucket", "my-vault",
            "--access-key", "AKIA123",
            "--secret-key", "SECRET456",
        ])
        .output()
        .unwrap();

    assert!(!output.status.success(), "should fail for unknown vault");
}
```

### Step 2: Run test to verify it fails

```
cargo test -p engram auth_config_test 2>&1 | tail -15
```

Expected: compile error or test failure — `auth add s3` currently writes to keychain.

### Step 3: Rewrite the auth functions in `main.rs`

**A. Add optional `--vault` arg to `BackendCommands::S3`, `BackendCommands::Azure`, and `BackendCommands::Gdrive`**:

```rust
S3 {
    /// Target vault name (defaults to the configured default vault)
    #[arg(long, default_value = "")]
    vault: String,
    #[arg(long)]
    endpoint: String,
    #[arg(long)]
    bucket: String,
    #[arg(long)]
    access_key: Option<String>,
    #[arg(long)]
    secret_key: Option<String>,
},
Azure {
    /// Target vault name (defaults to the configured default vault)
    #[arg(long, default_value = "")]
    vault: String,
    #[arg(long)]
    account: String,
    #[arg(long)]
    container: String,
},
Gdrive {
    /// Target vault name (defaults to the configured default vault)
    #[arg(long, default_value = "")]
    vault: String,
    #[arg(long)]
    bucket: String,
    #[arg(long)]
    key_file: String,
},
```

**B. Update the `main()` match arms** to pass `vault` through:

```rust
BackendCommands::S3 { vault, endpoint, bucket, access_key, secret_key } => {
    run_auth_add_s3(&vault, &endpoint, &bucket, access_key.as_deref(), secret_key.as_deref());
}
BackendCommands::Azure { vault, account, container } => {
    run_auth_add_azure(&vault, &account, &container);
}
BackendCommands::Gdrive { vault, bucket, key_file } => {
    run_auth_add_gdrive(&vault, &bucket, &key_file);
}
```

**C. Helper: resolve vault name for auth commands**

Add this small helper near the auth functions:

```rust
/// Resolve the vault name for `auth add` commands.
/// If `vault_arg` is empty, use the config default; exit 1 if none configured.
fn resolve_auth_vault_name(vault_arg: &str) -> String {
    if !vault_arg.is_empty() {
        return vault_arg.to_string();
    }
    let config = EngramConfig::load();
    config
        .default_vault()
        .map(|(n, _)| n.to_string())
        .unwrap_or_else(|| {
            eprintln!("No default vault configured. Run: engram vault add <name> --path <path> --default");
            std::process::exit(1);
        })
}
```

**D. Replace `run_auth_add_s3`**:

```rust
fn run_auth_add_s3(
    vault_arg: &str,
    endpoint: &str,
    bucket: &str,
    access_key: Option<&str>,
    secret_key: Option<&str>,
) {
    use std::io::{self, Write};

    let ak = access_key.map(|s| s.to_string()).unwrap_or_else(|| {
        print!("Access key ID: ");
        io::stdout().flush().unwrap();
        let mut input = String::new();
        io::stdin().read_line(&mut input).unwrap();
        input.trim().to_string()
    });
    let sk = secret_key
        .map(|s| s.to_string())
        .unwrap_or_else(|| rpassword::prompt_password("Secret access key: ").unwrap_or_default());

    let vault_name = resolve_auth_vault_name(vault_arg);
    let mut config = EngramConfig::load();

    let vault = match config.vaults.get_mut(&vault_name) {
        Some(v) => v,
        None => {
            eprintln!(
                "Vault '{}' not found. Run: engram vault add {} --path <path>",
                vault_name, vault_name
            );
            std::process::exit(1);
        }
    };

    vault.sync = Some(engram_core::config::SyncCredentials {
        backend: "s3".to_string(),
        endpoint: Some(endpoint.to_string()),
        bucket: Some(bucket.to_string()),
        access_key: Some(ak),
        secret_key: Some(sk),
        ..Default::default()
    });

    if let Err(e) = config.save() {
        eprintln!("Failed to save config: {e}");
        std::process::exit(1);
    }

    println!("S3 backend configured for vault '{}'.", vault_name);
    println!("  Endpoint: {endpoint}");
    println!("  Bucket:   {bucket}");
    println!("  Config:   {} (0600)", EngramConfig::config_path().display());
}
```

**E. Replace `run_auth_add_onedrive`** — keep the OAuth browser flow, replace `AuthStore::store` with config write:

```rust
fn run_auth_add_onedrive(folder: &str) {
    use std::io::{self, Write};

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
    let access_token = json["access_token"].as_str().expect("No access_token in response");
    let refresh_token = json["refresh_token"].as_str().unwrap_or("");

    // Write to the default vault's sync config.
    let vault_name = resolve_auth_vault_name("");
    let mut config = EngramConfig::load();
    let vault = match config.vaults.get_mut(&vault_name) {
        Some(v) => v,
        None => {
            eprintln!("Vault '{}' not found.", vault_name);
            std::process::exit(1);
        }
    };
    vault.sync = Some(engram_core::config::SyncCredentials {
        backend: "onedrive".to_string(),
        access_token: Some(access_token.to_string()),
        refresh_token: Some(refresh_token.to_string()),
        folder: Some(folder.to_string()),
        ..Default::default()
    });
    if let Err(e) = config.save() {
        eprintln!("Failed to save config: {e}");
        std::process::exit(1);
    }

    println!("OneDrive backend configured for vault '{}'.", vault_name);
    println!("  Folder: {folder}");
}
```

**F. Replace `run_auth_add_azure`**:

```rust
fn run_auth_add_azure(vault_arg: &str, account: &str, container: &str) {
    let ak = rpassword::prompt_password("Azure Storage access key: ").unwrap_or_default();

    let vault_name = resolve_auth_vault_name(vault_arg);
    let mut config = EngramConfig::load();
    let vault = match config.vaults.get_mut(&vault_name) {
        Some(v) => v,
        None => {
            eprintln!("Vault '{}' not found.", vault_name);
            std::process::exit(1);
        }
    };
    vault.sync = Some(engram_core::config::SyncCredentials {
        backend: "azure".to_string(),
        account: Some(account.to_string()),
        container: Some(container.to_string()),
        access_key: Some(ak),
        ..Default::default()
    });
    if let Err(e) = config.save() {
        eprintln!("Failed to save config: {e}");
        std::process::exit(1);
    }
    println!("Azure backend configured for vault '{}'.", vault_name);
    println!("  Account:   {account}");
    println!("  Container: {container}");
}
```

**G. Replace `run_auth_add_gdrive`**:

```rust
fn run_auth_add_gdrive(vault_arg: &str, bucket: &str, key_file: &str) {
    let vault_name = resolve_auth_vault_name(vault_arg);
    let mut config = EngramConfig::load();
    let vault = match config.vaults.get_mut(&vault_name) {
        Some(v) => v,
        None => {
            eprintln!("Vault '{}' not found.", vault_name);
            std::process::exit(1);
        }
    };
    vault.sync = Some(engram_core::config::SyncCredentials {
        backend: "gcs".to_string(),
        bucket: Some(bucket.to_string()),
        access_key: Some(key_file.to_string()), // reuse access_key for key_file path
        ..Default::default()
    });
    if let Err(e) = config.save() {
        eprintln!("Failed to save config: {e}");
        std::process::exit(1);
    }
    println!("GCS backend configured for vault '{}'.", vault_name);
    println!("  Bucket:   {bucket}");
    println!("  Key file: {key_file}");
}
```

**H. Replace `run_auth_list`** — iterate config vaults instead of keychain:

```rust
fn run_auth_list() {
    let config = EngramConfig::load();

    println!("{}", "─".repeat(41));
    println!("Configured sync backends:");
    println!();

    let mut any = false;
    for (name, entry) in &config.vaults {
        if let Some(sync) = &entry.sync {
            let details = match sync.backend.as_str() {
                "s3" => format!(
                    "endpoint={}, bucket={}",
                    sync.endpoint.as_deref().unwrap_or("?"),
                    sync.bucket.as_deref().unwrap_or("?")
                ),
                "onedrive" => format!("folder={}", sync.folder.as_deref().unwrap_or("?")),
                "azure" => format!(
                    "account={}, container={}",
                    sync.account.as_deref().unwrap_or("?"),
                    sync.container.as_deref().unwrap_or("?")
                ),
                "gcs" => format!("bucket={}", sync.bucket.as_deref().unwrap_or("?")),
                other => other.to_string(),
            };
            println!("  ✓ vault '{}': {} ({})", name, sync.backend, details);
            any = true;
        }
    }

    if !any {
        println!("  No backends configured.");
        println!();
        println!("  Run: engram auth add s3|onedrive|azure|gdrive");
    }
    println!();
}
```

**I. Replace `run_auth_remove`** — clear `vault.sync = None`:

```rust
fn run_auth_remove(vault_name: &str) {
    let mut config = EngramConfig::load();
    match config.vaults.get_mut(vault_name) {
        None => {
            eprintln!("Vault '{}' not found.", vault_name);
            std::process::exit(1);
        }
        Some(vault) => {
            if vault.sync.is_none() {
                println!("No sync backend configured for vault '{}'.", vault_name);
            } else {
                vault.sync = None;
                if let Err(e) = config.save() {
                    eprintln!("Failed to save config: {e}");
                    std::process::exit(1);
                }
                println!("Removed sync backend for vault '{}'.", vault_name);
            }
        }
    }
}
```

Also update the `AuthCommands::Remove` variant's field name in the `match` arm — it currently passes `backend` but should pass `vault_name`. Update the `Commands` match arm:

```rust
AuthCommands::Remove { backend } => run_auth_remove(&backend),
```

The field name `backend` is fine; it now refers to the vault name. Consider renaming the CLI arg to `vault` but that's optional — keeping `backend` preserves backward CLI compatibility.

### Step 4: Run tests to verify they pass

```
cargo test -p engram auth_config_test 2>&1 | tail -15
```

Expected: all 3 tests PASS.

```
cargo build -p engram 2>&1 | tail -5
```

Expected: clean build.

### Step 5: Commit

```
git add crates/engram-cli/src/main.rs crates/engram-cli/tests/auth_config_test.rs
git commit -m "feat(cli): auth add s3/onedrive/azure/gdrive writes credentials to config.toml (0600), not OS keychain — fixes headless panic"
```

---

## Task 6: Update `run_sync` to Read Credentials from `config.toml`

**Files:**
- Modify: `crates/engram-cli/src/main.rs`

### Step 1: Write the failing integration test

Create `crates/engram-cli/tests/sync_credentials_test.rs`:

```rust
// Verify run_sync reads backend credentials from config, not AuthStore.

use assert_cmd::Command;
use std::fs;
use tempfile::TempDir;

#[test]
fn test_sync_exits_cleanly_when_no_backend_configured() {
    let dir = TempDir::new().unwrap();
    let config_path = dir.path().join("config.toml");
    let vault_dir = dir.path().join("vault");
    fs::create_dir_all(&vault_dir).unwrap();
    fs::write(
        &config_path,
        format!(
            "[vaults.test]\npath = \"{}\"\ndefault = true\n",
            vault_dir.display()
        ),
    )
    .unwrap();

    // Sync with no backend configured: should print informative error, not panic.
    let output = Command::cargo_bin("engram")
        .unwrap()
        .env("ENGRAM_CONFIG_PATH", &config_path)
        .env("ENGRAM_VAULT_PASSPHRASE", "test-passphrase")
        .env_remove("ENGRAM_VAULT_KEY")
        .args(["sync", "--approve"])
        .output()
        .unwrap();

    // Must NOT panic (no keyring). Should exit non-zero with a clear message.
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        !stderr.contains("called `Result::unwrap()` on an `Err` value: Keyring"),
        "must not panic with keyring error:\n{stderr}"
    );
    assert!(
        stderr.contains("No sync backend") || stderr.contains("engram auth add") || stderr.contains("not initialized"),
        "should give a helpful error message:\n{stderr}"
    );
}
```

### Step 2: Run test to verify it fails

```
cargo test -p engram sync_credentials_test 2>&1 | tail -15
```

Expected: FAIL — `run_sync` currently calls `key_store.retrieve()` (keychain) and `AuthStore::retrieve().unwrap()` (keychain), both will panic in test context.

### Step 3: Rewrite `run_sync` credential and key sections

In `crates/engram-cli/src/main.rs`, find `run_sync` (starting around line 602). Make these targeted changes:

**A. Replace the `key_store.retrieve()` block** (lines ~644–652) with `resolve_vault_key()`:

```rust
let key = match resolve_vault_key() {
    Ok(k) => k,
    Err(e) => {
        eprintln!("Cannot access vault key: {e}");
        eprintln!("Tip: set ENGRAM_VAULT_PASSPHRASE or run: engram init");
        std::process::exit(1);
    }
};
```

**B. Replace the entire `effective_backend` / `backend: Box<dyn SyncBackend>` block** (lines ~654–690) with config-based credential lookup:

```rust
// Read credentials from config — no keychain.
let creds = config
    .get_vault(&vault_name)
    .and_then(|v| v.sync.as_ref());

let creds = match creds {
    Some(c) => c,
    None => {
        eprintln!(
            "No sync backend configured for vault '{}'.",
            vault_name
        );
        eprintln!("Run: engram auth add s3 --endpoint ... --bucket ...");
        std::process::exit(1);
    }
};

// The explicit backend_name arg overrides the configured backend name.
let effective_backend = backend_name.unwrap_or(creds.backend.as_str());

use engram_sync::{backend::SyncBackend, encrypt::encrypt_for_sync, onedrive::OneDriveBackend, s3::S3Backend};

let backend: Box<dyn SyncBackend> = match effective_backend {
    "s3" => {
        let endpoint = creds.endpoint.as_deref().unwrap_or("");
        let bucket   = creds.bucket.as_deref().unwrap_or("");
        let ak       = creds.access_key.as_deref().unwrap_or("");
        let sk       = creds.secret_key.as_deref().unwrap_or("");
        Box::new(S3Backend::new(endpoint, bucket, ak, sk).unwrap())
    }
    "onedrive" => {
        let token  = creds.access_token.as_deref().unwrap_or("");
        let folder = creds.folder.as_deref().unwrap_or("/Apps/Engram/vault");
        Box::new(OneDriveBackend::new(token, folder))
    }
    other => {
        eprintln!("Backend '{}' is not yet supported. Use: s3, onedrive", other);
        std::process::exit(1);
    }
};
```

Also remove the now-redundant `use engram_sync::auth::AuthStore` import from inside `run_sync` (it was inside the function body around line 639).

### Step 4: Run test to verify it passes

```
cargo test -p engram sync_credentials_test 2>&1 | tail -15
```

Expected: PASS.

```
cargo build -p engram 2>&1 | tail -5
```

Expected: clean build.

### Step 5: Commit

```
git add crates/engram-cli/src/main.rs crates/engram-cli/tests/sync_credentials_test.rs
git commit -m "feat(cli): run_sync reads backend credentials from config.toml, vault key from resolve_vault_key() — no more keychain .unwrap() panics in sync"
```

---

## Task 7: Replace All Remaining `key_store.retrieve()` Calls + Update `doctor` and `status`

**Files:**
- Modify: `crates/engram-cli/src/main.rs`

After Tasks 3–6, there are still five functions that call `KeyStore::new("engram").retrieve()`:
- `run_mcp` (line ~279)
- `run_load` (line ~1173)
- `run_observe` (line ~1248)
- `run_daemon` (line ~1318)
- `run_status` (line ~1808)

And `run_doctor` (line ~1449) calls `key_store.retrieve()` and uses the result to try opening the memory store.

### Step 1: No new test needed for this task

The e2e test in Task 8 covers the end-state. Run `cargo clippy` as the verification for this task.

### Step 2: Replace all five `key_store.retrieve()` blocks

For each function, replace the pattern:
```rust
let key_store = KeyStore::new("engram");
let key = match key_store.retrieve() {
    Ok(k) => k,
    Err(_) => {
        eprintln!("No vault key found. Run: engram init");
        std::process::exit(1);
    }
};
```

with:
```rust
let key = match resolve_vault_key() {
    Ok(k) => k,
    Err(e) => {
        eprintln!("Cannot access vault key: {e}");
        eprintln!("Tip: set ENGRAM_VAULT_PASSPHRASE or run: engram init");
        std::process::exit(1);
    }
};
```

Do this in: `run_mcp`, `run_load`, `run_observe`, `run_daemon`.

### Step 3: Update `run_status`

`run_status` calls `KeyStore::new("engram").retrieve()` to test if the key is accessible, then uses the result to try opening the memory store. Replace the key retrieval section with the new approach:

```rust
// ── Key / memory store status ─────────────────────────────────────────────
let key_result = resolve_vault_key();
let store_path = default_store_path_from_config(&config);

if store_path.exists() {
    match &key_result {
        Ok(key) => match MemoryStore::open(&store_path, key) {
            Ok(store) => {
                let count = store.record_count().unwrap_or(0);
                println!(
                    "Memory store: {} (present, {} records)",
                    store_path.display(),
                    count
                );
            }
            Err(_) => println!("Memory store: {} (wrong key)", store_path.display()),
        },
        Err(_) => println!("Memory store: {} (present, no key)", store_path.display()),
    }
} else {
    println!("Memory store: {} (not initialized)", store_path.display());
}

// ── Search index status ───────────────────────────────────────────────────
let vault_name = resolve_vault_name(None);
let search_dir = vault_storage_dir(&vault_name).join("search");
println!("{}", search_index_status(&search_dir));

// ── Key method ────────────────────────────────────────────────────────────
match &key_result {
    Ok(_) => println!("Key:          accessible ✓"),
    Err(e) => println!("Key:          not accessible — {e}"),
}
```

### Step 4: Update `run_doctor`

Replace the `KeyStore` retrieval block in `run_doctor` with a `resolve_vault_key()` call AND add a "key method" line showing which tier is active:

```rust
// ── Key status ──────────────────────────────────────────────────────────────
let key_method = if std::env::var("ENGRAM_VAULT_KEY").is_ok() {
    "ENGRAM_VAULT_KEY env var ✓".to_string()
} else if std::env::var("ENGRAM_VAULT_PASSPHRASE").is_ok() {
    "ENGRAM_VAULT_PASSPHRASE env var ✓".to_string()
} else {
    let config_now = EngramConfig::load();
    if config_now.key.salt.is_some() {
        "interactive passphrase (salt in config) ✓".to_string()
    } else {
        "not initialized ✗ — run: engram init".to_string()
    }
};
println!("Key:               {}", key_method);

let key_result = resolve_vault_key();

// ── Memory store status ─────────────────────────────────────────────────────
let store_path = default_store_path_from_config(&config);
if store_path.exists() {
    match &key_result {
        Ok(key) => match MemoryStore::open(&store_path, key) {
            Ok(store) => {
                let count = store.record_count().unwrap_or(0);
                println!("Store:             {} ({} records)", store_path.display(), count);
            }
            Err(_) => println!("Store:             {} (wrong key)", store_path.display()),
        },
        Err(_) => println!("Store:             {} (no key)", store_path.display()),
    }
} else {
    println!("Store:             {} (not initialized)", store_path.display());
}
```

Also remove the now-unused `use engram_core::{crypto::KeyStore, ...}` import at the top of `run_sync` (line ~637) and any other local `use engram_core::crypto::KeyStore` imports within function bodies.

At the top of the file, remove the `KeyStore` import from the module-level `use`:
```rust
// Change this line:
use engram_core::{crypto::KeyStore, store::MemoryStore, vault::Vault};
// To:
use engram_core::{store::MemoryStore, vault::Vault};
```

### Step 5: Build and check

```
cargo build -p engram 2>&1 | tail -10
```

Expected: clean build (no unused import warnings).

```
cargo clippy -p engram --all-targets -- -D warnings 2>&1 | tail -20
```

Expected: no warnings.

### Step 6: Commit

```
git add crates/engram-cli/src/main.rs
git commit -m "fix: replace all remaining KeyStore/keychain calls with resolve_vault_key(); update doctor and status to show key method tier"
```

---

## Task 8: End-to-End Tests — Full Flow Without Keychain

**Files:**
- Create: `crates/engram-cli/tests/e2e_no_keychain_test.rs`

This test confirms the exact scenario the user reported as broken — running `engram auth add s3` in a headless environment.

### Step 1: Write the tests

Create `crates/engram-cli/tests/e2e_no_keychain_test.rs`:

```rust
//! End-to-end tests confirming the complete init → vault add → auth add → status
//! flow works with ZERO OS keychain interaction.
//!
//! This is the exact scenario the user reported as broken:
//!   called `Result::unwrap()` on an `Err` value:
//!   Keyring("Platform secure storage failure: User interaction is not allowed.")

use assert_cmd::Command;
use std::fs;
use tempfile::TempDir;

/// Build a Command with ENGRAM_CONFIG_PATH and ENGRAM_VAULT_PASSPHRASE set,
/// and ENGRAM_VAULT_KEY explicitly unset (ensuring no pre-derived key shortcut).
fn engram(config_path: &std::path::Path) -> Command {
    let mut cmd = Command::cargo_bin("engram").unwrap();
    cmd.env("ENGRAM_CONFIG_PATH", config_path)
        .env("ENGRAM_VAULT_PASSPHRASE", "headless-test-passphrase")
        .env_remove("ENGRAM_VAULT_KEY");
    cmd
}

#[test]
fn test_init_headless() {
    let dir = TempDir::new().unwrap();
    let config_path = dir.path().join("config.toml");

    let output = engram(&config_path).arg("init").output().unwrap();
    assert!(
        output.status.success(),
        "engram init failed:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );

    let contents = fs::read_to_string(&config_path).unwrap();
    assert!(contents.contains("salt"), "config must contain salt after init");
    assert!(
        !contents.contains("vault_key") && !contents.contains("key_bytes"),
        "config must NOT store the derived key"
    );
}

#[test]
fn test_vault_add_headless() {
    let dir = TempDir::new().unwrap();
    let config_path = dir.path().join("config.toml");
    let vault_dir = dir.path().join("myvault");
    fs::create_dir_all(&vault_dir).unwrap();

    engram(&config_path).arg("init").assert().success();

    engram(&config_path)
        .args([
            "vault", "add", "myvault",
            "--path", &vault_dir.to_string_lossy(),
            "--sync", "approval",
            "--default",
        ])
        .assert()
        .success();

    let contents = fs::read_to_string(&config_path).unwrap();
    assert!(contents.contains("myvault"), "vault must be in config");
}

#[test]
fn test_auth_add_s3_headless_does_not_panic() {
    // This is the exact failing scenario from the bug report.
    let dir = TempDir::new().unwrap();
    let config_path = dir.path().join("config.toml");
    let vault_dir = dir.path().join("myvault");
    fs::create_dir_all(&vault_dir).unwrap();

    engram(&config_path).arg("init").assert().success();
    engram(&config_path)
        .args([
            "vault", "add", "myvault",
            "--path", &vault_dir.to_string_lossy(),
            "--default",
        ])
        .assert()
        .success();

    // This was the panic: "User interaction is not allowed."
    let output = engram(&config_path)
        .args([
            "auth", "add", "s3",
            "--endpoint", "https://r2.example.com",
            "--bucket", "test-bucket",
            "--access-key", "AKIA_TEST_KEY",
            "--secret-key", "SECRET_TEST_KEY",
        ])
        .output()
        .unwrap();

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        !stderr.contains("called `Result::unwrap()` on an `Err` value: Keyring"),
        "must not panic with keyring error:\n{stderr}"
    );
    assert!(
        output.status.success(),
        "engram auth add s3 must succeed without keychain:\n{stderr}"
    );

    // Verify credentials are in config.toml, not the keychain.
    let contents = fs::read_to_string(&config_path).unwrap();
    assert!(contents.contains("AKIA_TEST_KEY"), "access_key must be in config");
    assert!(contents.contains("r2.example.com"), "endpoint must be in config");
}

#[test]
fn test_status_headless() {
    let dir = TempDir::new().unwrap();
    let config_path = dir.path().join("config.toml");

    engram(&config_path).arg("init").assert().success();

    let output = engram(&config_path).arg("status").output().unwrap();
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        !stderr.contains("Keyring"),
        "status must not hit keychain:\n{stderr}"
    );
    // Status may succeed or give a "no vault" message, but must not panic.
    assert!(
        output.status.success(),
        "engram status must not crash:\n{stderr}"
    );
}

#[test]
fn test_doctor_shows_passphrase_method() {
    let dir = TempDir::new().unwrap();
    let config_path = dir.path().join("config.toml");

    engram(&config_path).arg("init").assert().success();

    let output = engram(&config_path).arg("doctor").output().unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("passphrase") || stdout.contains("ENGRAM_VAULT_PASSPHRASE"),
        "doctor should report passphrase key method, got:\n{stdout}"
    );
    assert!(
        !stdout.contains("not initialized"),
        "doctor should not say 'not initialized' after init:\n{stdout}"
    );
}
```

### Step 2: Run the tests

```
cargo test -p engram e2e_no_keychain 2>&1 | tail -20
```

Expected: all 5 tests PASS with zero keychain interaction.

### Step 3: Run the full test suite

```
cargo test --workspace 2>&1 | tail -20
```

Expected: all tests PASS (keychain-dependent tests in `crypto.rs` and `auth.rs` are marked `#[ignore]` and will not run).

### Step 4: Run clippy and fmt

```
cargo clippy --workspace --all-targets -- -D warnings 2>&1 | tail -10
cargo fmt --all -- --check 2>&1 | tail -10
```

Expected: both pass clean.

### Step 5: Final commit and push

```
git add crates/engram-cli/tests/e2e_no_keychain_test.rs
git commit -m "test: e2e coverage confirming init+auth+sync flow works in headless environments with no OS keychain interaction"

git pull --rebase origin main && git push origin main
```

---

## Summary of Changes

| File | Changes |
|------|---------|
| `crates/engram-core/src/config.rs` | Add `KeyConfig`, `SyncCredentials` structs; add `key: KeyConfig` to `EngramConfig`; add `sync: Option<SyncCredentials>` to `VaultEntry`; 0600 permissions in `save()` |
| `crates/engram-cli/Cargo.toml` | Add `base64 = { version = "0.22", features = ["std"] }` to `[dependencies]` |
| `crates/engram-cli/src/main.rs` | Add `Init` command; add `run_init()`; add `resolve_vault_key()` (three-tier); rewrite `run_auth_add_s3/onedrive/azure/gdrive`; rewrite `run_auth_list/remove`; update `run_sync`, `run_status`, `run_doctor`, `run_mcp`, `run_load`, `run_observe`, `run_daemon`; remove `KeyStore` import |
| `crates/engram-cli/tests/init_test.rs` | New: integration tests for `engram init` |
| `crates/engram-cli/tests/auth_config_test.rs` | New: integration tests for config-based credential storage |
| `crates/engram-cli/tests/sync_credentials_test.rs` | New: integration test for sync reading from config |
| `crates/engram-cli/tests/e2e_no_keychain_test.rs` | New: end-to-end headless flow tests |

`crates/engram-sync/src/auth.rs` — **not deleted**, but no longer called by the CLI primary flow. It remains for backward compatibility and any future migration tooling.
