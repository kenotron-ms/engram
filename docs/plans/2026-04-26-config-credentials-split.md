# Engram — Config / Credentials Split

> **Execution:** Use the subagent-driven-development workflow to implement this plan.

**Goal:** Split `~/.engram/config.toml` into two files: `config.toml` (non-sensitive, safe to sync — vault paths + key salt) and `credentials` (sensitive, 0600, never sync — S3 access keys + OAuth tokens). This is the AWS `~/.aws/config` + `~/.aws/credentials` pattern.

**Architecture:** Remove `sync: Option<SyncCredentials>` from `VaultEntry`. Add `CredentialsConfig` with its own save/load path (`~/.engram/credentials`, 0600). Auth commands write to credentials; sync commands read from credentials. `config.toml` contains only vault paths, access control, sync mode, and the key salt — all safe to copy to any device.

**Tech Stack:** `toml` (already present), `std::os::unix::fs::PermissionsExt` (unix only), `tempfile` (already in both `[dev-dependencies]`)

---

## Codebase orientation

Before starting, understand these sites so nothing surprises you mid-task:

**`crates/engram-core/src/config.rs`** — All types and methods. Key observations:
- `SyncCredentials` struct at line 71 (has all the credential fields)
- `VaultEntry` has `sync: Option<SyncCredentials>` at line 98 — this gets removed
- Existing tests use `sync: None` in every `VaultEntry { ... }` literal (lines 258, 313–319, 349–357)
- `make_entry()` helper at line 348 uses `sync: None` — needs the field removed
- `test_sync_credentials_roundtrip` at line 466 tests old `VaultEntry.sync` — must be replaced
- `test_sync_field_absent_when_none` at line 539 uses `sync: None` — must be updated
- `ENV_LOCK` pattern at line 231 serialises env-var-mutating tests — new tests must use it

**`crates/engram-cli/src/main.rs`** — The CLI. Key observations:
- Import at line 14: `use engram_core::config::{EngramConfig, SyncCredentials, ...}` — `SyncCredentials` import must change
- `run_auth_add_s3` at line 438: sets `vault.sync = Some(SyncCredentials {...})` then calls `config.save()`
- `run_auth_add_onedrive` at line 499: same pattern
- `run_auth_add_azure` at line 578: same pattern
- `run_auth_add_gdrive` at line 623: same pattern
- `run_auth_list` at line 653: reads `vault_entry.sync` to show configured backends
- `run_auth_remove` at line 706: sets `vault.sync = None` then `config.save()`
- `run_sync` at line 825: `config.get_vault(&vault_name).and_then(|v| v.sync.as_ref())`
- `run_vault_add` at line 1918: `sync: None` in VaultEntry struct literal — must be removed
- Error message at line 851 references `~/.engram/config.toml` — update to credentials file
- `run_doctor` at line 1618: add credentials file status line

**`crates/engram-cli/tests/auth_config_test.rs`** — Contains `test_auth_add_s3_writes_to_config` which currently ASSERTS that `AKID1234` is in `config.toml`. After this change, credentials must be in the **credentials file**, not config.toml. This test must be updated in Task 3.

**`crates/engram-cli/tests/sync_credentials_test.rs`** — Tests that `engram sync` exits cleanly when no backend is configured. Uses `ENGRAM_CONFIG_PATH`. After this change, also needs `ENGRAM_CREDENTIALS_PATH` to be set in tests that check the no-credentials path.

---

## New types (reference)

```rust
/// Credentials config — stored in ~/.engram/credentials (0600, never sync)
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct CredentialsConfig {
    #[serde(default)]
    pub vaults: BTreeMap<String, VaultSyncCredentials>,
}

/// Replaces SyncCredentials — same fields, separate storage
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct VaultSyncCredentials {
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

---

## Task 1: Add CredentialsConfig to config.rs; remove sync from VaultEntry

**Files:**
- Modify: `crates/engram-core/src/config.rs`

### Step 1: Write the failing tests

Add these three tests inside the `#[cfg(test)] mod tests { ... }` block in `config.rs`. Place them after the existing `// ── Task-1 tests` comment section (around line 451):

```rust
#[test]
fn test_credentials_config_default_is_empty() {
    let creds = CredentialsConfig::default();
    assert!(creds.vaults.is_empty());
}

#[test]
fn test_credentials_config_roundtrip() {
    let mut creds = CredentialsConfig::default();
    creds.vaults.insert("personal".to_string(), VaultSyncCredentials {
        backend: "s3".to_string(),
        endpoint: Some("https://r2.example.com".to_string()),
        bucket: Some("my-vault".to_string()),
        access_key: Some("AKIA123".to_string()),
        secret_key: Some("secret456".to_string()),
        ..Default::default()
    });
    let toml = toml::to_string_pretty(&creds).unwrap();
    let back: CredentialsConfig = toml::from_str(&toml).unwrap();
    assert_eq!(back.vaults["personal"].access_key.as_deref(), Some("AKIA123"));
}

#[test]
fn test_vault_entry_has_no_sync_field() {
    // VaultEntry should serialize without any sync section
    let entry = VaultEntry {
        path: PathBuf::from("~/.lifeos/memory"),
        access: VaultAccess::ReadWrite,
        sync_mode: SyncMode::Auto,
        default: true,
        vault_type: None,
    };
    let toml = toml::to_string_pretty(&entry).unwrap();
    assert!(!toml.contains("sync"), "VaultEntry must not contain sync credentials");
}
```

