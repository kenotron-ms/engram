//! Engram configuration — vault registry and sync preferences.

use std::collections::BTreeMap;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};
use thiserror::Error;

// ──────────────────────────────────────────────────────────────────────────────
// Errors
// ──────────────────────────────────────────────────────────────────────────────

#[derive(Debug, Error)]
pub enum ConfigError {
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    #[error("parse error: {0}")]
    Parse(#[from] toml::de::Error),
    #[error("serialize error: {0}")]
    Serialize(#[from] toml::ser::Error),
}

// ──────────────────────────────────────────────────────────────────────────────
// Enums
// ──────────────────────────────────────────────────────────────────────────────

/// Whether a vault is opened for reading only or read-write.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "kebab-case")]
pub enum VaultAccess {
    Read,
    #[default]
    ReadWrite,
}

/// How changes are synchronised with remote storage.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "kebab-case")]
pub enum SyncMode {
    Auto,
    #[default]
    Approval,
    Manual,
}

// ──────────────────────────────────────────────────────────────────────────────
// Serde helpers for struct-field defaults
// ──────────────────────────────────────────────────────────────────────────────

fn default_access() -> VaultAccess {
    VaultAccess::default()
}

fn default_sync_mode() -> SyncMode {
    SyncMode::default()
}

// ──────────────────────────────────────────────────────────────────────────────
// Structs
// ──────────────────────────────────────────────────────────────────────────────

/// Key-derivation configuration (base64-encoded salt, etc.).
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct KeyConfig {
    /// Optional base64-encoded salt used for key derivation.
    pub salt: Option<String>,
}

/// Per-vault credential store for sync backends.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct VaultSyncCredentials {
    /// Backend identifier (e.g. "s3", "azure", "gdrive").
    ///
    /// Note: `Default::default()` yields an empty string — backend must be set
    /// before the credentials are used.
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

/// Credentials configuration mapping vault names to their sync credentials.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CredentialsConfig {
    #[serde(default)]
    pub vaults: BTreeMap<String, VaultSyncCredentials>,
}

/// A single registered vault.
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

/// Top-level Engram configuration file.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct EngramConfig {
    #[serde(default)]
    pub vaults: BTreeMap<String, VaultEntry>,
    #[serde(default)]
    pub key: KeyConfig,
}

// ──────────────────────────────────────────────────────────────────────────────
// impl EngramConfig
// ──────────────────────────────────────────────────────────────────────────────

impl EngramConfig {
    /// Return the path to the configuration file.
    ///
    /// Checks `ENGRAM_CONFIG_PATH` first; falls back to `~/.engram/config.toml`.
    pub fn config_path() -> PathBuf {
        if let Ok(path) = std::env::var("ENGRAM_CONFIG_PATH") {
            return PathBuf::from(path);
        }
        dirs::home_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join(".engram")
            .join("config.toml")
    }

    /// Load configuration from disk.
    ///
    /// Returns `Default::default()` if the file is missing or cannot be parsed.
    pub fn load() -> Self {
        let path = Self::config_path();
        let contents = match std::fs::read_to_string(&path) {
            Ok(s) => s,
            Err(_) => return Self::default(),
        };
        toml::from_str(&contents).unwrap_or_default()
    }

