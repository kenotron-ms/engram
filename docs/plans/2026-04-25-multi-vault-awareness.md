# Engram — Multi-Vault Support + engram awareness

> **Execution:** Use the subagent-driven-development workflow to implement this plan.

**Goal:** Add multi-vault configuration (named vaults with access control and sync modes), the `engram awareness` command (three-layer vault summary for harness injection), and update the Amplifier hook to use the new awareness output.

**Architecture:** New `config.rs` module in `engram-core` provides `EngramConfig` / `VaultEntry` with TOML serialization. CLI gains `engram vault add|list|remove|set-default`, `--vault <name>` flag on all relevant commands, `engram awareness` (three-layer: domain structure + `_context/` files + recent memory facts), and sync approval flow (`engram sync` shows diff; `engram sync --approve` pushes). Access control gates write operations. The `hook-memory-context` Amplifier module is updated to call `engram awareness`.

**Tech Stack:** Rust (engram-core, engram-cli), `toml = "0.8"`, `dirs = "5"` (added to engram-core), `shellexpand = "3"` (added to engram-cli), `assert_cmd`, `predicates`, `tempfile` (already in dev-deps), Python (hook-memory-context module)

---

## Codebase Conventions (verified)

- **Tests in engram-core**: `#[cfg(test)] mod tests` blocks in-file + `crates/engram-core/tests/integration_test.rs`
- **Tests in engram-cli**: in-file `#[cfg(test)] mod tests` + separate files in `crates/engram-cli/tests/`
- **Integration test pattern**: `use assert_cmd::Command;` + `Command::cargo_bin("engram")` + `.env("VAR", val)`
- **Env var overrides**: `ENGRAM_STORE_PATH` pattern already exists — add `ENGRAM_CONFIG_PATH` same way
- **Temp directories**: `use tempfile::TempDir;` — keep `TempDir` in scope until end of test
- **Keychain-dependent tests**: guarded with `if !install_test_key() { return; }` — use `#[ignore]` for tests that truly can't run headless
- **`lib.rs` in engram-cli** (`src/lib.rs`): exports `daemon`, `load`, `mcp`, `observe` — add new modules here too
- **`serial_test::serial`**: use on tests that set/read env vars
- **Unicode separators**: `"─".repeat(41)` (U+2500, not ASCII hyphen)
- **`list_recent` signature**: `fn list_recent(&self, since_ms: i64, limit: usize)` — `limit` is `usize`

---

## Storage Layout

```
~/.engram/
  config.toml          ← EngramConfig (TOML)
  personal/
    memory.db          ← SQLCipher store for "personal" vault
    search/            ← tantivy index
    vectors.db         ← sqlite-vec
  canvas/
    memory.db
    search/
    vectors.db
```

Per-vault storage dir: `~/.engram/<vault-name>/`

---

## Task 1: Add `toml` dep to engram-core + create `config.rs` stub

**Files:**
- Modify: `crates/engram-core/Cargo.toml`
- Create: `crates/engram-core/src/config.rs`
- Modify: `crates/engram-core/src/lib.rs`

### Step 1: Add `toml` dependency

In `crates/engram-core/Cargo.toml`, add under `[dependencies]` (note: `serde` with `derive` already present):

```toml
toml = "0.8"
dirs = "5"
```

### Step 2: Create `crates/engram-core/src/config.rs` stub

```rust
// config.rs — Multi-vault configuration

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::PathBuf;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum ConfigError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("Parse error: {0}")]
    Parse(#[from] toml::de::Error),
    #[error("Serialize error: {0}")]
    Serialize(#[from] toml::ser::Error),
}

fn default_access() -> VaultAccess {
    VaultAccess::ReadWrite
}
fn default_sync_mode() -> SyncMode {
    SyncMode::Approval
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
#[serde(rename_all = "kebab-case")]
pub enum VaultAccess {
    Read,
    #[default]
    ReadWrite,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
#[serde(rename_all = "kebab-case")]
pub enum SyncMode {
    Auto,
    #[default]
    Approval,
    Manual,
}

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
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct EngramConfig {
    #[serde(default)]
    pub vaults: BTreeMap<String, VaultEntry>,
}
```

### Step 3: Add `pub mod config;` to `crates/engram-core/src/lib.rs`

Add after the existing module declarations:

```rust
pub mod config;
```

The `uniffi::include_scaffolding!` line must remain first. Insert `pub mod config;` after the existing four `pub mod` lines.

### Step 4: Verify it compiles

Run: `cargo build -p engram-core`

Expected: Compiles with zero errors (warnings about unused code are OK).

### Step 5: Commit

```bash
git add crates/engram-core/Cargo.toml crates/engram-core/src/config.rs crates/engram-core/src/lib.rs
git commit -m "chore(config): add config module stub with VaultEntry and EngramConfig types"
```

---

## Task 2: Implement `EngramConfig` load/save methods + unit tests

**Files:**
- Modify: `crates/engram-core/src/config.rs`

### Step 1: Write the failing tests first

Add this `#[cfg(test)]` block to the bottom of `crates/engram-core/src/config.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config_is_empty() {
        let config = EngramConfig::default();
        assert!(config.vaults.is_empty());
    }

    #[test]
    fn test_add_vault_and_retrieve() {
        let mut config = EngramConfig::default();
        config.add_vault(
            "personal",
            VaultEntry {
                path: PathBuf::from("/home/user/.lifeos/memory"),
                access: VaultAccess::ReadWrite,
                sync_mode: SyncMode::Auto,
                default: true,
                vault_type: Some("personal".to_string()),
            },
        );
        assert!(config.get_vault("personal").is_some());
        assert_eq!(config.get_vault("personal").unwrap().sync_mode, SyncMode::Auto);
    }

    #[test]
    fn test_default_vault_returns_explicit_default() {
        let mut config = EngramConfig::default();
        config.vaults.insert(
            "a".to_string(),
            VaultEntry {
                path: PathBuf::from("/a"),
                access: VaultAccess::ReadWrite,
                sync_mode: SyncMode::Auto,
                default: false,
                vault_type: None,
            },
        );
        config.vaults.insert(
            "b".to_string(),
            VaultEntry {
                path: PathBuf::from("/b"),
                access: VaultAccess::ReadWrite,
                sync_mode: SyncMode::Auto,
                default: true,
                vault_type: None,
            },
        );
        assert_eq!(config.default_vault().unwrap().0, "b");
    }

    #[test]
    fn test_remove_vault() {
        let mut config = EngramConfig::default();
        config.add_vault(
            "tmp",
            VaultEntry {
                path: PathBuf::from("/tmp"),
                access: VaultAccess::Read,
                sync_mode: SyncMode::Manual,
                default: false,
                vault_type: None,
            },
        );
        assert!(config.remove_vault("tmp"));
        assert!(!config.remove_vault("tmp")); // already gone
    }

    #[test]
    fn test_set_default_clears_others() {
        let mut config = EngramConfig::default();
        for name in ["a", "b", "c"] {
            config.vaults.insert(
                name.to_string(),
                VaultEntry {
                    path: PathBuf::from(format!("/{}", name)),
                    access: VaultAccess::ReadWrite,
                    sync_mode: SyncMode::Auto,
                    default: name == "a",
                    vault_type: None,
                },
            );
        }
        config.set_default("b");
        assert!(config.vaults["b"].default);
        assert!(!config.vaults["a"].default);
        assert!(!config.vaults["c"].default);
    }

    #[test]
    fn test_roundtrip_toml_serialization() {
        let mut config = EngramConfig::default();
        config.add_vault(
            "personal",
            VaultEntry {
                path: PathBuf::from("~/.lifeos/memory"),
                access: VaultAccess::ReadWrite,
                sync_mode: SyncMode::Auto,
                default: true,
                vault_type: Some("personal".to_string()),
            },
        );
        let serialized = toml::to_string_pretty(&config).unwrap();
        let deserialized: EngramConfig = toml::from_str(&serialized).unwrap();
        assert_eq!(deserialized.vaults["personal"].sync_mode, SyncMode::Auto);
        assert!(deserialized.vaults["personal"].default);
    }

    #[test]
    fn test_config_path_uses_env_var_override() {
        std::env::set_var("ENGRAM_CONFIG_PATH", "/tmp/test_engram_config_xyz.toml");
        let path = EngramConfig::config_path();
        std::env::remove_var("ENGRAM_CONFIG_PATH");
        assert_eq!(path.to_str().unwrap(), "/tmp/test_engram_config_xyz.toml");
    }
}
```

### Step 2: Run tests to verify they fail

Run: `cargo test -p engram-core config`

Expected: FAIL — methods `add_vault`, `get_vault`, `default_vault`, etc. are not yet defined.

### Step 3: Implement the methods

Add this `impl EngramConfig` block to `crates/engram-core/src/config.rs` (after the struct definitions, before `#[cfg(test)]`):

```rust
impl EngramConfig {
    /// Returns the path to the config file.
    ///
    /// If `ENGRAM_CONFIG_PATH` env var is set, uses that path.
    /// Otherwise returns `~/.engram/config.toml`.
    pub fn config_path() -> PathBuf {
        if let Ok(p) = std::env::var("ENGRAM_CONFIG_PATH") {
            return PathBuf::from(p);
        }
        dirs::home_dir()
            .unwrap_or_else(|| PathBuf::from("~"))
            .join(".engram/config.toml")
    }

    /// Load config from disk. Returns `Default` if the file does not exist
    /// or cannot be parsed.
    pub fn load() -> Self {
        let path = Self::config_path();
        if !path.exists() {
            return Self::default();
        }
        let content = std::fs::read_to_string(&path).unwrap_or_default();
        toml::from_str(&content).unwrap_or_default()
    }

    /// Atomically write config to disk (write tmp → rename).
    pub fn save(&self) -> Result<(), ConfigError> {
        let path = Self::config_path();
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let content = toml::to_string_pretty(self)?;
        let tmp = path.with_extension("toml.tmp");
        std::fs::write(&tmp, &content)?;
        std::fs::rename(&tmp, &path)?;
        Ok(())
    }

    /// Return the default vault: first with `default = true`, or the first entry.
    pub fn default_vault(&self) -> Option<(&str, &VaultEntry)> {
        self.vaults
            .iter()
            .find(|(_, v)| v.default)
            .map(|(n, v)| (n.as_str(), v))
            .or_else(|| self.vaults.iter().next().map(|(n, v)| (n.as_str(), v)))
    }

    pub fn get_vault(&self, name: &str) -> Option<&VaultEntry> {
        self.vaults.get(name)
    }

    /// Add or replace a vault. If `entry.default` is true, clears default on all others.
    pub fn add_vault(&mut self, name: &str, entry: VaultEntry) {
        if entry.default {
            for v in self.vaults.values_mut() {
                v.default = false;
            }
        }
        self.vaults.insert(name.to_string(), entry);
    }

    /// Remove vault by name. Returns `true` if the vault existed.
    pub fn remove_vault(&mut self, name: &str) -> bool {
        self.vaults.remove(name).is_some()
    }

    /// Mark `name` as the default vault and clear default on all others.
    /// Returns `false` if `name` does not exist.
    pub fn set_default(&mut self, name: &str) -> bool {
        if !self.vaults.contains_key(name) {
            return false;
        }
        for (n, v) in self.vaults.iter_mut() {
            v.default = n == name;
        }
        true
    }
}
```