### Step 2: Run tests to verify they fail with compile errors

```
cargo test -p engram-core config 2>&1 | head -30
```

Expected: compile error — `CredentialsConfig` and `VaultSyncCredentials` not found, `VaultEntry` struct has unknown field `sync`.

### Step 3: Make the structural changes to config.rs

**3a. Add the two new structs** after the existing `SyncCredentials` struct (around line 83):

```rust
/// Credentials config — stored in ~/.engram/credentials (0600, never sync).
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct CredentialsConfig {
    /// Maps vault name → sync credentials.
    #[serde(default)]
    pub vaults: BTreeMap<String, VaultSyncCredentials>,
}

/// Per-vault sync credentials — replaces SyncCredentials, now lives outside VaultEntry.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct VaultSyncCredentials {
    /// Backend identifier (e.g. "s3", "azure", "gdrive", "onedrive").
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

**3b. Delete the `SyncCredentials` struct** (lines 70–83). It is entirely replaced by `VaultSyncCredentials` in `CredentialsConfig`. Do not leave it behind.

**3c. Remove the `sync` field from `VaultEntry`** — delete this line:

```rust
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub sync: Option<SyncCredentials>,
```

`VaultEntry` now has exactly five fields: `path`, `access`, `sync_mode`, `default`, `vault_type`.

### Step 4: Fix all existing tests that construct VaultEntry with sync: None

Search for `sync: None` in config.rs — there are five occurrences. Remove `sync: None,` from each `VaultEntry { ... }` literal. Sites:

- `vault_entry_type_field_is_renamed_in_toml` test (around line 258–268)
- `engram_config_roundtrip_toml` test (around line 313–319)
- `make_entry()` helper (around line 349–357)
- `test_sync_field_absent_when_none` test (around line 539–555)

For `test_sync_field_absent_when_none`: also update the assertion. The test checked that `[sync]` is absent when `sync: None`. Since the field no longer exists, change the assertion to verify `sync_mode` serialises without any separate `[sync]` credential table:

```rust
#[test]
fn test_vault_entry_has_no_sync_section_in_toml() {
    // VaultEntry has no sync credentials — confirm the TOML has no [sync] or sync.*  credential keys
    let entry = VaultEntry {
        path: PathBuf::from("/vaults/no-sync"),
        access: VaultAccess::ReadWrite,
        sync_mode: SyncMode::Approval,
        default: false,
        vault_type: None,
    };
    let toml_str = toml::to_string(&entry).expect("serialize VaultEntry");
    assert!(
        !toml_str.contains("[sync]"),
        "no [sync] table should appear — credentials live in the credentials file:\n{toml_str}"
    );
    // sync_mode is still present (not sensitive)
    assert!(toml_str.contains("sync_mode"), "sync_mode field must still be present");
}
```

### Step 5: Delete the old test_sync_credentials_roundtrip test

Delete the `test_sync_credentials_roundtrip` test at around line 466–495 entirely. It tested `vault.sync = Some(SyncCredentials {...})` which no longer exists. The new `test_credentials_config_roundtrip` test added in Step 1 covers the replacement.

### Step 6: Run tests to verify they pass

```
cargo test -p engram-core config
```

Expected: all tests pass. The three new tests pass. The updated/renamed `test_vault_entry_has_no_sync_section_in_toml` passes.

> **Note:** `cargo build -p engram` will currently fail because `main.rs` still imports `SyncCredentials` and accesses `vault.sync`. That is expected — it gets fixed in Task 3. Only run engram-core tests here.

### Step 7: Commit

```
git add crates/engram-core/src/config.rs
git commit -m "refactor(config): split SyncCredentials into separate CredentialsConfig — VaultEntry no longer holds credentials"
```

---

## Task 2: Add credentials_path, load_credentials, save_credentials methods

**Files:**
- Modify: `crates/engram-core/src/config.rs`

### Step 1: Write the failing tests

Add these four tests to the `#[cfg(test)] mod tests { ... }` block. They must use `env_lock()` (already defined at line 233) because they mutate `ENGRAM_CREDENTIALS_PATH` in the process environment:

```rust
// ── Task-2 (credentials file) tests ─────────────────────────────────────────

#[cfg(unix)]
#[test]
fn test_save_credentials_sets_0600() {
    use std::os::unix::fs::PermissionsExt;
    use std::env;
    use tempfile::tempdir;

    let _guard = env_lock();
    let dir = tempdir().expect("tempdir");
    let path = dir.path().join("credentials");
    env::set_var("ENGRAM_CREDENTIALS_PATH", &path);

    let mut creds = CredentialsConfig::default();
    creds.vaults.insert("test".to_string(), VaultSyncCredentials {
        backend: "s3".to_string(),
        ..Default::default()
    });
    EngramConfig::save_credentials(&creds).unwrap();

    let mode = std::fs::metadata(&path).unwrap().permissions().mode() & 0o777;
    assert_eq!(mode, 0o600, "credentials must be 0600, got {:o}", mode);
    env::remove_var("ENGRAM_CREDENTIALS_PATH");
}

#[test]
fn test_load_credentials_returns_default_when_missing() {
    use std::env;
    use tempfile::tempdir;

    let _guard = env_lock();
    let dir = tempdir().expect("tempdir");
    env::set_var("ENGRAM_CREDENTIALS_PATH", dir.path().join("nonexistent"));
    let creds = EngramConfig::load_credentials();
    assert!(creds.vaults.is_empty());
    env::remove_var("ENGRAM_CREDENTIALS_PATH");
}

#[test]
fn test_credentials_roundtrip_through_files() {
    use std::env;
    use tempfile::tempdir;

    let _guard = env_lock();
    let dir = tempdir().expect("tempdir");
    env::set_var("ENGRAM_CREDENTIALS_PATH", dir.path().join("credentials"));

    let mut creds = CredentialsConfig::default();
    creds.vaults.insert("personal".to_string(), VaultSyncCredentials {
        backend: "s3".to_string(),
        access_key: Some("AKIA_TEST".to_string()),
        secret_key: Some("secret_test".to_string()),
        endpoint: Some("https://r2.example.com".to_string()),
        bucket: Some("my-vault".to_string()),
        ..Default::default()
    });
    EngramConfig::save_credentials(&creds).unwrap();

    let loaded = EngramConfig::load_credentials();
    assert_eq!(loaded.vaults["personal"].access_key.as_deref(), Some("AKIA_TEST"));
    env::remove_var("ENGRAM_CREDENTIALS_PATH");
}

#[test]
fn test_config_toml_never_contains_credentials() {
    use std::env;
    use tempfile::tempdir;

    let _guard = env_lock();
    let dir = tempdir().expect("tempdir");
    env::set_var("ENGRAM_CONFIG_PATH", dir.path().join("config.toml"));
    env::set_var("ENGRAM_CREDENTIALS_PATH", dir.path().join("credentials"));

    // Save config with a vault — no credentials inside
    let mut config = EngramConfig::default();
    config.add_vault("personal".to_string(), VaultEntry {
        path: PathBuf::from("~/.lifeos/memory"),
        access: VaultAccess::ReadWrite,
        sync_mode: SyncMode::Auto,
        default: true,
        vault_type: None,
    });
    config.save().unwrap();

    // Save credentials separately
    let mut creds = CredentialsConfig::default();
    creds.vaults.insert("personal".to_string(), VaultSyncCredentials {
        backend: "s3".to_string(),
        access_key: Some("AKIA_SECRET".to_string()),
        ..Default::default()
    });
    EngramConfig::save_credentials(&creds).unwrap();

    // config.toml must NOT contain the access key
    let config_contents = std::fs::read_to_string(dir.path().join("config.toml")).unwrap();
    assert!(
        !config_contents.contains("AKIA_SECRET"),
        "config.toml must NEVER contain credentials:\n{}", config_contents
    );

    // credentials file must contain it
    let creds_contents = std::fs::read_to_string(dir.path().join("credentials")).unwrap();
    assert!(creds_contents.contains("AKIA_SECRET"));

    env::remove_var("ENGRAM_CONFIG_PATH");
    env::remove_var("ENGRAM_CREDENTIALS_PATH");
}
```

### Step 2: Run tests to verify they fail with compile errors

```
cargo test -p engram-core config 2>&1 | head -20
```

Expected: compile error — `EngramConfig::save_credentials`, `EngramConfig::load_credentials`, `EngramConfig::credentials_path` not found.

### Step 3: Add the four methods to the impl EngramConfig block

Add these methods to `impl EngramConfig { ... }` in `config.rs` (place them after the existing `save()` method, around line 166):

```rust
/// Return the path to the credentials file.
///
/// Checks `ENGRAM_CREDENTIALS_PATH` first; falls back to `~/.engram/credentials`.
pub fn credentials_path() -> PathBuf {
    if let Ok(p) = std::env::var("ENGRAM_CREDENTIALS_PATH") {
        return PathBuf::from(p);
    }
    dirs::home_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join(".engram")
        .join("credentials")
}

/// Load credentials from disk.
///
/// Returns `Default::default()` if the file is missing or cannot be parsed.
pub fn load_credentials() -> CredentialsConfig {
    let path = Self::credentials_path();
    if !path.exists() {
        return CredentialsConfig::default();
    }
    let content = std::fs::read_to_string(&path).unwrap_or_default();
    toml::from_str(&content).unwrap_or_default()
}

/// Persist credentials to disk using an atomic tmp-file + rename.
///
/// Sets 0600 permissions on Unix. Creates parent directories if needed.
pub fn save_credentials(creds: &CredentialsConfig) -> Result<(), ConfigError> {
    let path = Self::credentials_path();
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let content = toml::to_string_pretty(creds)?;
    let tmp = path.with_extension("tmp");
    std::fs::write(&tmp, &content)?;
    std::fs::rename(&tmp, &path)?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o600))
            .map_err(ConfigError::Io)?;
    }
    Ok(())
}

/// Look up credentials for a specific vault by name.
pub fn credentials_for_vault<'a>(
    name: &str,
    creds: &'a CredentialsConfig,
) -> Option<&'a VaultSyncCredentials> {
    creds.vaults.get(name)
}
```

### Step 4: Run tests to verify they pass

```
cargo test -p engram-core config
```

Expected: all tests pass.

### Step 5: Commit

