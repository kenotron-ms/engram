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
}