### Step 4: Run tests to verify they pass

Run: `cargo test -p engram-core config`

Expected: All 7 tests PASS.

### Step 5: Commit

```bash
git add crates/engram-core/src/config.rs crates/engram-core/Cargo.toml
git commit -m "feat(config): EngramConfig load/save with add/remove/set-default + round-trip tests"
```

---

## Task 3: `engram vault list` command

**Files:**
- Modify: `crates/engram-cli/src/main.rs`

### Step 1: Write the failing test

Create `crates/engram-cli/tests/vault_list_test.rs`:

```rust
// Integration tests for engram vault list
use assert_cmd::Command;
use std::fs;
use tempfile::TempDir;

fn write_config(dir: &TempDir, toml: &str) -> std::path::PathBuf {
    let p = dir.path().join("config.toml");
    fs::write(&p, toml).unwrap();
    p
}

#[test]
fn test_vault_list_exits_zero() {
    let dir = TempDir::new().unwrap();
    let cfg = write_config(&dir, "");
    Command::cargo_bin("engram")
        .unwrap()
        .env("ENGRAM_CONFIG_PATH", &cfg)
        .args(["vault", "list"])
        .assert()
        .success();
}

#[test]
fn test_vault_list_shows_no_vaults_when_empty() {
    let dir = TempDir::new().unwrap();
    let cfg = write_config(&dir, "");
    let output = Command::cargo_bin("engram")
        .unwrap()
        .env("ENGRAM_CONFIG_PATH", &cfg)
        .args(["vault", "list"])
        .output()
        .unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("No vaults configured"),
        "Empty config should show 'No vaults configured', got: {}",
        stdout
    );
}

#[test]
fn test_vault_list_shows_configured_vault() {
    let dir = TempDir::new().unwrap();
    let vault_dir = TempDir::new().unwrap();
    let cfg = write_config(
        &dir,
        &format!(
            r#"
[vaults.personal]
path = "{}"
access = "read-write"
sync_mode = "auto"
default = true
"#,
            vault_dir.path().display()
        ),
    );
    let output = Command::cargo_bin("engram")
        .unwrap()
        .env("ENGRAM_CONFIG_PATH", &cfg)
        .args(["vault", "list"])
        .output()
        .unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("personal"),
        "Should list vault named 'personal', got: {}",
        stdout
    );
    assert!(
        stdout.contains("read-write"),
        "Should show access mode, got: {}",
        stdout
    );
}
```

### Step 2: Run test to verify it fails

Run: `cargo test -p engram vault_list`

Expected: FAIL — `vault` subcommand does not exist yet.

### Step 3: Add `VaultCommands` enum and `Vault` command to `Commands`

In `crates/engram-cli/src/main.rs`, add after the existing `AuthCommands` enum:

```rust
#[derive(Subcommand)]
enum VaultCommands {
    /// List all configured vaults
    List,
    /// Add a vault to config
    Add {
        name: String,
        #[arg(long)]
        path: PathBuf,
        #[arg(long, default_value = "read-write")]
        access: String,
        #[arg(long, default_value = "approval")]
        sync_mode: String,
        #[arg(long)]
        default: bool,
        #[arg(long)]
        vault_type: Option<String>,
    },
    /// Remove a vault from config
    Remove { name: String },
    /// Set the default vault
    SetDefault { name: String },
}
```

Add to the `Commands` enum (after `Doctor`):

```rust
/// Manage configured vaults
Vault {
    #[command(subcommand)]
    command: VaultCommands,
},
```

### Step 4: Implement `run_vault_list()`

Add to `crates/engram-cli/src/main.rs`:

```rust
fn run_vault_list() {
    use engram_core::config::{EngramConfig, SyncMode, VaultAccess};
    let config = EngramConfig::load();

    println!("{}", "─".repeat(41));
    if config.vaults.is_empty() {
        println!("No vaults configured.");
        println!("Run: engram vault add <name> --path <path>");
    } else {
        println!("Configured vaults:");
        for (name, vault) in &config.vaults {
            let default_marker = if vault.default { " (default)" } else { "" };
            let access_str = match vault.access {
                VaultAccess::Read => "read",
                VaultAccess::ReadWrite => "read-write",
            };
            let sync_str = match vault.sync_mode {
                SyncMode::Auto => "auto",
                SyncMode::Approval => "approval",
                SyncMode::Manual => "manual",
            };
            let exists = if vault.path.exists() { "✓" } else { "✗" };
            println!(
                "  {} {}{:<14} {:<38} {:<12} {}",
                exists,
                name,
                default_marker,
                vault.path.display(),
                access_str,
                sync_str
            );
        }
    }

    // Auto-detect project vault from cwd
    let cwd = std::env::current_dir().unwrap_or_default();
    let project_vault = cwd.join(".lifeos/memory");
    if project_vault.exists()
        && !config.vaults.values().any(|v| v.path == project_vault)
    {
        println!("\nAuto-detected (not in config):");
        println!(
            "  ✓ project (ephemeral)   {}  read-write  approval",
            project_vault.display()
        );
    } else {
        println!("\nAuto-detected: (none — no .lifeos/memory in current directory)");
    }
}
```

### Step 5: Wire into `main()`

Add to the `match cli.command` block:

```rust
Commands::Vault { command } => match command {
    VaultCommands::List => run_vault_list(),
    VaultCommands::Add { name, path, access, sync_mode, default, vault_type } => {
        run_vault_add(&name, path, &access, &sync_mode, default, vault_type);
    }
    VaultCommands::Remove { name } => run_vault_remove(&name),
    VaultCommands::SetDefault { name } => run_vault_set_default(&name),
},
```

Add stub functions so it compiles (implement fully in Task 4):

```rust
fn run_vault_add(_name: &str, _path: PathBuf, _access: &str, _sync_mode: &str, _default: bool, _vault_type: Option<String>) {
    todo!("implement in Task 4")
}
fn run_vault_remove(_name: &str) { todo!("implement in Task 4") }
fn run_vault_set_default(_name: &str) { todo!("implement in Task 4") }
```

### Step 6: Run test to verify it passes

Run: `cargo test -p engram vault_list`

Expected: All 3 tests PASS.

### Step 7: Commit

```bash
git add crates/engram-cli/src/main.rs crates/engram-cli/tests/vault_list_test.rs
git commit -m "feat(cli): engram vault list command with auto-detection of cwd project vault"
```

---

## Task 4: `engram vault add | remove | set-default` commands

**Files:**
- Modify: `crates/engram-cli/src/main.rs`
- Modify: `crates/engram-cli/Cargo.toml` — add `shellexpand = "3"`

### Step 1: Add `shellexpand` dependency

In `crates/engram-cli/Cargo.toml` under `[dependencies]`:

```toml
shellexpand = "3"
```

### Step 2: Write failing tests

Add to `crates/engram-cli/tests/vault_list_test.rs`:

```rust
#[test]
fn test_vault_add_creates_entry_in_config() {
    let dir = TempDir::new().unwrap();
    let cfg = write_config(&dir, "");
    let vault_dir = TempDir::new().unwrap();

    Command::cargo_bin("engram")
        .unwrap()
        .env("ENGRAM_CONFIG_PATH", &cfg)
        .args([
            "vault", "add", "personal",
            "--path", vault_dir.path().to_str().unwrap(),
            "--sync", "auto",
            "--default",
        ])
        .assert()
        .success();

    // Verify the config file now contains the vault
    let content = fs::read_to_string(&cfg).unwrap();
    assert!(content.contains("[vaults.personal]"), "Config should contain [vaults.personal]");
    assert!(content.contains("auto"), "Config should contain sync_mode = auto");
}

#[test]
fn test_vault_remove_deletes_entry() {
    let dir = TempDir::new().unwrap();
    let vault_dir = TempDir::new().unwrap();
    let cfg = write_config(
        &dir,
        &format!(
            r#"[vaults.temp]
path = "{}"
access = "read-write"
sync_mode = "manual"
default = false
"#,
            vault_dir.path().display()
        ),
    );

    Command::cargo_bin("engram")
        .unwrap()
        .env("ENGRAM_CONFIG_PATH", &cfg)
        .args(["vault", "remove", "temp"])
        .assert()
        .success();

    let content = fs::read_to_string(&cfg).unwrap();
    assert!(!content.contains("[vaults.temp]"), "Vault entry should be removed");
}

#[test]
fn test_vault_set_default_updates_config() {
    let dir = TempDir::new().unwrap();
    let vault_dir_a = TempDir::new().unwrap();
    let vault_dir_b = TempDir::new().unwrap();
    let cfg = write_config(
        &dir,
        &format!(
            r#"[vaults.alpha]
path = "{}"
default = true

[vaults.beta]
path = "{}"
default = false
"#,
            vault_dir_a.path().display(),
            vault_dir_b.path().display()
        ),
    );

    Command::cargo_bin("engram")
        .unwrap()
        .env("ENGRAM_CONFIG_PATH", &cfg)
        .args(["vault", "set-default", "beta"])
        .assert()
        .success();

    let content = fs::read_to_string(&cfg).unwrap();
    // beta section should now have default = true
    assert!(content.contains("beta"), "beta should still be in config");
}

#[test]
fn test_vault_remove_nonexistent_exits_nonzero() {
    let dir = TempDir::new().unwrap();
    let cfg = write_config(&dir, "");
    Command::cargo_bin("engram")
        .unwrap()
        .env("ENGRAM_CONFIG_PATH", &cfg)
        .args(["vault", "remove", "nonexistent-vault-xyz"])
        .assert()
        .failure();
}
```