```
git add crates/engram-core/src/config.rs
git commit -m "feat(config): add credentials_path/load_credentials/save_credentials — separate file with 0600 permissions"
```

---

## Task 3: Update all auth commands and vault add to write to credentials file

**Files:**
- Modify: `crates/engram-cli/src/main.rs`
- Modify: `crates/engram-cli/tests/auth_config_test.rs`

### Step 1: Update the import line in main.rs

Find line 14:
```rust
use engram_core::config::{EngramConfig, SyncCredentials, SyncMode, VaultAccess, VaultEntry};
```

Replace with:
```rust
use engram_core::config::{
    CredentialsConfig, EngramConfig, SyncMode, VaultAccess, VaultEntry, VaultSyncCredentials,
};
```

### Step 2: Update run_vault_add — remove sync: None from VaultEntry

In `run_vault_add` (around line 1912–1919), find this `VaultEntry` struct literal:

```rust
    let entry = VaultEntry {
        path: expanded_path,
        access: access_mode,
        sync_mode: sync,
        default,
        vault_type: vault_type.map(|s| s.to_string()),
        sync: None,
    };
```

Remove the `sync: None,` line. `VaultEntry` no longer has that field.

### Step 3: Rewrite run_auth_add_s3

Replace the entire `run_auth_add_s3` function body with this implementation. Keep the function signature identical:

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
        .unwrap_or_else(|| match rpassword::prompt_password("Secret access key: ") {
            Ok(s) if !s.is_empty() => s,
            Ok(_) => {
                eprintln!("Secret key must not be empty.");
                std::process::exit(1);
            }
            Err(e) => {
                eprintln!("Failed to read secret key: {}", e);
                std::process::exit(1);
            }
        });

    let vault_name = resolve_auth_vault_name(vault_arg);
    let config = EngramConfig::load();

    // Verify the vault exists in config before writing credentials.
    if !config.vaults.contains_key(&vault_name) {
        eprintln!("Vault '{}' not found in config.", vault_name);
        std::process::exit(1);
    }

    // Load credentials, upsert, save — separate from config.toml.
    let mut creds = EngramConfig::load_credentials();
    creds.vaults.insert(vault_name.clone(), VaultSyncCredentials {
        backend: "s3".to_string(),
        endpoint: Some(endpoint.to_string()),
        bucket: Some(bucket.to_string()),
        access_key: Some(ak),
        secret_key: Some(sk),
        ..Default::default()
    });
    if let Err(e) = EngramConfig::save_credentials(&creds) {
        eprintln!("Failed to save credentials: {}", e);
        std::process::exit(1);
    }

    println!("\u{2713} S3 backend configured for vault '{}'", vault_name);
    println!("  Endpoint:    {}", endpoint);
    println!("  Bucket:      {}", bucket);
    println!("  Credentials: {} (0600, do not sync)", EngramConfig::credentials_path().display());
}
```

### Step 4: Rewrite run_auth_add_onedrive

The OneDrive OAuth token exchange stays unchanged. Only replace the config-write block at the end (currently `config.vaults.get_mut(...) { vault.sync = Some(...) }` + `config.save()`).

Replace that block with:

```rust
    // Verify vault exists.
    if !config.vaults.contains_key(&vault_name) {
        eprintln!("Vault '{}' not found in config.", vault_name);
        std::process::exit(1);
    }

    // Write tokens to credentials file — never to config.toml.
    let mut creds = EngramConfig::load_credentials();
    creds.vaults.insert(vault_name.clone(), VaultSyncCredentials {
        backend: "onedrive".to_string(),
        access_token: Some(access_token),
        refresh_token: Some(refresh_token),
        folder: Some(folder.to_string()),
        ..Default::default()
    });
    if let Err(e) = EngramConfig::save_credentials(&creds) {
        eprintln!("Failed to save credentials: {}", e);
        std::process::exit(1);
    }

    println!("\u{2713} OneDrive backend configured for vault '{}'", vault_name);
    println!("  Folder:      {}", folder);
    println!("  Credentials: {} (0600, do not sync)", EngramConfig::credentials_path().display());
```

Also add `let config = EngramConfig::load();` before `let vault_name = resolve_auth_vault_name("");` if not already present in that function (currently it's the line after vault_name).

### Step 5: Rewrite run_auth_add_azure

Apply the same pattern — replace the `config.vaults.get_mut(...) { vault.sync = Some(...) }` + `config.save()` block:

```rust
    let vault_name = resolve_auth_vault_name(vault_arg);
    let config = EngramConfig::load();

    if !config.vaults.contains_key(&vault_name) {
        eprintln!("Vault '{}' not found in config.", vault_name);
        std::process::exit(1);
    }

    let mut creds = EngramConfig::load_credentials();
    creds.vaults.insert(vault_name.clone(), VaultSyncCredentials {
        backend: "azure".to_string(),
        account: Some(account.to_string()),
        container: Some(container.to_string()),
        access_key: Some(ak),
        ..Default::default()
    });
    if let Err(e) = EngramConfig::save_credentials(&creds) {
        eprintln!("Failed to save credentials: {}", e);
        std::process::exit(1);
    }

    println!("\u{2713} Azure backend configured for vault '{}'", vault_name);
    println!("  Account:     {}", account);
    println!("  Container:   {}", container);
    println!("  Credentials: {} (0600, do not sync)", EngramConfig::credentials_path().display());