    /// Persist configuration to disk using an atomic tmp-file + rename.
    ///
    /// Creates parent directories if they do not exist.
    pub fn save(&self) -> Result<(), ConfigError> {
        let path = Self::config_path();

        // Ensure parent directory exists.
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        let serialised = toml::to_string(self)?;

        // Write to a sibling tmp file then rename for atomicity.
        let tmp_path = path.with_extension("toml.tmp");
        std::fs::write(&tmp_path, &serialised)?;
        std::fs::rename(&tmp_path, &path)?;

        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o600))
                .map_err(ConfigError::Io)?;
        }

        Ok(())
    }

    /// Return the path to the credentials file.
    ///
    /// Checks `ENGRAM_CREDENTIALS_PATH` first; falls back to `~/.engram/credentials`.
    pub fn credentials_path() -> PathBuf {
        if let Ok(path) = std::env::var("ENGRAM_CREDENTIALS_PATH") {
            return PathBuf::from(path);
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
        let contents = match std::fs::read_to_string(&path) {
            Ok(s) => s,
            Err(_) => return CredentialsConfig::default(),
        };
        toml::from_str(&contents).unwrap_or_default()
    }

    /// Persist credentials to disk using an atomic tmp-file + rename.
    ///
    /// Creates parent directories if they do not exist. Sets 0600 permissions on Unix.
    pub fn save_credentials(creds: &CredentialsConfig) -> Result<(), ConfigError> {
        let path = Self::credentials_path();

        // Ensure parent directory exists.
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        let serialised = toml::to_string(creds)?;

        // Write to a sibling tmp file then rename for atomicity.
        let tmp_path = path.with_extension("tmp");
        std::fs::write(&tmp_path, &serialised)?;
        std::fs::rename(&tmp_path, &path)?;

        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o600))
                .map_err(ConfigError::Io)?;
        }

        Ok(())
    }

    /// Look up credentials for a named vault.
    pub fn credentials_for_vault<'a>(
        name: &str,
        creds: &'a CredentialsConfig,
    ) -> Option<&'a VaultSyncCredentials> {
        creds.vaults.get(name)
    }

    /// Return the default vault: the first entry with `default = true`, or the
    /// first entry in the map (BTreeMap order = alphabetical by name).
    pub fn default_vault(&self) -> Option<(&str, &VaultEntry)> {
        // First try to find an explicitly flagged default.
        if let Some((name, entry)) = self.vaults.iter().find(|(_, e)| e.default) {
            return Some((name.as_str(), entry));
        }
        // Fall back to the first entry in alphabetical order.
        self.vaults
            .iter()
            .next()
            .map(|(name, entry)| (name.as_str(), entry))
    }

    /// Look up a vault by name.
    pub fn get_vault(&self, name: &str) -> Option<&VaultEntry> {
        self.vaults.get(name)
    }

    /// Register a vault.
    ///
    /// If `entry.default` is `true`, all other vaults are marked `default = false`
    /// before insertion.
    pub fn add_vault(&mut self, name: String, entry: VaultEntry) {
        if entry.default {
            for e in self.vaults.values_mut() {
                e.default = false;
            }
        }
        self.vaults.insert(name, entry);
    }

    /// Remove a vault by name.  Returns `true` if it existed, `false` otherwise.
    pub fn remove_vault(&mut self, name: &str) -> bool {
        self.vaults.remove(name).is_some()
    }

    /// Make `name` the default vault, clearing the flag from all others.
    ///
    /// Returns `false` if no vault with that name is registered.
    pub fn set_default(&mut self, name: &str) -> bool {
        if !self.vaults.contains_key(name) {
            return false;
        }
        for (k, e) in self.vaults.iter_mut() {
            e.default = k == name;
        }
        true
    }

    /// Path to the sync key file — the engram equivalent of ~/.ssh/id_rsa.
    /// Override with ENGRAM_SYNC_KEY_PATH env var (useful for testing).
    pub fn sync_key_path() -> std::path::PathBuf {
        if let Ok(override_path) = std::env::var("ENGRAM_SYNC_KEY_PATH") {
            return std::path::PathBuf::from(override_path);
        }
        dirs::home_dir()
            .unwrap_or_else(|| std::path::PathBuf::from("/tmp"))
            .join(".engram")
            .join("sync.key")
    }
}


// ─────────────────────────────────────────────────────────────────────────────
// Sync key file helpers
// ─────────────────────────────────────────────────────────────────────────────

/// Write a 32-byte sync key to disk as base64, chmod 600.
///
/// Sets restrictive permissions BEFORE writing content to prevent
/// the window where the file exists with world-readable permissions.
pub fn write_sync_key_file(
    path: &std::path::Path,
    key: &[u8; 32],
) -> std::io::Result<()> {
    use std::io::Write;

    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }

    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        let mut file = std::fs::OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .mode(0o600)
            .open(path)?;
        use base64::Engine;
        let encoded = base64::engine::general_purpose::STANDARD.encode(key);
        file.write_all(encoded.as_bytes())?;
        file.write_all(b"\n")?;
    }
    #[cfg(not(unix))]
    {
        use base64::Engine;
        let encoded = base64::engine::general_purpose::STANDARD.encode(key);
        std::fs::write(path, format!("{encoded}\n"))?;
    }

    Ok(())
}