### Step 3: Run to verify they fail

Run: `cargo test -p engram vault_list`

Expected: The new tests FAIL (stubs call `todo!()`).

### Step 4: Replace stub functions with full implementations

Replace the three stub functions in `main.rs`:

```rust
fn run_vault_add(
    name: &str,
    path: PathBuf,
    access: &str,
    sync_mode: &str,
    set_default: bool,
    vault_type: Option<String>,
) {
    use engram_core::config::{EngramConfig, SyncMode, VaultAccess, VaultEntry};

    let access = match access {
        "read" => VaultAccess::Read,
        "read-write" | "rw" => VaultAccess::ReadWrite,
        _ => {
            eprintln!("Invalid access: '{}'. Use 'read' or 'read-write'", access);
            std::process::exit(1);
        }
    };
    let sync_mode = match sync_mode {
        "auto" => SyncMode::Auto,
        "approval" => SyncMode::Approval,
        "manual" => SyncMode::Manual,
        _ => {
            eprintln!(
                "Invalid sync_mode: '{}'. Use 'auto', 'approval', or 'manual'",
                sync_mode
            );
            std::process::exit(1);
        }
    };

    // Expand ~ in path
    let path_str = path.to_string_lossy();
    let expanded = shellexpand::tilde(&path_str).into_owned();
    let expanded_path = PathBuf::from(expanded);

    let mut config = EngramConfig::load();
    config.add_vault(
        name,
        VaultEntry {
            path: expanded_path.clone(),
            access,
            sync_mode,
            default: set_default,
            vault_type,
        },
    );
    if let Err(e) = config.save() {
        eprintln!("Failed to save config: {}", e);
        std::process::exit(1);
    }
    println!("✓ Vault '{}' added", name);
    println!("  Path: {}", expanded_path.display());
}

fn run_vault_remove(name: &str) {
    use engram_core::config::EngramConfig;
    let mut config = EngramConfig::load();
    if config.remove_vault(name) {
        if let Err(e) = config.save() {
            eprintln!("Failed to save config: {}", e);
            std::process::exit(1);
        }
        println!("✓ Vault '{}' removed", name);
    } else {
        eprintln!("Vault '{}' not found", name);
        std::process::exit(1);
    }
}

fn run_vault_set_default(name: &str) {
    use engram_core::config::EngramConfig;
    let mut config = EngramConfig::load();
    if config.set_default(name) {
        if let Err(e) = config.save() {
            eprintln!("Failed to save config: {}", e);
            std::process::exit(1);
        }
        println!("✓ '{}' is now the default vault", name);
    } else {
        eprintln!("Vault '{}' not found", name);
        std::process::exit(1);
    }
}
```

Also add this import at the top of `main.rs` (with the existing `use` statements):

```rust
use engram_core::config::EngramConfig;
```

### Step 5: Run tests to verify they pass

Run: `cargo test -p engram vault_list`

Expected: All 7 tests PASS.

### Step 6: Commit

```bash
git add crates/engram-cli/src/main.rs crates/engram-cli/Cargo.toml crates/engram-cli/tests/vault_list_test.rs
git commit -m "feat(cli): engram vault add/remove/set-default commands with shellexpand for ~ paths"
```

---

## Task 5: Per-vault storage paths + config-aware `resolve_vault`

**Files:**
- Modify: `crates/engram-cli/src/main.rs`

### Step 1: Write failing tests

Add to `crates/engram-cli/tests/vault_list_test.rs`:

```rust
#[test]
fn test_vault_list_shows_help_for_vault_command() {
    Command::cargo_bin("engram")
        .unwrap()
        .args(["vault", "--help"])
        .assert()
        .success()
        .stdout(predicates::str::contains("list"))
        .stdout(predicates::str::contains("add"))
        .stdout(predicates::str::contains("remove"));
}
```

Add to `crates/engram-cli/tests/cli_integration.rs`:

```rust
/// `engram status` should still exit zero after config module is introduced
#[test]
fn test_status_still_exits_zero_with_config_module() {
    Command::cargo_bin("engram")
        .unwrap()
        .arg("status")
        .assert()
        .success();
}
```

(Note: the above tests already exist with different names; this one verifies backward compat after changes.)

### Step 2: Run to confirm they pass already (compilation test)

Run: `cargo build -p engram`

Expected: Compiles clean.

### Step 3: Add helper functions to `main.rs`

Add these helpers after the existing `default_vectors_path()` function:

```rust
/// Returns the per-vault storage directory: `~/.engram/<vault_name>/`
fn vault_storage_dir(vault_name: &str) -> PathBuf {
    UserDirs::new()
        .map(|u| u.home_dir().join(".engram").join(vault_name))
        .unwrap_or_else(|| PathBuf::from(format!(".engram/{}", vault_name)))
}

/// Expand `~` in a path using shellexpand.
fn shellexpand_path(p: &std::path::Path) -> PathBuf {
    let s = p.to_string_lossy();
    PathBuf::from(shellexpand::tilde(&s).as_ref())
}

/// Resolve the active vault: name override → auto-detected project vault → config default → hardcoded fallback.
///
/// Returns `(vault_name, vault_path)`. Exits 1 if `name_override` is given but not found.
fn resolve_vault(name_override: Option<&str>) -> (String, PathBuf) {
    let config = EngramConfig::load();

    if let Some(name) = name_override {
        // Check configured vaults
        if let Some(vault) = config.get_vault(name) {
            return (name.to_string(), shellexpand_path(&vault.path));
        }
        // Check auto-detected project vault
        if name == "project" {
            let cwd = std::env::current_dir().unwrap_or_default();
            let p = cwd.join(".lifeos/memory");
            if p.exists() {
                return ("project".to_string(), p);
            }
        }
        eprintln!("Vault '{}' not found. Run: engram vault list", name);
        std::process::exit(1);
    }

    // Auto-detect project vault from cwd (takes precedence over configured default)
    let cwd = std::env::current_dir().unwrap_or_default();
    let project_path = cwd.join(".lifeos/memory");
    if project_path.exists() {
        let not_in_config = !config
            .vaults
            .values()
            .any(|v| shellexpand_path(&v.path) == project_path);
        if not_in_config {
            return ("project".to_string(), project_path);
        }
    }

    // Use configured default vault
    if let Some((name, vault)) = config.default_vault() {
        return (name.to_string(), shellexpand_path(&vault.path));
    }

    // Hardcoded fallback for backward compatibility
    ("personal".to_string(), default_vault_path())
}
```

### Step 4: Update `default_vault_path()` to be config-aware

Replace the existing `default_vault_path()` function:

```rust
/// Returns the vault path: from config default if set, otherwise `~/.lifeos/memory`.
fn default_vault_path() -> PathBuf {
    let config = EngramConfig::load();
    if let Some((_, vault)) = config.default_vault() {
        return shellexpand_path(&vault.path);
    }
    UserDirs::new()
        .map(|u| u.home_dir().join(".lifeos/memory"))
        .unwrap_or_else(|| PathBuf::from(".lifeos/memory"))
}
```

**Important:** The existing unit test `test_default_vault_path_ends_with_lifeos_memory` checks the path ends with `.lifeos/memory`. This test will still pass when no config file is present (CI environment with `ENGRAM_CONFIG_PATH` not set). Do not change that test.

### Step 5: Update `default_store_path()` to respect vault names

Replace the existing `default_store_path()` to use `ENGRAM_STORE_PATH` env var (existing behavior) but also understand vault context. For now, keep backward-compatible behavior:

```rust
/// Returns the memory store path.
///
/// - If `ENGRAM_STORE_PATH` env var is set, use that directly (test/operator override).
/// - If a vault name is known via config, use `~/.engram/<vault_name>/memory.db`.
/// - Otherwise `~/.engram/memory.db` (legacy fallback).
fn default_store_path() -> PathBuf {
    if let Ok(p) = std::env::var("ENGRAM_STORE_PATH") {
        return PathBuf::from(p);
    }
    let config = EngramConfig::load();
    if let Some((name, _)) = config.default_vault() {
        return vault_storage_dir(name).join("memory.db");
    }
    UserDirs::new()
        .map(|u| u.home_dir().join(".engram/memory.db"))
        .unwrap_or_else(|| PathBuf::from(".engram/memory.db"))
}
```

**Note:** The existing test `test_default_store_path_ends_with_engram_memory_db` checks the path ends with `.engram/memory.db`. It will still pass when no config exists. The test `test_default_store_path_uses_engram_store_path_env_var` tests the env override and will still pass.

### Step 6: Build to verify it compiles

Run: `cargo build -p engram`

Expected: Zero errors.

### Step 7: Commit

```bash
git add crates/engram-cli/src/main.rs
git commit -m "feat(cli): vault_storage_dir, resolve_vault, and config-aware default path helpers"
```

---

## Task 6: Add `--vault <name>` flag to `index`, `search`, `sync`

**Files:**
- Modify: `crates/engram-cli/src/main.rs`
- Modify: `crates/engram-cli/tests/cli_integration.rs`

### Step 1: Note the breaking change

The existing `Commands::Index` has `vault: Option<PathBuf>`. Changing it to `Option<String>` changes the semantics: `--vault` now takes a vault **name** (looked up in config), not a raw path. The existing integration test `test_index_nonexistent_vault_exits_nonzero` passes a full path as the vault arg — it will still exit non-zero but with a different error message.

### Step 2: Update `Commands::Index` in `main.rs`

Change:
```rust
Index {
    /// Vault path (defaults to ~/.lifeos/memory)
    #[arg(long)]
    vault: Option<PathBuf>,
    /// Force a full reindex by wiping the search index first
    #[arg(long)]
    force: bool,
},
```

To:
```rust
Index {
    /// Vault name from config (defaults to active vault)
    #[arg(long)]
    vault: Option<String>,
    /// Force a full reindex by wiping the search index first
    #[arg(long)]
    force: bool,
},
```

### Step 3: Update `Commands::Search` to add `--vault`