```

### Step 6: Rewrite run_auth_add_gdrive

Same pattern:

```rust
    let vault_name = resolve_auth_vault_name(vault_arg);
    let config = EngramConfig::load();

    if !config.vaults.contains_key(&vault_name) {
        eprintln!("Vault '{}' not found in config.", vault_name);
        std::process::exit(1);
    }

    let mut creds = EngramConfig::load_credentials();
    creds.vaults.insert(vault_name.clone(), VaultSyncCredentials {
        backend: "gcs".to_string(),
        bucket: Some(bucket.to_string()),
        // Reuse access_key field for the key file path.
        access_key: Some(key_file.to_string()),
        ..Default::default()
    });
    if let Err(e) = EngramConfig::save_credentials(&creds) {
        eprintln!("Failed to save credentials: {}", e);
        std::process::exit(1);
    }

    println!("\u{2713} GCS backend configured for vault '{}'", vault_name);
    println!("  Bucket:      {}", bucket);
    println!("  Key file:    {}", key_file);
    println!("  Credentials: {} (0600, do not sync)", EngramConfig::credentials_path().display());
```

### Step 7: Rewrite run_auth_list

`run_auth_list` currently iterates `config.vaults` and reads `vault_entry.sync`. Replace the display loop with a version that reads from `EngramConfig::load_credentials()`:

```rust
fn run_auth_list() {
    let config = EngramConfig::load();
    let creds_config = EngramConfig::load_credentials();

    println!("{}", "\u{2500}".repeat(41));
    println!("Vault sync backends:");
    println!("  (credentials: {})", EngramConfig::credentials_path().display());
    println!();

    if config.vaults.is_empty() {
        println!("  No vaults configured.");
        println!();
        println!("  Run: engram vault add <name> --path <path>");
        println!();
        return;
    }

    let mut any_configured = false;
    for vault_name in config.vaults.keys() {
        if let Some(creds) = EngramConfig::credentials_for_vault(vault_name, &creds_config) {
            let details = match creds.backend.as_str() {
                "s3" => {
                    let endpoint = creds.endpoint.as_deref().unwrap_or("(none)");
                    let bucket = creds.bucket.as_deref().unwrap_or("(none)");
                    format!("endpoint={}, bucket={}", endpoint, bucket)
                }
                "onedrive" => {
                    let folder = creds.folder.as_deref().unwrap_or("(none)");
                    format!("folder={}", folder)
                }
                "azure" => {
                    let account = creds.account.as_deref().unwrap_or("(none)");
                    let container = creds.container.as_deref().unwrap_or("(none)");
                    format!("account={}, container={}", account, container)
                }
                "gcs" => {
                    let bucket = creds.bucket.as_deref().unwrap_or("(none)");
                    format!("bucket={}", bucket)
                }
                other => format!("backend={}", other),
            };
            println!("  \u{2713} {} \u{2014} {} ({})", vault_name, creds.backend, details);
            any_configured = true;
        } else {
            println!("  \u{00b7} {} \u{2014} no sync configured", vault_name);
        }
    }

    if !any_configured {
        println!();
        println!("  Run: engram auth add s3|onedrive|azure|gdrive --vault <name>");
    }
    println!();
}
```

### Step 8: Rewrite run_auth_remove

Replace the current implementation (which sets `vault.sync = None` and saves config):

```rust
fn run_auth_remove(vault_name: &str) {
    let mut creds = EngramConfig::load_credentials();
    if creds.vaults.remove(vault_name).is_some() {
        if let Err(e) = EngramConfig::save_credentials(&creds) {
            eprintln!("Failed to save credentials: {}", e);
            std::process::exit(1);
        }
        println!("\u{2713} Removed sync credentials for vault '{}'", vault_name);
    } else {
        println!("No sync credentials configured for vault '{}'", vault_name);
    }
}
```

### Step 9: Update auth_config_test.rs

The existing `test_auth_add_s3_writes_to_config` test currently ASSERTS that credentials appear in `config.toml`. That was the old (wrong) behaviour. Update it to assert the opposite — credentials are in the **credentials file**, and config.toml is clean.

Open `crates/engram-cli/tests/auth_config_test.rs` and make these changes:

**Update `write_config_with_vault` helper** — no change needed, still valid.

**Add `ENGRAM_CREDENTIALS_PATH` env var to every `Command`** in this file. Add a credentials path alongside the config path in each test so the binary writes to a controlled location:

```rust
// At the top of tests in auth_config_test.rs, update each command to also set
// ENGRAM_CREDENTIALS_PATH:
cmd.env("ENGRAM_CONFIG_PATH", &config_path)
   .env("ENGRAM_CREDENTIALS_PATH", dir.path().join("credentials").to_string_lossy().to_string())
   ...