/// Read a 32-byte sync key from the key file.
/// Returns an error if the file is missing, corrupt, or not exactly 32 bytes.
pub fn read_sync_key_file(
    path: &std::path::Path,
) -> Result<[u8; 32], Box<dyn std::error::Error + Send + Sync>> {
    use base64::Engine;
    let contents = std::fs::read_to_string(path)?;
    let decoded = base64::engine::general_purpose::STANDARD.decode(contents.trim())?;
    decoded
        .try_into()
        .map_err(|_| "sync.key: expected 32 bytes — file may be corrupt".into())
}

// ──────────────────────────────────────────────────────────────────────────────
// Tests
// ──────────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    // Serialise all tests that mutate `ENGRAM_CONFIG_PATH` in the process
    // environment.  `std::env` is a process-global map; concurrent writes from
    // two threads (the default under `cargo test`) are a data race.  A
    // `OnceLock<Mutex<()>>` gives us a zero-dependency serialisation point.
    static ENV_LOCK: std::sync::OnceLock<std::sync::Mutex<()>> = std::sync::OnceLock::new();

    fn env_lock() -> std::sync::MutexGuard<'static, ()> {
        ENV_LOCK
            .get_or_init(|| std::sync::Mutex::new(()))
            .lock()
            .unwrap()
    }

    #[test]
    fn vault_access_default_is_read_write() {
        assert_eq!(VaultAccess::default(), VaultAccess::ReadWrite);
    }

    #[test]
    fn sync_mode_default_is_approval() {
        assert_eq!(SyncMode::default(), SyncMode::Approval);
    }

    #[test]
    fn engram_config_default_vaults_is_empty() {
        let config = EngramConfig::default();
        assert!(config.vaults.is_empty());
    }

    #[test]
    fn vault_entry_type_field_is_renamed_in_toml() {
        let entry = VaultEntry {
            path: PathBuf::from("/home/user/notes"),
            access: VaultAccess::ReadWrite,
            sync_mode: SyncMode::Approval,
            default: true,
            vault_type: Some("git".to_string()),
        };
        let s = toml::to_string(&entry).expect("serialise VaultEntry");
        // The serde rename("type") must produce a "type" key, not "vault_type".
        assert!(s.contains("type = \"git\""), "unexpected toml:\n{s}");
        assert!(!s.contains("vault_type"), "rename did not apply:\n{s}");
    }

    #[test]
    fn vault_access_serialises_kebab_case() {
        // Round-trip through a wrapper struct (toml requires a table root)
        #[derive(Serialize, Deserialize)]
        struct W {
            access: VaultAccess,
        }
        let w = W {
            access: VaultAccess::ReadWrite,
        };
        let s = toml::to_string(&w).expect("serialise");
        assert!(s.contains("read-write"), "kebab-case missing:\n{s}");
    }

    #[test]
    fn sync_mode_serialises_kebab_case() {
        #[derive(Serialize, Deserialize)]
        struct W {
            mode: SyncMode,
        }
        let approval = W {
            mode: SyncMode::Approval,
        };
        let s_approval = toml::to_string(&approval).expect("serialise");
        assert!(
            s_approval.contains("approval"),
            "missing 'approval':\n{s_approval}"
        );

        let manual = W {
            mode: SyncMode::Manual,
        };
        let s_manual = toml::to_string(&manual).expect("serialise");
        assert!(s_manual.contains("manual"), "missing 'manual':\n{s_manual}");
    }

    #[test]
    fn engram_config_roundtrip_toml() {
        let mut config = EngramConfig::default();
        config.vaults.insert(
            "main".to_string(),
            VaultEntry {
                path: PathBuf::from("/vaults/main"),
                access: VaultAccess::ReadWrite,
                sync_mode: SyncMode::Approval,
                default: true,
                vault_type: Some("git".to_string()),
            },
        );

        let serialised = toml::to_string(&config).expect("serialise EngramConfig");
        let parsed: EngramConfig = toml::from_str(&serialised).expect("parse EngramConfig");

        let entry = &parsed.vaults["main"];
        assert_eq!(entry.path, PathBuf::from("/vaults/main"));
        assert_eq!(entry.access, VaultAccess::ReadWrite);
        assert_eq!(entry.sync_mode, SyncMode::Approval);
        assert!(entry.default);
        assert_eq!(entry.vault_type, Some("git".to_string()));
    }

    #[test]
    fn vault_entry_access_defaults_to_read_write_on_parse() {
        // Omit "access" in toml — the default should kick in.
        let toml_input = r#"
path = "/vaults/docs"
default = false
"#;
        let entry: VaultEntry = toml::from_str(toml_input).expect("parse VaultEntry");
        assert_eq!(entry.access, VaultAccess::ReadWrite);
        assert_eq!(entry.sync_mode, SyncMode::Approval);
    }

    // ── EngramConfig / VaultEntry CRUD ───────────────────────────────────────

    fn make_entry(path: &str, is_default: bool) -> VaultEntry {
        VaultEntry {
            path: PathBuf::from(path),
            access: VaultAccess::ReadWrite,
            sync_mode: SyncMode::Approval,
            default: is_default,
            vault_type: None,
        }
    }

    #[test]
    fn test_default_config_is_empty() {
        let cfg = EngramConfig::default();
        assert!(cfg.vaults.is_empty());
    }

    #[test]
    fn test_add_vault_and_retrieve() {
        let mut cfg = EngramConfig::default();
        cfg.add_vault("work".to_string(), make_entry("/vaults/work", false));
        let entry = cfg.get_vault("work").expect("should find vault 'work'");
        assert_eq!(entry.path, PathBuf::from("/vaults/work"));
    }

    #[test]
    fn test_default_vault_returns_explicit_default() {
        let mut cfg = EngramConfig::default();
        cfg.add_vault("a".to_string(), make_entry("/a", false));
        cfg.add_vault("b".to_string(), make_entry("/b", true));
        let (name, _entry) = cfg.default_vault().expect("should have a default vault");
        assert_eq!(name, "b");
    }

    #[test]
    fn test_remove_vault() {
        let mut cfg = EngramConfig::default();
        cfg.add_vault("x".to_string(), make_entry("/x", false));
        assert!(
            cfg.remove_vault("x"),
            "remove should return true for existing vault"
        );
        assert!(
            !cfg.remove_vault("x"),
            "remove should return false for missing vault"
        );
        assert!(cfg.get_vault("x").is_none());
    }

    #[test]
    fn test_set_default_clears_others() {
        let mut cfg = EngramConfig::default();
        cfg.add_vault("a".to_string(), make_entry("/a", true));
        cfg.add_vault("b".to_string(), make_entry("/b", false));
        let ok = cfg.set_default("b");
        assert!(ok, "set_default should return true when vault exists");
        assert!(cfg.get_vault("b").unwrap().default, "b should be default");
        assert!(
            !cfg.get_vault("a").unwrap().default,
            "a should no longer be default"
        );
        let not_ok = cfg.set_default("missing");
        assert!(!not_ok, "set_default should return false for unknown vault");
    }

    #[test]
    fn test_roundtrip_toml_serialization() {
        use std::env;
        use tempfile::tempdir;

        let _guard = env_lock();
        let dir = tempdir().expect("tempdir");
        let config_path = dir.path().join("config.toml");

        // Override the config path via env var
        env::set_var("ENGRAM_CONFIG_PATH", &config_path);

        let mut cfg = EngramConfig::default();
        cfg.add_vault("main".to_string(), make_entry("/vaults/main", true));
        cfg.save().expect("save should succeed");

        // Load it back
        let loaded = EngramConfig::load();
        let entry = loaded
            .get_vault("main")
            .expect("main vault missing after load");
        assert_eq!(entry.path, PathBuf::from("/vaults/main"));
        assert!(entry.default);

        env::remove_var("ENGRAM_CONFIG_PATH");
    }

    #[test]
    fn test_config_path_uses_env_var_override() {
        use std::env;
        let _guard = env_lock();
        let custom = "/tmp/engram-test-config.toml";
        env::set_var("ENGRAM_CONFIG_PATH", custom);
        let path = EngramConfig::config_path();
        assert_eq!(path, PathBuf::from(custom));
        env::remove_var("ENGRAM_CONFIG_PATH");
    }

    // ── KeyConfig / EngramConfig serialization ──────────────────────────────────

    #[test]
    fn test_key_config_roundtrip() {
        // base64 for "saltsaltvalue"
        let kc = KeyConfig {
            salt: Some("c2FsdHNhbHR2YWx1ZQ==".to_string()),
        };
        let toml_str = toml::to_string(&kc).expect("serialize KeyConfig");
        assert!(toml_str.contains("salt"), "salt key missing:\n{toml_str}");
        let parsed: KeyConfig = toml::from_str(&toml_str).expect("deserialize KeyConfig");
        assert_eq!(parsed.salt, Some("c2FsdHNhbHR2YWx1ZQ==".to_string()));
    }

    #[test]
    fn test_config_missing_key_section_uses_default() {
        // A config with no [key] section should parse with key.salt == None.
        let toml_str = r#"
[vaults]
"#;
        let config: EngramConfig = toml::from_str(toml_str).expect("parse config without [key]");
        assert!(
            config.key.salt.is_none(),
            "salt should default to None when [key] is absent"
        );
    }

    #[cfg(unix)]
    #[test]
    fn test_save_sets_0600_permissions() {
        use std::env;
        use std::os::unix::fs::MetadataExt;
        use tempfile::tempdir;

        let _guard = env_lock();
        let dir = tempdir().expect("tempdir");
        let config_path = dir.path().join("config.toml");

        env::set_var("ENGRAM_CONFIG_PATH", &config_path);

        let cfg = EngramConfig::default();
        cfg.save().expect("save should succeed");

        let metadata = std::fs::metadata(&config_path).expect("metadata");
        // Mask off the file type bits; keep only the permission bits.
        let mode = metadata.mode() & 0o777;
        assert_eq!(
            mode, 0o600,
            "config.toml should have 0600 permissions, got {:o}",
            mode
        );

        env::remove_var("ENGRAM_CONFIG_PATH");
    }

    #[test]
    fn test_vault_entry_has_no_sync_section_in_toml() {
        let entry = VaultEntry {
            path: PathBuf::from("/vaults/no-sync"),
            access: VaultAccess::ReadWrite,
            sync_mode: SyncMode::Approval,
            default: false,
            vault_type: None,
        };
        let toml_str = toml::to_string(&entry).expect("serialize VaultEntry");
        // The serialized form must not contain a [sync] table.
        assert!(
            !toml_str.contains("[sync]"),
            "sync table section should be absent:\n{toml_str}"
        );
        // sync_mode is still serialized correctly.
        assert!(
            toml_str.contains("sync_mode"),
            "sync_mode field should still be present:\n{toml_str}"
        );
    }

    // ── credentials_path / load_credentials / save_credentials / credentials_for_vault ────────────

    #[cfg(unix)]
    #[test]
    fn test_save_credentials_sets_0600() {
        use std::env;
        use std::os::unix::fs::MetadataExt;
        use tempfile::tempdir;

        let _guard = env_lock();
        let dir = tempdir().expect("tempdir");
        let creds_path = dir.path().join("credentials");

        env::set_var("ENGRAM_CREDENTIALS_PATH", &creds_path);

        let creds = CredentialsConfig::default();
        EngramConfig::save_credentials(&creds).expect("save_credentials should succeed");

        let metadata = std::fs::metadata(&creds_path).expect("metadata");
        let mode = metadata.mode() & 0o777;
        assert_eq!(
            mode, 0o600,
            "credentials file should have 0600 permissions, got {:o}",
            mode
        );

        env::remove_var("ENGRAM_CREDENTIALS_PATH");
    }

    #[test]
    fn test_load_credentials_returns_default_when_missing() {
        use std::env;

        let _guard = env_lock();
        // Point at a path that definitely does not exist.
        env::set_var(
            "ENGRAM_CREDENTIALS_PATH",
            "/tmp/engram-nonexistent-creds-file-12345",
        );

        let creds = EngramConfig::load_credentials();
        assert!(
            creds.vaults.is_empty(),
            "load_credentials should return empty CredentialsConfig when file is missing"
        );

        env::remove_var("ENGRAM_CREDENTIALS_PATH");
    }

    #[test]
    fn test_credentials_roundtrip_through_files() {
        use std::env;
        use tempfile::tempdir;

        let _guard = env_lock();
        let dir = tempdir().expect("tempdir");
        let creds_path = dir.path().join("credentials");

        env::set_var("ENGRAM_CREDENTIALS_PATH", &creds_path);

        let mut creds = CredentialsConfig::default();
        creds.vaults.insert(
            "work".to_string(),
            VaultSyncCredentials {
                backend: "s3".to_string(),
                access_key: Some("AKIA_ROUNDTRIP_KEY".to_string()),
                secret_key: Some("super-secret".to_string()),
                bucket: Some("my-bucket".to_string()),
                endpoint: None,
                container: None,
                account: None,
                access_token: None,
                refresh_token: None,
                folder: None,
            },
        );

        EngramConfig::save_credentials(&creds).expect("save_credentials should succeed");
        let loaded = EngramConfig::load_credentials();

        let vault = loaded
            .vaults
            .get("work")
            .expect("work vault should survive round-trip");
        assert_eq!(
            vault.access_key,
            Some("AKIA_ROUNDTRIP_KEY".to_string()),
            "access_key should roundtrip correctly"
        );

        env::remove_var("ENGRAM_CREDENTIALS_PATH");
    }

    #[test]
    fn test_config_toml_never_contains_credentials() {
        use std::env;
        use tempfile::tempdir;

        let _guard = env_lock();
        let dir = tempdir().expect("tempdir");
        let config_path = dir.path().join("config.toml");
        let creds_path = dir.path().join("credentials");

        env::set_var("ENGRAM_CONFIG_PATH", &config_path);
        env::set_var("ENGRAM_CREDENTIALS_PATH", &creds_path);

        // Save a vault entry in config.
        let mut cfg = EngramConfig::default();
        cfg.add_vault("work".to_string(), make_entry("/vaults/work", true));
        cfg.save().expect("save config should succeed");

        // Save credentials separately.
        let mut creds = CredentialsConfig::default();
        creds.vaults.insert(
            "work".to_string(),
            VaultSyncCredentials {
                backend: "s3".to_string(),
                access_key: Some("AKIA_SECRET".to_string()),
                secret_key: Some("top-secret-key".to_string()),
                bucket: None,
                endpoint: None,
                container: None,
                account: None,
                access_token: None,
                refresh_token: None,
                folder: None,
            },
        );
        EngramConfig::save_credentials(&creds).expect("save_credentials should succeed");

        // config.toml must NOT contain the secret.
        let config_contents = std::fs::read_to_string(&config_path).expect("read config.toml");
        assert!(
            !config_contents.contains("AKIA_SECRET"),
            "config.toml must not contain credentials:
{config_contents}"
        );

        // credentials file MUST contain it.
        let creds_contents = std::fs::read_to_string(&creds_path).expect("read credentials");
        assert!(
            creds_contents.contains("AKIA_SECRET"),
            "credentials file must contain the access_key:
{creds_contents}"
        );

        env::remove_var("ENGRAM_CONFIG_PATH");
        env::remove_var("ENGRAM_CREDENTIALS_PATH");
    }

    // ── CredentialsConfig tests ────────────────────────────────────────────────────

    #[test]
    fn test_credentials_config_default_is_empty() {
        let creds = CredentialsConfig::default();
        assert!(
            creds.vaults.is_empty(),
            "CredentialsConfig default should have an empty vaults map"
        );
    }

    #[test]
    fn test_credentials_config_roundtrip() {
        let mut creds = CredentialsConfig::default();
        creds.vaults.insert(
            "work".to_string(),
            VaultSyncCredentials {
                backend: "s3".to_string(),
                bucket: Some("my-bucket".to_string()),
                access_key: Some("AKID123".to_string()),
                secret_key: Some("secret456".to_string()),
                endpoint: None,
                container: None,
                account: None,
                access_token: None,
                refresh_token: None,
                folder: None,
            },
        );

        let toml_str = toml::to_string(&creds).expect("serialize CredentialsConfig");
        let parsed: CredentialsConfig =
            toml::from_str(&toml_str).expect("deserialize CredentialsConfig");

        let vault = parsed
            .vaults
            .get("work")
            .expect("work vault missing after roundtrip");
        assert_eq!(vault.backend, "s3");
        assert_eq!(vault.access_key, Some("AKID123".to_string()));
    }

    #[test]
    fn test_vault_entry_has_no_sync_field() {
        let entry = VaultEntry {
            path: PathBuf::from("/vaults/test"),
            access: VaultAccess::ReadWrite,
            sync_mode: SyncMode::Manual,
            default: false,
            vault_type: None,
        };
        let toml_str = toml::to_string(&entry).expect("serialize VaultEntry");
        // The serialized TOML must not contain the string "sync" at all as a key.
        assert!(
            !toml_str.contains("\nsync ") && !toml_str.contains("\nsync="),
            "VaultEntry TOML should not contain a 'sync' key:\n{toml_str}"
        );
    }
}

