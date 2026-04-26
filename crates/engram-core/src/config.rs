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

        Ok(())
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
}

// ──────────────────────────────────────────────────────────────────────────────
// Tests
// ──────────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

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
        let w = W { access: VaultAccess::ReadWrite };
        let s = toml::to_string(&w).expect("serialise");
        assert!(s.contains("read-write"), "kebab-case missing:\n{s}");
    }

    #[test]
    fn sync_mode_serialises_kebab_case() {
        #[derive(Serialize, Deserialize)]
        struct W {
            mode: SyncMode,
        }
        let approval = W { mode: SyncMode::Approval };
        let s_approval = toml::to_string(&approval).expect("serialise");
        assert!(s_approval.contains("approval"), "missing 'approval':\n{s_approval}");

        let manual = W { mode: SyncMode::Manual };
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

    // ── Task-2 tests ──────────────────────────────────────────────────────────

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
        assert!(cfg.remove_vault("x"), "remove should return true for existing vault");
        assert!(!cfg.remove_vault("x"), "remove should return false for missing vault");
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
        assert!(!cfg.get_vault("a").unwrap().default, "a should no longer be default");
        let not_ok = cfg.set_default("missing");
        assert!(!not_ok, "set_default should return false for unknown vault");
    }

    #[test]
    fn test_roundtrip_toml_serialization() {
        use std::env;
        use tempfile::tempdir;

        let dir = tempdir().expect("tempdir");
        let config_path = dir.path().join("config.toml");

        // Override the config path via env var
        env::set_var("ENGRAM_CONFIG_PATH", &config_path);

        let mut cfg = EngramConfig::default();
        cfg.add_vault("main".to_string(), make_entry("/vaults/main", true));
        cfg.save().expect("save should succeed");

        // Load it back
        let loaded = EngramConfig::load();
        let entry = loaded.get_vault("main").expect("main vault missing after load");
        assert_eq!(entry.path, PathBuf::from("/vaults/main"));
        assert!(entry.default);

        env::remove_var("ENGRAM_CONFIG_PATH");
    }

    #[test]
    fn test_config_path_uses_env_var_override() {
        use std::env;
        let custom = "/tmp/engram-test-config.toml";
        env::set_var("ENGRAM_CONFIG_PATH", custom);
        let path = EngramConfig::config_path();
        assert_eq!(path, PathBuf::from(custom));
        env::remove_var("ENGRAM_CONFIG_PATH");
    }
}