```

**Replace `test_auth_add_s3_writes_to_config`** entirely:

```rust
/// `engram auth add s3` must write credentials to the credentials file, NOT config.toml.
/// config.toml must be credential-free after the command.
#[test]
fn test_auth_add_s3_writes_to_credentials_not_config() {
    let dir = TempDir::new().unwrap();
    let vault_path = dir.path().join("vault");
    fs::create_dir_all(&vault_path).unwrap();
    let config_path = write_config_with_vault(dir.path(), "myvault", &vault_path.to_string_lossy());
    let creds_path = dir.path().join("credentials");

    let mut cmd = Command::cargo_bin("engram").unwrap();
    cmd.args([
        "auth", "add", "s3",
        "--vault", "myvault",
        "--endpoint", "https://s3.example.com",
        "--bucket", "test-bucket",
        "--access-key", "AKID1234",
        "--secret-key", "secretkey5678",
    ])
    .env("ENGRAM_CONFIG_PATH", &config_path)
    .env("ENGRAM_CREDENTIALS_PATH", &creds_path)
    .env_remove("ENGRAM_VAULT_KEY")
    .env_remove("ENGRAM_VAULT_PASSPHRASE");

    cmd.assert().success();

    // config.toml must NOT contain credentials.
    let config_contents = fs::read_to_string(&config_path).expect("read config.toml");
    assert!(
        !config_contents.contains("AKID1234"),
        "config.toml must NOT contain credentials (it's syncable!):\n{config_contents}"
    );
    assert!(
        !config_contents.contains("backend"),
        "config.toml must NOT contain backend field:\n{config_contents}"
    );

    // Credentials file must contain the credentials.
    let creds_contents = fs::read_to_string(&creds_path)
        .expect("credentials file should have been created");
    assert!(
        creds_contents.contains("AKID1234"),
        "credentials file must contain access_key:\n{creds_contents}"
    );
    assert!(
        creds_contents.contains("s3.example.com"),
        "credentials file must contain endpoint:\n{creds_contents}"
    );
    assert!(
        creds_contents.contains("test-bucket"),
        "credentials file must contain bucket:\n{creds_contents}"
    );
}
```

**Update `test_auth_add_s3_sets_0600_permissions`** — the 0600 check now applies to the **credentials file** (not config.toml). Update accordingly:

```rust
#[test]
#[cfg(unix)]
fn test_auth_add_s3_credentials_file_is_0600() {
    use std::os::unix::fs::PermissionsExt;

    let dir = TempDir::new().unwrap();
    let vault_path = dir.path().join("vault");
    fs::create_dir_all(&vault_path).unwrap();
    let config_path = write_config_with_vault(dir.path(), "myvault", &vault_path.to_string_lossy());
    let creds_path = dir.path().join("credentials");

    let mut cmd = Command::cargo_bin("engram").unwrap();
    cmd.args([
        "auth", "add", "s3",
        "--vault", "myvault",
        "--endpoint", "https://s3.example.com",
        "--bucket", "test-bucket",
        "--access-key", "AKID1234",
        "--secret-key", "secretkey5678",
    ])
    .env("ENGRAM_CONFIG_PATH", &config_path)
    .env("ENGRAM_CREDENTIALS_PATH", &creds_path)
    .env_remove("ENGRAM_VAULT_KEY")
    .env_remove("ENGRAM_VAULT_PASSPHRASE");

    cmd.assert().success();

    let meta = fs::metadata(&creds_path).expect("credentials file should exist");
    let mode = meta.permissions().mode() & 0o777;
    assert_eq!(mode, 0o600, "credentials must be 0600, got {mode:04o}");
}
```

**Update `test_auth_add_s3_fails_for_unknown_vault`** — add `ENGRAM_CREDENTIALS_PATH` env var (no content change to the assertion):

```rust
cmd.env("ENGRAM_CONFIG_PATH", &config_path)
   .env("ENGRAM_CREDENTIALS_PATH", dir.path().join("credentials").to_string_lossy().to_string())
   ...
```

### Step 10: Verify the workspace builds clean

```
cargo build -p engram
```

Expected: clean build, no more references to `SyncCredentials` or `vault.sync` anywhere.

### Step 11: Commit

```
git add crates/engram-cli/src/main.rs crates/engram-cli/tests/auth_config_test.rs
git commit -m "feat(cli): auth commands write to ~/.engram/credentials (0600) — config.toml stays credential-free"
```

---

## Task 4: Update run_sync to read from credentials file

**Files:**
- Modify: `crates/engram-cli/src/main.rs`
- Modify: `crates/engram-cli/tests/sync_credentials_test.rs`

### Step 1: Locate the credentials lookup in run_sync

In `run_sync`, find this block (around line 824–836):

```rust
    // Determine which backend to use from config credentials.
    let creds = config.get_vault(&vault_name).and_then(|v| v.sync.as_ref());

    let creds = match creds {
        Some(c) => c,
        None => {
            eprintln!(
                "No sync backend configured for vault '{}'. Run: engram auth add s3|onedrive|azure|gdrive --vault {}",
                vault_name, vault_name
            );
            std::process::exit(1);
        }
    };
```

### Step 2: Replace it with credentials-file lookup

```rust
    // Read credentials from the separate credentials file — not config.toml.
    let creds_config = EngramConfig::load_credentials();
    let creds = EngramConfig::credentials_for_vault(&vault_name, &creds_config);

    let creds = match creds {
        Some(c) => c,
        None => {
            eprintln!(
                "No sync credentials configured for vault '{}'.",
                vault_name
            );
            eprintln!(
                "Run: engram auth add s3 --vault {} --endpoint <url> --bucket <name>",
                vault_name
            );
            eprintln!(
                "Credentials are stored in: {}",
                EngramConfig::credentials_path().display()
            );
            std::process::exit(1);
        }
    };