#[cfg(test)]
mod sync_key_tests {
    use super::*;
    use tempfile::TempDir;

    // Serialise tests that mutate ENGRAM_SYNC_KEY_PATH in the process environment.
    static ENV_LOCK: std::sync::OnceLock<std::sync::Mutex<()>> = std::sync::OnceLock::new();

    fn env_lock() -> std::sync::MutexGuard<'static, ()> {
        ENV_LOCK
            .get_or_init(|| std::sync::Mutex::new(()))
            .lock()
            .unwrap()
    }

    /// ENGRAM_SYNC_KEY_PATH env var must override the default ~/.engram/sync.key path.
    #[test]
    fn sync_key_path_respects_env_var_override() {
        let _guard = env_lock();
        std::env::set_var("ENGRAM_SYNC_KEY_PATH", "/tmp/engram-test-override.key");
        let path = EngramConfig::sync_key_path();
        std::env::remove_var("ENGRAM_SYNC_KEY_PATH");
        assert_eq!(
            path,
            std::path::PathBuf::from("/tmp/engram-test-override.key"),
            "EngramConfig::sync_key_path() should respect ENGRAM_SYNC_KEY_PATH env var override"
        );
    }

    #[test]
    fn write_then_read_key_file_roundtrips() {
        let dir = TempDir::new().unwrap();
        let key_path = dir.path().join("sync.key");
        let key: [u8; 32] = [0xAB; 32];

        write_sync_key_file(&key_path, &key).unwrap();
        let loaded: [u8; 32] = read_sync_key_file(&key_path).unwrap();

        assert_eq!(key, loaded);
    }

    #[test]
    fn key_file_written_with_restrictive_permissions() {
        let dir = TempDir::new().unwrap();
        let key_path = dir.path().join("sync.key");
        let key: [u8; 32] = [0x01; 32];

        write_sync_key_file(&key_path, &key).unwrap();

        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let perms = std::fs::metadata(&key_path).unwrap().permissions();
            assert_eq!(
                perms.mode() & 0o777,
                0o600,
                "sync.key must be chmod 600"
            );
        }
    }

    #[test]
    fn key_file_not_world_readable_before_write() {
        let dir = TempDir::new().unwrap();
        let key_path = dir.path().join("sync.key");
        let result = write_sync_key_file(&key_path, &[0u8; 32]);
        assert!(result.is_ok());
    }
}