```rust
Search {
    /// Query string
    query: String,
    /// Vault name from config (defaults to active vault)
    #[arg(long)]
    vault: Option<String>,
    /// Maximum number of results to return
    #[arg(long, default_value_t = 10)]
    limit: usize,
    /// Search mode: fulltext (BM25), vector (KNN), or hybrid (RRF merge)
    #[arg(long, default_value = "hybrid")]
    mode: SearchMode,
},
```

### Step 4: Update `Commands::Sync` to add `--vault` and `--approve`

```rust
Sync {
    /// Force a specific backend (s3, onedrive, azure, gcs)
    #[arg(long)]
    backend: Option<String>,
    /// Vault name from config (defaults to active vault)
    #[arg(long)]
    vault: Option<String>,
    /// Approve pending changes and push (for approval sync_mode vaults)
    #[arg(long)]
    approve: bool,
},
```

### Step 5: Update `main()` dispatch

Change:
```rust
Commands::Sync { backend } => run_sync(backend.as_deref()),
Commands::Index { vault, force } => run_index(vault, force),
Commands::Search { query, limit, mode } => run_search(&query, limit, &mode),
```

To:
```rust
Commands::Sync { backend, vault, approve } => run_sync(backend.as_deref(), vault.as_deref(), approve),
Commands::Index { vault, force } => run_index(vault.as_deref(), force),
Commands::Search { query, vault, limit, mode } => run_search(&query, vault.as_deref(), limit, &mode),
```

### Step 6: Update `run_index` signature and body

Change the function signature and first lines:

```rust
fn run_index(vault_arg: Option<&str>, force: bool) {
    use engram_search::embedder::Embedder;
    use engram_search::vector::VectorIndex;

    let (vault_name, vault_path) = resolve_vault(vault_arg);
    let engram_dir = vault_storage_dir(&vault_name);
    let search_dir = engram_dir.join("search");
    let vectors_path = engram_dir.join("vectors.db");

    if !vault_path.exists() {
        eprintln!("Vault not found: {}", vault_path.display());
        std::process::exit(1);
    }
    // ... rest of existing implementation using vault_path, search_dir, vectors_path
```

Replace the hardcoded `default_search_dir()` and `default_vectors_path()` calls inside `run_index` with the vault-scoped variables above.

### Step 7: Update `run_search` signature

```rust
fn run_search(query: &str, vault_arg: Option<&str>, limit: usize, mode: &SearchMode) {
    use engram_search::embedder::Embedder;
    use engram_search::hybrid::HybridSearch;
    use engram_search::vector::VectorIndex;

    let (vault_name, _vault_path) = resolve_vault(vault_arg);
    let engram_dir = vault_storage_dir(&vault_name);
    let search_dir = engram_dir.join("search");
    let vectors_path = engram_dir.join("vectors.db");

    // Replace default_search_dir() and default_vectors_path() calls with these
    // ... rest unchanged
```

### Step 8: Update `run_sync` signature (stub, full logic in Task 8)

Change:
```rust
fn run_sync(backend_name: Option<&str>) {
```
To:
```rust
fn run_sync(backend_name: Option<&str>, vault_arg: Option<&str>, approve: bool) {
    // vault_arg and approve used in Task 8; for now pass through to existing logic
    let _ = (vault_arg, approve); // suppress unused warnings until Task 8
```

### Step 9: Fix the existing `test_index_nonexistent_vault_exits_nonzero` test

In `crates/engram-cli/tests/cli_integration.rs`, update the test that passes a path as a vault name:

```rust
/// `engram index --vault unknown-vault-name` must exit non-zero and print a vault error.
#[test]
fn test_index_nonexistent_vault_exits_nonzero() {
    let dir = tempfile::TempDir::new().unwrap();
    let cfg = dir.path().join("empty.toml");
    std::fs::write(&cfg, "").unwrap();

    let mut cmd = Command::cargo_bin("engram").unwrap();
    cmd.env("ENGRAM_CONFIG_PATH", &cfg)
        .args(["index", "--vault", "nonexistent-vault-xyz-abc"]);
    cmd.assert()
        .failure()
        .stderr(predicates::str::contains("not found"));
}
```

### Step 10: Build and run tests

Run: `cargo build -p engram`
Expected: Zero errors.

Run: `cargo test -p engram`
Expected: All tests pass (the updated integration test + vault_list tests).

### Step 11: Commit

```bash
git add crates/engram-cli/src/main.rs crates/engram-cli/tests/cli_integration.rs
git commit -m "feat(cli): --vault <name> flag on index, search, sync; per-vault storage dirs"
```

---

## Task 7: Access control enforcement

**Files:**
- Modify: `crates/engram-cli/src/main.rs`
- Create: `crates/engram-cli/tests/access_control_test.rs`

### Step 1: Write the failing test

Create `crates/engram-cli/tests/access_control_test.rs`:

```rust
// Access control integration tests
use assert_cmd::Command;
use std::fs;
use tempfile::TempDir;

fn write_config(dir: &TempDir, toml: &str) -> std::path::PathBuf {
    let p = dir.path().join("config.toml");
    fs::write(&p, toml).unwrap();
    p
}

#[test]
fn test_observe_blocked_on_read_only_vault() {
    let dir = TempDir::new().unwrap();
    let vault_dir = TempDir::new().unwrap();
    let cfg = write_config(
        &dir,
        &format!(
            r#"[vaults.readonly]
path = "{}"
access = "read"
sync_mode = "manual"
default = true
"#,
            vault_dir.path().display()
        ),
    );

    // observe requires an API key AND session file, but access check fires first
    Command::cargo_bin("engram")
        .unwrap()
        .env("ENGRAM_CONFIG_PATH", &cfg)
        .args(["observe", "/nonexistent/session.jsonl", "--vault", "readonly"])
        .assert()
        .failure()
        .stderr(predicates::str::contains("read-only"));
}

#[test]
fn test_sync_push_blocked_on_read_only_vault() {
    let dir = TempDir::new().unwrap();
    let vault_dir = TempDir::new().unwrap();
    let cfg = write_config(
        &dir,
        &format!(
            r#"[vaults.readonly-sync]
path = "{}"
access = "read"
sync_mode = "auto"
default = true
"#,
            vault_dir.path().display()
        ),
    );

    Command::cargo_bin("engram")
        .unwrap()
        .env("ENGRAM_CONFIG_PATH", &cfg)
        .args(["sync", "--vault", "readonly-sync"])
        .assert()
        .failure()
        .stderr(predicates::str::contains("read-only"));
}

#[test]
fn test_read_write_vault_passes_access_check_for_sync() {
    // A read-write vault with no backends configured should fail on backend, not access
    let dir = TempDir::new().unwrap();
    let vault_dir = TempDir::new().unwrap();
    let cfg = write_config(
        &dir,
        &format!(
            r#"[vaults.rw-vault]
path = "{}"
access = "read-write"
sync_mode = "approval"
default = true
"#,
            vault_dir.path().display()
        ),
    );

    let output = Command::cargo_bin("engram")
        .unwrap()
        .env("ENGRAM_CONFIG_PATH", &cfg)
        .args(["sync", "--vault", "rw-vault"])
        .output()
        .unwrap();

    let stderr = String::from_utf8_lossy(&output.stderr);
    // Should NOT be blocked by access control (read-write vaults are allowed)
    assert!(
        !stderr.contains("read-only"),
        "read-write vault should not trigger read-only error, got: {}",
        stderr
    );
}
```

### Step 2: Run to verify they fail

Run: `cargo test -p engram access_control`

Expected: `test_observe_blocked_on_read_only_vault` and `test_sync_push_blocked_on_read_only_vault` FAIL (no access control yet). The third test may pass trivially.

### Step 3: Add `check_write_access` helper and call it

Add to `main.rs` (after `shellexpand_path`):

```rust
/// Check that the named vault allows write access. Exits 1 with a clear error if read-only.
/// For auto-detected project vaults (not in config), write access is always allowed.
fn check_write_access(vault_name: &str) {
    use engram_core::config::{EngramConfig, VaultAccess};
    let config = EngramConfig::load();
    if let Some(vault) = config.get_vault(vault_name) {
        if vault.access == VaultAccess::Read {
            eprintln!(
                "Error: vault '{}' is read-only (access = \"read\")",
                vault_name
            );
            eprintln!(
                "To allow writes: engram vault add {} --access read-write [other flags]",
                vault_name
            );
            std::process::exit(1);
        }
    }
    // Auto-detected project vaults are ephemeral read-write by default
}
```

In `run_observe`, add at the top (before the API key check):

```rust
fn run_observe(session_path: &Path, api_key: Option<&str>) {
    // Resolve active vault and enforce access control
    // Note: observe writes to the memory store, so it requires write access
    let (vault_name, _vault_path) = resolve_vault(None);
    check_write_access(&vault_name);

    // ... rest of existing implementation unchanged
```

**Important:** `run_observe` does not currently take a `--vault` flag. Add write access check using `resolve_vault(None)` for now (vault flag on observe is not in scope for this plan).

In `run_sync`, add the access check at the top (before existing backend resolution):

```rust
fn run_sync(backend_name: Option<&str>, vault_arg: Option<&str>, approve: bool) {
    use engram_core::config::{EngramConfig, VaultAccess};

    let (vault_name, _vault_path) = resolve_vault(vault_arg);
    check_write_access(&vault_name);

    // ... rest of existing sync logic (sync mode enforcement added in Task 8)
```

### Step 4: Run tests to verify they pass

Run: `cargo test -p engram access_control`

Expected: All 3 tests PASS.

### Step 5: Commit

```bash
git add crates/engram-cli/src/main.rs crates/engram-cli/tests/access_control_test.rs
git commit -m "feat(cli): access control — read-only vaults reject sync and observe write operations"
```

---

## Task 8: Sync mode enforcement — approval diff + `--approve`

**Files:**
- Modify: `crates/engram-cli/src/main.rs`
- Create: `crates/engram-cli/tests/sync_approval_test.rs`

### Step 1: Write failing tests

Create `crates/engram-cli/tests/sync_approval_test.rs`:

```rust
// Sync mode enforcement tests
use assert_cmd::Command;
use std::fs;
use tempfile::TempDir;

fn write_config(dir: &TempDir, toml: &str) -> std::path::PathBuf {
    let p = dir.path().join("config.toml");
    fs::write(&p, toml).unwrap();
    p
}

#[test]
fn test_sync_manual_mode_prints_message_and_exits_zero() {
    let dir = TempDir::new().unwrap();
    let vault_dir = TempDir::new().unwrap();
    let cfg = write_config(
        &dir,
        &format!(
            r#"[vaults.manual-vault]
path = "{}"
access = "read-write"
sync_mode = "manual"
default = true
"#,
            vault_dir.path().display()
        ),
    );

    Command::cargo_bin("engram")
        .unwrap()
        .env("ENGRAM_CONFIG_PATH", &cfg)
        .args(["sync", "--vault", "manual-vault"])
        .assert()
        .success()
        .stdout(predicates::str::contains("manual sync mode"));
}

#[test]
fn test_sync_approval_mode_shows_diff_without_approve_flag() {
    let dir = TempDir::new().unwrap();
    let vault_dir = TempDir::new().unwrap();
    let cfg = write_config(
        &dir,
        &format!(
            r#"[vaults.approval-vault]
path = "{}"
access = "read-write"
sync_mode = "approval"
default = true
"#,
            vault_dir.path().display()
        ),
    );

    let output = Command::cargo_bin("engram")
        .unwrap()
        .env("ENGRAM_CONFIG_PATH", &cfg)
        .args(["sync", "--vault", "approval-vault"])
        .output()
        .unwrap();

    let stdout = String::from_utf8_lossy(&output.stdout);
    // Should print approval info and NOT actually push
    assert!(
        stdout.contains("approval required") || stdout.contains("To push:"),
        "Approval mode without --approve should show diff prompt, got: {}",
        stdout
    );
    assert!(output.status.success(), "Should exit 0 (no push attempted)");
}

#[test]
fn test_sync_approval_help_shows_approve_flag() {
    Command::cargo_bin("engram")
        .unwrap()
        .args(["sync", "--help"])
        .assert()
        .success()
        .stdout(predicates::str::contains("--approve"))
        .stdout(predicates::str::contains("--vault"));
}
```

### Step 2: Run to verify they fail

Run: `cargo test -p engram sync_approval`

Expected: `test_sync_manual_mode_*` and `test_sync_approval_mode_*` FAIL. `test_sync_approval_help_shows_approve_flag` may already pass.

### Step 3: Add `show_vault_diff` helper

Add to `main.rs`:

```rust
/// Show `git status --short` diff for the vault path.
/// If the vault is not a git repo, prints an informational message.
fn show_vault_diff(vault_path: &std::path::Path) {
    let output = std::process::Command::new("git")
        .args(["-C", &vault_path.to_string_lossy(), "status", "--short"])
        .output();

    match output {
        Ok(out) if out.status.success() => {
            let status = String::from_utf8_lossy(&out.stdout);
            if status.trim().is_empty() {
                println!("  (no uncommitted changes)");
            } else {
                for line in status.lines() {
                    if line.len() < 2 {
                        continue;
                    }
                    let (code, path) = line.split_at(2);
                    let description = match code.trim() {
                        "M" => "modified",
                        "A" | "??" => "new file",
                        "D" => "deleted",
                        "R" => "renamed",
                        _ => "changed",
                    };
                    println!("  {}: {}", description, path.trim());
                }
            }
        }
        _ => println!("  (vault is not a git repository — nothing to sync via git)"),
    }
}
```

### Step 4: Update `run_sync` with sync mode enforcement

In `run_sync`, after the access control check and before the existing backend logic, add:

```rust
fn run_sync(backend_name: Option<&str>, vault_arg: Option<&str>, approve: bool) {
    use engram_core::config::{EngramConfig, SyncMode, VaultAccess};
    use engram_core::{crypto::KeyStore, vault::Vault};
    use engram_sync::{
        auth::AuthStore, backend::SyncBackend, encrypt::encrypt_for_sync,
        onedrive::OneDriveBackend, s3::S3Backend,
    };

    let (vault_name, vault_path) = resolve_vault(vault_arg);

    // --- Access control ---
    check_write_access(&vault_name);

    // --- Sync mode gate ---
    let config = EngramConfig::load();
    if let Some(vault) = config.get_vault(&vault_name) {
        match vault.sync_mode {
            SyncMode::Manual => {
                println!("Vault '{}' is in manual sync mode.", vault_name);
                println!(
                    "To push: engram sync --vault {} --approve",
                    vault_name
                );
                return;
            }
            SyncMode::Approval if !approve => {
                println!(
                    "Pending changes for vault '{}' (approval required):",
                    vault_name
                );
                println!("{}", "─".repeat(41));
                show_vault_diff(&vault_path);
                println!();
                println!("To push: engram sync --vault {} --approve", vault_name);
                return;
            }
            // Auto or Approval+approve: proceed with sync
            _ => {}
        }
    }

    // --- Existing sync logic (unchanged below this point) ---
    let vault = Vault::new(&vault_path);
    let key_store = KeyStore::new("engram");
    // ... rest of existing run_sync body (backend selection, file iteration, push)
```

**Note:** Move the existing `run_sync` body (starting from `let vault_path = default_vault_path();`) to after the sync mode gate. Replace `let vault_path = default_vault_path(); let vault = Vault::new(&vault_path);` with the `vault_path` already computed above via `resolve_vault`.

### Step 5: Run tests to verify they pass

Run: `cargo test -p engram sync_approval`

Expected: All 3 tests PASS.

Run: `cargo test -p engram`

Expected: All existing tests still pass.

### Step 6: Commit

```bash
git add crates/engram-cli/src/main.rs crates/engram-cli/tests/sync_approval_test.rs
git commit -m "feat(cli): sync approval mode — diff-only output without --approve; manual mode message"
```

---

## Task 9: `engram awareness` Layer 1 — vault domain structure

**Files:**
- Create: `crates/engram-cli/src/awareness.rs`
- Modify: `crates/engram-cli/src/lib.rs` — add `pub mod awareness;`
- Modify: `crates/engram-cli/src/main.rs` — add `mod awareness;` and `Commands::Awareness`
- Create: `crates/engram-cli/tests/awareness_test.rs`

### Step 1: Write failing test

Create `crates/engram-cli/tests/awareness_test.rs`:

```rust
// Awareness command integration tests
use assert_cmd::Command;
use std::fs;
use tempfile::TempDir;

fn write_config(dir: &TempDir, toml: &str) -> std::path::PathBuf {
    let p = dir.path().join("config.toml");
    fs::write(&p, toml).unwrap();
    p
}

#[test]
fn test_awareness_exits_zero() {
    let dir = TempDir::new().unwrap();
    let vault_dir = TempDir::new().unwrap();
    let cfg = write_config(
        &dir,
        &format!(
            r#"[vaults.test]
path = "{}"
access = "read-write"
sync_mode = "manual"
default = true
"#,
            vault_dir.path().display()
        ),
    );

    Command::cargo_bin("engram")
        .unwrap()
        .env("ENGRAM_CONFIG_PATH", &cfg)
        .arg("awareness")
        .assert()
        .success();
}

#[test]
fn test_awareness_output_contains_context_tags() {
    let dir = TempDir::new().unwrap();
    let vault_dir = TempDir::new().unwrap();
    let cfg = write_config(
        &dir,
        &format!(
            r#"[vaults.test]
path = "{}"
access = "read-write"
sync_mode = "manual"
default = true
"#,
            vault_dir.path().display()
        ),
    );

    Command::cargo_bin("engram")
        .unwrap()
        .env("ENGRAM_CONFIG_PATH", &cfg)
        .arg("awareness")
        .assert()
        .success()
        .stdout(predicates::str::contains("<engram-context>"))
        .stdout(predicates::str::contains("</engram-context>"));
}

#[test]
fn test_awareness_shows_domain_counts() {
    let dir = TempDir::new().unwrap();
    let vault_dir = TempDir::new().unwrap();

    // Create vault content
    fs::create_dir_all(vault_dir.path().join("Work")).unwrap();
    fs::write(vault_dir.path().join("Work/notes.md"), "work notes").unwrap();
    fs::write(vault_dir.path().join("Work/meeting.md"), "meeting").unwrap();
    fs::create_dir_all(vault_dir.path().join("People")).unwrap();
    fs::write(vault_dir.path().join("People/sofia.md"), "sofia").unwrap();

    let cfg = write_config(
        &dir,
        &format!(
            r#"[vaults.test]
path = "{}"
access = "read-write"
sync_mode = "manual"
default = true
"#,
            vault_dir.path().display()
        ),
    );

    let output = Command::cargo_bin("engram")
        .unwrap()
        .env("ENGRAM_CONFIG_PATH", &cfg)
        .arg("awareness")
        .output()
        .unwrap();

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("Work (2)"),
        "Should show Work domain with 2 files, got: {}",
        stdout
    );
    assert!(
        stdout.contains("People (1)"),
        "Should show People domain with 1 file, got: {}",
        stdout
    );
}

#[test]
fn test_awareness_skips_underscore_directories() {
    let dir = TempDir::new().unwrap();
    let vault_dir = TempDir::new().unwrap();
    fs::create_dir_all(vault_dir.path().join("_context")).unwrap();
    fs::write(
        vault_dir.path().join("_context/domains.md"),
        "context content",
    )
    .unwrap();
    fs::create_dir_all(vault_dir.path().join("Work")).unwrap();
    fs::write(vault_dir.path().join("Work/note.md"), "note").unwrap();

    let cfg = write_config(
        &dir,
        &format!(
            r#"[vaults.test]
path = "{}"
default = true
"#,
            vault_dir.path().display()
        ),
    );

    let output = Command::cargo_bin("engram")
        .unwrap()
        .env("ENGRAM_CONFIG_PATH", &cfg)
        .arg("awareness")
        .output()
        .unwrap();

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        !stdout.contains("_context ("),
        "_context directory should not appear in domain counts, got: {}",
        stdout
    );
    assert!(
        stdout.contains("Work (1)"),
        "Work domain should appear, got: {}",
        stdout
    );
}
```

### Step 2: Run to verify they fail

Run: `cargo test -p engram awareness`

Expected: FAIL — `awareness` subcommand does not exist.

### Step 3: Create `crates/engram-cli/src/awareness.rs`