```

### Step 3: Fix the stale error message inside the s3 backend match arm

Around line 851, update this error message to point at the credentials file:

```rust
                Err(e) => {
                    eprintln!("Failed to initialize S3 backend: {}", e);
                    eprintln!(
                        "Check the endpoint URL and credentials in: {}",
                        EngramConfig::credentials_path().display()
                    );
                    std::process::exit(1);
                }
```

### Step 4: Update the effective_backend line

The next line uses `creds.backend.as_str()`. Since `creds` is now `&VaultSyncCredentials` instead of `&SyncCredentials`, the field names are identical — no change needed here.

### Step 5: Update sync_credentials_test.rs

`sync_credentials_test.rs` currently passes `ENGRAM_CONFIG_PATH` but not `ENGRAM_CREDENTIALS_PATH`. The `test_sync_exits_cleanly_when_no_backend_configured` test checks that a vault with no sync block exits with a helpful error — the behaviour is unchanged, but add the env var so the test doesn't accidentally read real credentials from `~/.engram/credentials`:

```rust
    let mut cmd = Command::cargo_bin("engram").unwrap();
    cmd.args(["sync", "--vault", "myvault"])
        .env("ENGRAM_CONFIG_PATH", &config_path)
        .env("ENGRAM_CREDENTIALS_PATH", dir.path().join("credentials").to_string_lossy().to_string())
        .env("ENGRAM_VAULT_KEY", DUMMY_VAULT_KEY)
        .env_remove("ENGRAM_VAULT_PASSPHRASE");
```

### Step 6: Build and run all tests

```
cargo build -p engram
cargo test --workspace
```

Expected: clean build and all tests pass.

### Step 7: Commit

```
git add crates/engram-cli/src/main.rs crates/engram-cli/tests/sync_credentials_test.rs
git commit -m "feat(cli): run_sync reads credentials from ~/.engram/credentials instead of config.toml"
```

---

## Task 5: Update engram doctor to show both files

**Files:**
- Modify: `crates/engram-cli/src/main.rs`

### Step 1: Locate the run_doctor function

Find `run_doctor()` starting around line 1618. It currently shows vault, key method, memory store, and `ANTHROPIC_API_KEY`. It does not show config or credentials file paths.

### Step 2: Add config and credentials file lines

Add these two `println!` calls directly after the `println!("Binary: ...")` line (around line 1632), before the vault status block:

```rust
    // ── Config / credentials files ────────────────────────────────────────────
    let config_path = EngramConfig::config_path();
    println!(
        "Config:            {} ({})",
        config_path.display(),
        if config_path.exists() {
            "present \u{2713} — safe to sync"
        } else {
            "missing \u{2717}"
        }
    );

    let creds_path = EngramConfig::credentials_path();
    println!(
        "Credentials:       {} ({})",
        creds_path.display(),
        if creds_path.exists() {
            "present \u{2713} — do NOT sync"
        } else {
            "not configured"
        }
    );
```

### Step 3: Smoke-test the output

```
cargo run --bin engram -- doctor
```

Expected output includes (exact paths will vary):
```
Config:            /Users/<you>/.engram/config.toml (present ✓ — safe to sync)
Credentials:       /Users/<you>/.engram/credentials (not configured)
```

### Step 4: Commit

```
git add crates/engram-cli/src/main.rs
git commit -m "feat(cli): doctor shows config.toml (syncable) and credentials (never sync) separately"
```

---

## Task 6: End-to-end test — credentials never appear in config.toml

**Files:**
- Create: `crates/engram-cli/tests/credentials_split_test.rs`

### Step 1: Create the test file

```rust
// End-to-end tests validating that credentials are stored separately from config.toml.
//
// These tests run the compiled binary and check:
// - `engram auth add s3` writes to the credentials file, never to config.toml
// - config.toml is safe to sync (contains only salt, paths, access modes)
// - credentials file is 0600 (never sync)
// - `engram auth remove` clears from credentials file, not config.toml

use assert_cmd::Command;
use std::fs;
use tempfile::TempDir;

// ── helper ─────────────────────────────────────────────────────────────────

/// Returns a Command pre-configured with isolated config and credentials paths.
/// All tests must use this to avoid reading from or writing to ~/.engram/*.
fn engram_with_dirs(dir: &TempDir) -> Command {
    let mut cmd = Command::cargo_bin("engram").unwrap();
    cmd.env("ENGRAM_CONFIG_PATH", dir.path().join("config.toml"))
       .env("ENGRAM_CREDENTIALS_PATH", dir.path().join("credentials"))
       .env("ENGRAM_VAULT_PASSPHRASE", "test-passphrase");
    cmd
}

// ── tests ──────────────────────────────────────────────────────────────────

#[test]
fn test_auth_add_s3_writes_to_credentials_not_config() {
    let dir = TempDir::new().unwrap();
    let vault_dir = TempDir::new().unwrap();

    engram_with_dirs(&dir).arg("init").assert().success();
    engram_with_dirs(&dir)
        .args([
            "vault", "add", "personal",
            "--path", &vault_dir.path().to_string_lossy(),
            "--default",
        ])
        .assert().success();

    engram_with_dirs(&dir)
        .args([
            "auth", "add", "s3",
            "--endpoint", "https://r2.example.com",
            "--bucket", "my-vault",
            "--access-key", "AKIA_TEST_123",
            "--secret-key", "secret_test_456",
        ])
        .assert().success();

    // config.toml must NOT contain credentials.
    let config = fs::read_to_string(dir.path().join("config.toml")).unwrap();
    assert!(
        !config.contains("AKIA_TEST_123"),
        "config.toml must not contain credentials (it's syncable!):\n{}", config
    );
    assert!(
        !config.contains("secret_test"),
        "config.toml must not contain secret key:\n{}", config
    );

    // credentials file must contain them.
    let creds = fs::read_to_string(dir.path().join("credentials")).unwrap();
    assert!(creds.contains("AKIA_TEST_123"), "credentials must have access key:\n{}", creds);
    assert!(creds.contains("r2.example.com"), "credentials must have endpoint:\n{}", creds);

    // credentials file must be 0600.
    #[cfg(unix)] {
        use std::os::unix::fs::PermissionsExt;
        let mode = fs::metadata(dir.path().join("credentials"))
            .unwrap().permissions().mode() & 0o777;
        assert_eq!(mode, 0o600, "credentials file must be 0600 (user-readable only)");
    }
}

#[test]
fn test_config_toml_is_safe_to_sync() {
    let dir = TempDir::new().unwrap();
    let vault_dir = TempDir::new().unwrap();

    engram_with_dirs(&dir).arg("init").assert().success();
    engram_with_dirs(&dir)
        .args([
            "vault", "add", "personal",
            "--path", &vault_dir.path().to_string_lossy(),
            "--default",
        ])
        .assert().success();

    let config = fs::read_to_string(dir.path().join("config.toml")).unwrap();

    // Must have salt (needed on all devices for key derivation).
    assert!(config.contains("salt"), "config.toml must have salt:\n{}", config);

    // Must have vault path (needed on all devices).
    assert!(config.contains("path"), "config.toml must have vault path:\n{}", config);

    // Must NOT have any sensitive values.
    assert!(!config.contains("access_key"), "config.toml must not have access_key:\n{}", config);
    assert!(!config.contains("secret_key"), "config.toml must not have secret_key:\n{}", config);
    assert!(!config.contains("access_token"), "config.toml must not have OAuth tokens:\n{}", config);
    assert!(!config.contains("backend"), "config.toml must not have backend field:\n{}", config);
}

#[test]
fn test_auth_remove_clears_from_credentials_file() {
    let dir = TempDir::new().unwrap();
    let vault_dir = TempDir::new().unwrap();

    engram_with_dirs(&dir).arg("init").assert().success();
    engram_with_dirs(&dir)
        .args([
            "vault", "add", "personal",
            "--path", &vault_dir.path().to_string_lossy(),
            "--default",
        ])
        .assert().success();

    engram_with_dirs(&dir)
        .args([
            "auth", "add", "s3",
            "--endpoint", "https://r2.example.com",
            "--bucket", "b",
            "--access-key", "K",
            "--secret-key", "S",
        ])
        .assert().success();

    // Verify credentials are written.
    let creds_before = fs::read_to_string(dir.path().join("credentials")).unwrap();
    assert!(creds_before.contains("access_key"));

    // Remove credentials.
    engram_with_dirs(&dir)
        .args(["auth", "remove", "personal"])
        .assert().success();

    // Either the file is gone or the access_key is absent — both are acceptable.
    let creds_path = dir.path().join("credentials");
    if creds_path.exists() {
        let creds_after = fs::read_to_string(&creds_path).unwrap();
        assert!(
            !creds_after.contains("access_key"),
            "credentials file should not contain access_key after remove:\n{}", creds_after
        );
    }
    // config.toml must be unchanged (no credential fields).
    let config = fs::read_to_string(dir.path().join("config.toml")).unwrap();
    assert!(!config.contains("access_key"), "config.toml must never have credentials:\n{}", config);
}
```

### Step 2: Run the new tests

```
cargo test -p engram credentials_split_test -- --nocapture
```

Expected: all three tests pass.

### Step 3: Run the full workspace

```
cargo test --workspace && cargo clippy --workspace --all-targets -- -D warnings
```

Expected: clean. No warnings promoted to errors.

### Step 4: Commit and push

```
git add crates/engram-cli/tests/credentials_split_test.rs
git commit -m "test: e2e validation that credentials never appear in syncable config.toml"
git pull --rebase origin main
git push origin main
```

---

## Final file layout

After all six tasks:

```
~/.engram/
  config.toml       (0600, safe to sync — salt, vault paths, access modes, sync modes)
  credentials       (0600, NEVER sync — S3 keys, OAuth tokens)
```

`config.toml` example:
```toml
# engram config — safe to sync across devices
[key]
salt = "aGVsbG93b3JsZA=="

[vaults.personal]
path = "~/.lifeos/memory"
access = "read-write"
sync_mode = "auto"
default = true
```

`credentials` example:
```toml
# engram credentials — DO NOT sync or commit
[vaults.personal]
backend = "s3"
endpoint = "https://accountid.r2.cloudflarestorage.com"
bucket = "engram-vault"
access_key = "AKIAXXXXXXXX"
secret_key = "xxxxxxxxxx"
```