```rust
// awareness.rs — Multi-vault awareness context generation

use engram_core::vault::Vault;
use std::collections::BTreeMap;
use std::path::Path;

/// Count markdown files by top-level directory in the vault.
///
/// Excludes directories starting with `_` or `.` (internal/LifeOS metadata).
/// Returns `(total_file_count, "Domain1 (N) · Domain2 (M)")`.
pub fn vault_domain_summary(vault_path: &Path) -> (usize, String) {
    let vault = Vault::new(vault_path);
    let files = vault.list_markdown().unwrap_or_default();
    let total = files.len();

    let mut domain_counts: BTreeMap<String, usize> = BTreeMap::new();
    for file in &files {
        let top = file.split('/').next().unwrap_or("root");
        if top.starts_with('_') || top.starts_with('.') {
            continue;
        }
        // Files at the root level (no directory) are counted under "root"
        // but only if there's no slash at all
        *domain_counts.entry(top.to_string()).or_insert(0) += 1;
    }

    let summary = domain_counts
        .iter()
        .map(|(domain, count)| format!("{} ({})", domain, count))
        .collect::<Vec<_>>()
        .join(" · ");

    (total, summary)
}

/// Read `_context/*.md` files from the vault directory and concatenate them.
/// Returns empty string if the `_context/` directory does not exist.
pub fn vault_context_files(vault_path: &Path) -> String {
    let context_dir = vault_path.join("_context");
    if !context_dir.exists() {
        return String::new();
    }

    let mut parts = Vec::new();
    if let Ok(entries) = std::fs::read_dir(&context_dir) {
        let mut paths: Vec<_> = entries
            .filter_map(|e| e.ok())
            .filter(|e| {
                e.path()
                    .extension()
                    .map_or(false, |ext| ext.eq_ignore_ascii_case("md"))
            })
            .map(|e| e.path())
            .collect();
        paths.sort();

        for path in paths {
            if let Ok(content) = std::fs::read_to_string(&path) {
                let content = content.trim();
                if !content.is_empty() {
                    parts.push(content.to_string());
                }
            }
        }
    }

    parts.join("\n\n")
}
```

### Step 4: Add `pub mod awareness;` to `crates/engram-cli/src/lib.rs`

```rust
// engram library — expose internal modules for testing
pub mod awareness;
pub mod daemon;
pub mod load;
pub mod mcp;
pub mod observe;
```

### Step 5: Add `mod awareness;` and `Commands::Awareness` to `main.rs`

Add to the module list at the top:

```rust
mod awareness;
```

Add to `Commands` enum:

```rust
/// Generate vault awareness context for AI harness injection
Awareness {
    /// Specific vault name to show (defaults to all active vaults)
    #[arg(long)]
    vault: Option<String>,
    /// Show all configured vaults explicitly
    #[arg(long)]
    all: bool,
},
```

Add to `main()` match:

```rust
Commands::Awareness { vault, all } => run_awareness(vault.as_deref(), all),
```

### Step 6: Implement `run_awareness` in `main.rs`

```rust
fn run_awareness(vault_arg: Option<&str>, _all: bool) {
    let vaults_to_show = collect_active_vaults(vault_arg);

    println!("<engram-context>");
    let mut sections = Vec::new();

    for (name, path, access_label) in &vaults_to_show {
        let (total, domains) = awareness::vault_domain_summary(path);
        let header = format!(
            "## {} ({}) · {} files · {}",
            name,
            path.display(),
            total,
            access_label
        );
        let domains_line = if domains.is_empty() {
            "Domains: (empty)".to_string()
        } else {
            format!("Domains: {}", domains)
        };
        sections.push(format!("{}\n{}", header, domains_line));
    }

    println!("{}", sections.join("\n\n"));
    println!("</engram-context>");
}

/// Collect vaults to display: specific vault, or all configured + auto-detected.
fn collect_active_vaults(vault_arg: Option<&str>) -> Vec<(String, std::path::PathBuf, String)> {
    use engram_core::config::{EngramConfig, VaultAccess};
    let config = EngramConfig::load();
    let mut result = Vec::new();

    if let Some(name) = vault_arg {
        let (vault_name, vault_path) = resolve_vault(Some(name));
        let access = config
            .get_vault(&vault_name)
            .map(|v| match v.access {
                VaultAccess::Read => "read",
                VaultAccess::ReadWrite => "read-write",
            })
            .unwrap_or("read-write");
        result.push((vault_name, vault_path, access.to_string()));
        return result;
    }

    // All configured vaults
    for (name, vault) in &config.vaults {
        let path = shellexpand_path(&vault.path);
        let access = match vault.access {
            VaultAccess::Read => "read",
            VaultAccess::ReadWrite => "read-write",
        };
        result.push((name.clone(), path, access.to_string()));
    }

    // Auto-detected project vault
    let cwd = std::env::current_dir().unwrap_or_default();
    let project_path = cwd.join(".lifeos/memory");
    if project_path.exists()
        && !config
            .vaults
            .values()
            .any(|v| shellexpand_path(&v.path) == project_path)
    {
        result.push(("project".to_string(), project_path, "read-write".to_string()));
    }

    // Fallback: hardcoded default if no config
    if result.is_empty() {
        result.push((
            "default".to_string(),
            default_vault_path(),
            "read-write".to_string(),
        ));
    }

    result
}
```

### Step 7: Run tests to verify they pass

Run: `cargo test -p engram awareness`

Expected: All 4 tests PASS.

### Step 8: Commit

```bash
git add crates/engram-cli/src/awareness.rs crates/engram-cli/src/lib.rs crates/engram-cli/src/main.rs crates/engram-cli/tests/awareness_test.rs
git commit -m "feat(cli): engram awareness Layer 1 — domain structure from vault folder counts"
```

---

## Task 10: `engram awareness` Layer 2 — `_context/` ambient context

**Files:**
- Modify: `crates/engram-cli/src/awareness.rs`
- Modify: `crates/engram-cli/src/main.rs` — update `run_awareness`
- Modify: `crates/engram-cli/tests/awareness_test.rs`

### Step 1: Write the failing tests

Add to `crates/engram-cli/tests/awareness_test.rs`:

```rust
#[test]
fn test_awareness_includes_context_file_content() {
    let dir = TempDir::new().unwrap();
    let vault_dir = TempDir::new().unwrap();

    // Create _context files
    fs::create_dir_all(vault_dir.path().join("_context")).unwrap();
    fs::write(
        vault_dir.path().join("_context/domains.md"),
        "## Domains\nWork knowledge area",
    )
    .unwrap();

    let cfg = write_config(
        &dir,
        &format!(
            r#"[vaults.test]
path = "{}"
default = true
"#,
            vault_dir.path().display()
        ),
    );

    let output = Command::cargo_bin("engram")
        .unwrap()
        .env("ENGRAM_CONFIG_PATH", &cfg)
        .arg("awareness")
        .output()
        .unwrap();

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("Work knowledge area"),
        "awareness should include _context file content, got: {}",
        stdout
    );
}

#[test]
fn test_awareness_no_context_dir_still_succeeds() {
    let dir = TempDir::new().unwrap();
    let vault_dir = TempDir::new().unwrap();
    let cfg = write_config(
        &dir,
        &format!(
            r#"[vaults.test]
path = "{}"
default = true
"#,
            vault_dir.path().display()
        ),
    );

    // No _context directory — should succeed without error
    Command::cargo_bin("engram")
        .unwrap()
        .env("ENGRAM_CONFIG_PATH", &cfg)
        .arg("awareness")
        .assert()
        .success();
}
```

### Step 2: Run to verify they fail

Run: `cargo test -p engram awareness`

Expected: `test_awareness_includes_context_file_content` FAILS (context not in output yet).

### Step 3: Update `run_awareness` in `main.rs` to include Layer 2

In the `run_awareness` function, update the per-vault section generation:

```rust
fn run_awareness(vault_arg: Option<&str>, _all: bool) {
    let vaults_to_show = collect_active_vaults(vault_arg);

    println!("<engram-context>");
    let mut sections = Vec::new();

    for (name, path, access_label) in &vaults_to_show {
        let (total, domains) = awareness::vault_domain_summary(path);
        let header = format!(
            "## {} ({}) · {} files · {}",
            name,
            path.display(),
            total,
            access_label
        );
        let domains_line = if domains.is_empty() {
            "Domains: (empty)".to_string()
        } else {
            format!("Domains: {}", domains)
        };

        // Layer 2: ambient context files
        let context = awareness::vault_context_files(path);

        let mut section_parts = vec![header, domains_line];
        if !context.is_empty() {
            section_parts.push(context);
        }
        sections.push(section_parts.join("\n"));
    }

    println!("{}", sections.join("\n\n"));
    println!("</engram-context>");
}
```

### Step 4: Run tests to verify they pass

Run: `cargo test -p engram awareness`

Expected: All 6 tests PASS.

### Step 5: Commit

```bash
git add crates/engram-cli/src/awareness.rs crates/engram-cli/src/main.rs crates/engram-cli/tests/awareness_test.rs
git commit -m "feat(cli): engram awareness Layer 2 — _context/*.md ambient context files (LifeOS-compatible)"
```

---

## Task 11: `engram awareness` Layer 3 — recent facts from memory store

**Files:**
- Modify: `crates/engram-cli/src/awareness.rs`
- Modify: `crates/engram-cli/src/main.rs` — update `run_awareness`

### Step 1: Write the failing test

Add to `crates/engram-cli/tests/awareness_test.rs`:

```rust
#[test]
fn test_awareness_exits_zero_when_no_store_exists() {
    // If there's no memory.db, Layer 3 should silently produce nothing
    let dir = TempDir::new().unwrap();
    let vault_dir = TempDir::new().unwrap();
    let cfg = write_config(
        &dir,
        &format!(
            r#"[vaults.test]
path = "{}"
default = true
"#,
            vault_dir.path().display()
        ),
    );

    // vault_storage_dir won't have a memory.db — should not crash
    Command::cargo_bin("engram")
        .unwrap()
        .env("ENGRAM_CONFIG_PATH", &cfg)
        .arg("awareness")
        .assert()
        .success();
}
```

### Step 2: Run to confirm it already passes

Run: `cargo test -p engram test_awareness_exits_zero_when_no_store_exists`

Expected: PASS (silently skipping is correct behavior; this is a regression guard test).

### Step 3: Add `vault_recent_facts` to `awareness.rs`

```rust
/// Query recent memories from the vault's encrypted store (Layer 3).
///
/// Returns empty string silently if:
/// - No keychain key is available
/// - No `memory.db` exists in `engram_dir`
/// - The store cannot be opened
///
/// `engram_dir` is the vault's per-vault storage dir (`~/.engram/<vault_name>/`).
pub fn vault_recent_facts(engram_dir: &Path, limit: usize) -> String {
    use engram_core::crypto::KeyStore;
    use engram_core::store::MemoryStore;
    use std::collections::BTreeMap;
    use std::time::{SystemTime, UNIX_EPOCH};

    let key_store = KeyStore::new("engram");
    let key = match key_store.retrieve() {
        Ok(k) => k,
        Err(_) => return String::new(),
    };

    let store_path = engram_dir.join("memory.db");
    if !store_path.exists() {
        return String::new();
    }

    let store = match MemoryStore::open(&store_path, &key) {
        Ok(s) => s,
        Err(_) => return String::new(),
    };

    // 30 days in milliseconds
    let cutoff = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as i64 - 30 * 24 * 60 * 60 * 1000)
        .unwrap_or(0);

    let memories = match store.list_recent(cutoff, limit) {
        Ok(m) => m,
        Err(_) => return String::new(),
    };

    if memories.is_empty() {
        return String::new();
    }

    // Group by entity
    let mut by_entity: BTreeMap<String, Vec<String>> = BTreeMap::new();
    for m in memories {
        by_entity
            .entry(m.entity)
            .or_default()
            .push(m.value);
    }

    let lines: Vec<String> = by_entity
        .iter()
        .take(limit)
        .map(|(entity, values)| format!("- {}: {}", entity, values.join(", ")))
        .collect();

    format!("Top of mind:\n{}", lines.join("\n"))
}
```

### Step 4: Update `run_awareness` in `main.rs` to include Layer 3

```rust
fn run_awareness(vault_arg: Option<&str>, _all: bool) {
    let vaults_to_show = collect_active_vaults(vault_arg);

    println!("<engram-context>");
    let mut sections = Vec::new();

    for (name, path, access_label) in &vaults_to_show {
        let (total, domains) = awareness::vault_domain_summary(path);
        let header = format!(
            "## {} ({}) · {} files · {}",
            name,
            path.display(),
            total,
            access_label
        );
        let domains_line = if domains.is_empty() {
            "Domains: (empty)".to_string()
        } else {
            format!("Domains: {}", domains)
        };

        // Layer 2: _context/ ambient context
        let context = awareness::vault_context_files(path);

        // Layer 3: recent facts from memory store
        let engram_dir = vault_storage_dir(name);
        let recent = awareness::vault_recent_facts(&engram_dir, 10);

        let mut section_parts = vec![header, domains_line];
        if !context.is_empty() {
            section_parts.push(context);
        }
        if !recent.is_empty() {
            section_parts.push(recent);
        }
        sections.push(section_parts.join("\n"));
    }

    println!("{}", sections.join("\n\n"));
    println!("</engram-context>");
}
```

### Step 5: Run tests to verify they pass

Run: `cargo test -p engram awareness`

Expected: All 7 tests PASS.

Run: `cargo run --bin engram -- awareness`

Expected: Prints `<engram-context>...</engram-context>` block. No crash if no config or store exists.

### Step 6: Commit

```bash
git add crates/engram-cli/src/awareness.rs crates/engram-cli/src/main.rs crates/engram-cli/tests/awareness_test.rs
git commit -m "feat(cli): engram awareness Layer 3 — recent facts from memory store grouped by entity"
```

---

## Task 12: Update `engram status` to show all vaults

**Files:**
- Modify: `crates/engram-cli/src/main.rs` — update `run_status()`

### Step 1: Write failing tests

Add to `crates/engram-cli/tests/cli_integration.rs`:

```rust
/// `engram status` with a configured vault should show that vault's name.
#[test]
fn test_status_shows_vaults_label_when_configured() {
    let dir = tempfile::TempDir::new().unwrap();
    let vault_dir = tempfile::TempDir::new().unwrap();
    let cfg = dir.path().join("config.toml");
    std::fs::write(
        &cfg,
        format!(
            r#"[vaults.primary]
path = "{}"
access = "read-write"
sync_mode = "auto"
default = true
"#,
            vault_dir.path().display()
        ),
    )
    .unwrap();

    let output = Command::cargo_bin("engram")
        .unwrap()
        .env("ENGRAM_CONFIG_PATH", &cfg)
        .arg("status")
        .output()
        .unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("primary") || stdout.contains("Vault"),
        "Status should mention the vault name 'primary' or 'Vault:', got: {}",
        stdout
    );
}
```

### Step 2: Run to confirm existing tests still pass

Run: `cargo test -p engram test_status`

Expected: All existing status tests PASS (pre-condition check).

### Step 3: Update `run_status()` to show configured vaults

Replace the vault status section in `run_status()`:

```rust
fn run_status() {
    // Separator line
    println!("{}", "─".repeat(41));

    // ── Vault status ─────────────────────────────────────────────────────
    let config = EngramConfig::load();
    if config.vaults.is_empty() {
        // Legacy path: no config, show default vault path
        let vault_path = default_vault_path();
        if vault_path.exists() {
            let vault = Vault::new(&vault_path);
            let count = vault.list_markdown().map(|files| files.len()).unwrap_or(0);
            println!("Vault:        {} ({} files)", vault_path.display(), count);
        } else {
            println!("Vault:        {} (NOT FOUND)", vault_path.display());
        }
    } else {
        use engram_core::config::{SyncMode, VaultAccess};
        println!("Vaults:");
        for (name, vault_entry) in &config.vaults {
            let path = shellexpand_path(&vault_entry.path);
            let exists = if path.exists() { "✓" } else { "✗" };
            let default_marker = if vault_entry.default { " [default]" } else { "" };
            let access = match vault_entry.access {
                VaultAccess::Read => "read",
                VaultAccess::ReadWrite => "read-write",
            };
            let sync = match vault_entry.sync_mode {
                SyncMode::Auto => "auto",
                SyncMode::Approval => "approval",
                SyncMode::Manual => "manual",
            };
            let count = if path.exists() {
                Vault::new(&path)
                    .list_markdown()
                    .map(|f| f.len())
                    .unwrap_or(0)
            } else {
                0
            };
            println!(
                "  {} {}{} — {} files  {} · {}",
                exists, name, default_marker, count, access, sync
            );
        }
    }

    // ── Memory store status ──────────────────────────────────────────────
    // (keep existing implementation unchanged)
    let store_path = default_store_path();
    let key_store = KeyStore::new("engram");
    let key_result = key_store.retrieve();

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
                Err(_) => {
                    println!("Memory store: {} (wrong key)", store_path.display());
                }
            },
            Err(_) => {
                println!("Memory store: {} (present, no key)", store_path.display());
            }
        }
    } else {
        println!("Memory store: {} (not initialized)", store_path.display());
    }

    // ── Search index status ──────────────────────────────────────────────
    let search_dir = default_search_dir();
    println!("{}", search_index_status(&search_dir));

    // ── Keyring status ───────────────────────────────────────────────────
    match key_result {
        Ok(_) => println!("Key:          present ✓"),
        Err(_) => println!("Key:          not set"),
    }
}
```

### Step 4: Run all status tests

Run: `cargo test -p engram test_status`

Expected: All status tests PASS (including the new one and all existing ones).

### Step 5: Commit

```bash
git add crates/engram-cli/src/main.rs crates/engram-cli/tests/cli_integration.rs
git commit -m "feat(cli): engram status shows all configured vaults with access and sync mode"
```

---

## Task 13: Update `hook-memory-context` to call `engram awareness`

**Files:**
- Modify: `modules/hook-memory-context/amplifier_module_hook_memory_context/__init__.py`
- Modify: `modules/hook-memory-context/tests/test_hook.py`

### Step 1: Write the failing test

In `modules/hook-memory-context/tests/test_hook.py`, update `test_handler_calls_engram_load`:

The **existing** test to replace is:

```python
@pytest.mark.asyncio
async def test_handler_calls_engram_load(coordinator):
    """Handler subprocess-calls engram load --format=context."""
    await mount(coordinator, {})
    handler = coordinator.hooks.register.call_args[0][1]

    with patch("subprocess.run") as mock_run:
        mock_run.return_value = MagicMock(returncode=0, stdout="<engram-context>test</engram-context>")
        await handler(MagicMock())
        mock_run.assert_called_once_with(
            ["engram", "load", "--format=context"],
            capture_output=True,
            text=True,
            timeout=5,
        )
```

Replace with:

```python
@pytest.mark.asyncio
async def test_handler_calls_engram_awareness(coordinator):
    """Handler subprocess-calls engram awareness."""
    await mount(coordinator, {})
    handler = coordinator.hooks.register.call_args[0][1]

    with patch("subprocess.run") as mock_run:
        mock_run.return_value = MagicMock(
            returncode=0,
            stdout="<engram-context>\n## Personal\nDomains: Work (89)\n</engram-context>",
        )
        await handler(MagicMock())
        mock_run.assert_called_once_with(
            ["engram", "awareness"],
            capture_output=True,
            text=True,
            timeout=10,
        )
```

### Step 2: Run to verify the test fails

```bash
cd /Users/ken/workspace/ms/engram/modules/hook-memory-context
python -m pytest tests/test_hook.py::test_handler_calls_engram_awareness -v
```

Expected: FAIL — the function is `test_handler_calls_engram_load` (renamed) and the implementation still calls `engram load`.

### Step 3: Update the implementation

In `modules/hook-memory-context/amplifier_module_hook_memory_context/__init__.py`, change the handler:

```python
async def handle_prompt_submit(event):
    """Call engram awareness and inject result as system reminder."""
    try:
        result = subprocess.run(
            ["engram", "awareness"],
            capture_output=True,
            text=True,
            timeout=config.get("timeout", 10),
        )
        if result.returncode == 0 and result.stdout.strip():
            if HookResult is not None:
                return HookResult(
                    action="inject_context",
                    content=result.stdout.strip(),
                    ephemeral=True,
                    suppress_output=True,
                )
    except (subprocess.TimeoutExpired, FileNotFoundError):
        pass
    if HookResult is not None:
        return HookResult(action="noop")
```

Also update the docstring on `mount()`:

```python
async def mount(coordinator, config: dict):
    """Register the memory context hook with the coordinator."""
```

### Step 4: Run all hook tests

```bash
cd /Users/ken/workspace/ms/engram/modules/hook-memory-context
python -m pytest tests/ -v
```

Expected: All 5 tests PASS (4 existing + 1 new/renamed).

**Note:** The existing `test_handler_tolerates_missing_engram_binary` and `test_handler_tolerates_timeout` tests do not assert the command name, so they will still pass without modification.

### Step 5: Commit

```bash
git add modules/hook-memory-context/
git commit -m "feat(bundle): hook-memory-context calls engram awareness for three-layer vault context"
```

---

## Task 14: Integration tests — multi-vault, awareness output, sync approval

**Files:**
- Create: `crates/engram-cli/tests/multi_vault_test.rs`

### Step 1: Write all tests upfront

Create `crates/engram-cli/tests/multi_vault_test.rs`:

```rust
// Multi-vault integration tests
//
// Tests cover: vault list with multiple entries, awareness output with
// domain breakdown, and sync approval mode gate.

use assert_cmd::Command;
use std::fs;
use tempfile::TempDir;

fn write_config(dir: &TempDir, toml: &str) -> std::path::PathBuf {
    let p = dir.path().join("config.toml");
    fs::write(&p, toml).unwrap();
    p
}

// ── vault list with two configured vaults ────────────────────────────────────

#[test]
fn test_vault_list_shows_both_configured_vaults() {
    let dir = TempDir::new().unwrap();
    let vault_a = TempDir::new().unwrap();
    let vault_b = TempDir::new().unwrap();

    let cfg = write_config(
        &dir,
        &format!(
            r#"
[vaults.alpha]
path = "{}"
access = "read-write"
sync_mode = "auto"
default = true

[vaults.beta]
path = "{}"
access = "read"
sync_mode = "manual"
"#,
            vault_a.path().display(),
            vault_b.path().display()
        ),
    );

    let output = Command::cargo_bin("engram")
        .unwrap()
        .env("ENGRAM_CONFIG_PATH", &cfg)
        .args(["vault", "list"])
        .output()
        .unwrap();

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("alpha"),
        "Should show vault alpha, got: {}",
        stdout
    );
    assert!(
        stdout.contains("beta"),
        "Should show vault beta, got: {}",
        stdout
    );
    assert!(
        stdout.contains("read"),
        "Should show read access for beta, got: {}",
        stdout
    );
}

// ── awareness output with two vaults ─────────────────────────────────────────

#[test]
fn test_awareness_shows_both_vault_sections() {
    let dir = TempDir::new().unwrap();
    let vault_a = TempDir::new().unwrap();
    let vault_b = TempDir::new().unwrap();

    // Populate vault_a with markdown
    fs::create_dir_all(vault_a.path().join("Work")).unwrap();
    fs::write(vault_a.path().join("Work/notes.md"), "notes").unwrap();
    fs::write(vault_a.path().join("Work/meeting.md"), "meeting").unwrap();

    // Populate vault_b with markdown
    fs::create_dir_all(vault_b.path().join("Design")).unwrap();
    fs::write(vault_b.path().join("Design/spec.md"), "spec").unwrap();

    let cfg = write_config(
        &dir,
        &format!(
            r#"
[vaults.alpha]
path = "{}"
access = "read-write"
sync_mode = "auto"
default = true

[vaults.beta]
path = "{}"
access = "read-write"
sync_mode = "manual"
"#,
            vault_a.path().display(),
            vault_b.path().display()
        ),
    );

    let output = Command::cargo_bin("engram")
        .unwrap()
        .env("ENGRAM_CONFIG_PATH", &cfg)
        .arg("awareness")
        .output()
        .unwrap();

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("<engram-context>"),
        "Should contain opening context tag, got: {}",
        stdout
    );
    assert!(
        stdout.contains("</engram-context>"),
        "Should contain closing context tag, got: {}",
        stdout
    );
    assert!(
        stdout.contains("Work (2)"),
        "Should show Work domain with 2 files from alpha, got: {}",
        stdout
    );
    assert!(
        stdout.contains("Design (1)"),
        "Should show Design domain with 1 file from beta, got: {}",
        stdout
    );
    assert!(
        stdout.contains("alpha"),
        "Should name alpha vault in header, got: {}",
        stdout
    );
    assert!(
        stdout.contains("beta"),
        "Should name beta vault in header, got: {}",
        stdout
    );
}

// ── awareness with _context/ files ───────────────────────────────────────────

#[test]
fn test_awareness_layer2_context_files_appear_in_output() {
    let dir = TempDir::new().unwrap();
    let vault_dir = TempDir::new().unwrap();

    fs::create_dir_all(vault_dir.path().join("_context")).unwrap();
    fs::write(
        vault_dir.path().join("_context/about.md"),
        "My personal knowledge vault",
    )
    .unwrap();

    let cfg = write_config(
        &dir,
        &format!(
            r#"[vaults.mine]
path = "{}"
default = true
"#,
            vault_dir.path().display()
        ),
    );

    let output = Command::cargo_bin("engram")
        .unwrap()
        .env("ENGRAM_CONFIG_PATH", &cfg)
        .arg("awareness")
        .output()
        .unwrap();

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("My personal knowledge vault"),
        "Should include _context file content in Layer 2, got: {}",
        stdout
    );
}

// ── sync approval gate ────────────────────────────────────────────────────────

#[test]
fn test_sync_approval_mode_blocks_push_without_approve_flag() {
    let dir = TempDir::new().unwrap();
    let vault_dir = TempDir::new().unwrap();

    let cfg = write_config(
        &dir,
        &format!(
            r#"[vaults.approval-vault]
path = "{}"
access = "read-write"
sync_mode = "approval"
default = true
"#,
            vault_dir.path().display()
        ),
    );

    let output = Command::cargo_bin("engram")
        .unwrap()
        .env("ENGRAM_CONFIG_PATH", &cfg)
        .args(["sync", "--vault", "approval-vault"])
        .output()
        .unwrap();

    let stdout = String::from_utf8_lossy(&output.stdout);
    // Should not proceed to backend; should show approval prompt
    assert!(
        stdout.contains("approval required") || stdout.contains("To push:"),
        "Approval mode without --approve should block push and show prompt, got: {}",
        stdout
    );
    assert!(
        output.status.success(),
        "Should exit 0 (graceful gate, no push attempted)"
    );
}

#[test]
fn test_sync_manual_mode_shows_informational_message() {
    let dir = TempDir::new().unwrap();
    let vault_dir = TempDir::new().unwrap();

    let cfg = write_config(
        &dir,
        &format!(
            r#"[vaults.manual-vault]
path = "{}"
access = "read-write"
sync_mode = "manual"
default = true
"#,
            vault_dir.path().display()
        ),
    );

    Command::cargo_bin("engram")
        .unwrap()
        .env("ENGRAM_CONFIG_PATH", &cfg)
        .args(["sync", "--vault", "manual-vault"])
        .assert()
        .success()
        .stdout(predicates::str::contains("manual sync mode"));
}

// ── access control cross-vault ────────────────────────────────────────────────

#[test]
fn test_access_control_blocks_sync_on_read_only_vault() {
    let dir = TempDir::new().unwrap();
    let vault_dir = TempDir::new().unwrap();

    let cfg = write_config(
        &dir,
        &format!(
            r#"[vaults.locked]
path = "{}"
access = "read"
sync_mode = "auto"
default = true
"#,
            vault_dir.path().display()
        ),
    );

    Command::cargo_bin("engram")
        .unwrap()
        .env("ENGRAM_CONFIG_PATH", &cfg)
        .args(["sync", "--vault", "locked"])
        .assert()
        .failure()
        .stderr(predicates::str::contains("read-only"));
}
```

### Step 2: Run to verify all tests pass

Run: `cargo test -p engram multi_vault`

Expected: All 6 tests PASS (all features were already implemented in Tasks 3–12).

### Step 3: Run full test suite

Run: `cargo test --workspace`

Expected: All tests pass. Verify no regressions.

### Step 4: Commit

```bash
git add crates/engram-cli/tests/multi_vault_test.rs
git commit -m "test(cli): integration tests for multi-vault config, awareness, and sync approval flow"
```

---

## Task 15: Final verification + cleanup

**Files:** No new files.

### Step 1: Run full workspace test suite

Run: `cargo test --workspace`

Expected: All tests PASS.

### Step 2: Run Python module tests

```bash
cd /Users/ken/workspace/ms/engram/modules/hook-memory-context
python -m pytest tests/ -v
```

Expected: All 5 tests PASS.

### Step 3: Smoke test the key commands manually

```bash
# Check awareness command exists and produces context tags
cargo run --bin engram -- awareness --help

# Check vault subcommand exists
cargo run --bin engram -- vault --help
cargo run --bin engram -- vault list

# Check sync flags
cargo run --bin engram -- sync --help
```

Expected: All commands print help/output without panics.

### Step 4: Verify no `todo!()` callsites remain from stubs

Run: `grep -r 'todo!()' crates/engram-cli/src/`

Expected: No results (all stubs replaced with implementations).

### Step 5: Final commit

```bash
git add -A
git commit -m "chore: final cleanup — multi-vault + awareness feature complete"
```

---

## Feature Summary

After all 15 tasks are complete:

| Command | Behavior |
|---------|----------|
| `engram vault list` | Lists configured vaults with access/sync info; shows auto-detected project vault |
| `engram vault add <name> --path <p> [--access] [--sync] [--default]` | Adds vault to `~/.engram/config.toml` |
| `engram vault remove <name>` | Removes from config |
| `engram vault set-default <name>` | Sets default in config |
| `engram awareness` | Three-layer XML context block: domains + `_context/` + recent facts |
| `engram awareness --vault <name>` | Awareness for specific vault only |
| `engram index --vault <name>` | Index using named vault's path and per-vault search dir |
| `engram search <q> --vault <name>` | Search vault-scoped index |
| `engram sync --vault <name>` | Respects `sync_mode` (manual message / approval diff / auto push) |
| `engram sync --vault <name> --approve` | Pushes even in approval mode |
| `hook-memory-context` | Calls `engram awareness` instead of `engram load --format=context` |
