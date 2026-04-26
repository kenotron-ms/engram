// Integration tests for sync credential lookup from config.toml
//
// These tests verify that `engram sync` reads credentials from config.toml
// (not from the OS keychain), and exits cleanly with a helpful message when
// no sync backend is configured.

use assert_cmd::Command;
use std::fs;
use std::path::Path;
use tempfile::TempDir;

// ── helpers ──────────────────────────────────────────────────────────────────

/// Write a TOML config file at `<dir>/config.toml` and return the full path
/// as a `String` suitable for passing to `ENGRAM_CONFIG_PATH`.
fn write_config(dir: &Path, toml: &str) -> String {
    let config_path = dir.join("config.toml");
    fs::write(&config_path, toml).expect("failed to write config file");
    config_path.to_string_lossy().to_string()
}

/// A valid base64-encoded 32-byte key (all zeros) suitable for ENGRAM_VAULT_KEY.
/// This bypasses the vault-key derivation step so tests can reach the
/// sync-credentials check cleanly.
const DUMMY_VAULT_KEY: &str = "AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA=";

// ── tests ─────────────────────────────────────────────────────────────────────

/// A vault with no `[vaults.<name>.sync]` block must exit with a helpful error
/// message — NOT panic with a keychain/keyring error.
///
/// Acceptable messages (any one must appear in stderr or stdout):
///   - "No sync backend"   – produced when config has no sync credentials
///   - "engram auth add"   – produced as a hint for the user
///   - "not initialized"   – produced if the vault key setup is missing
#[test]
fn test_sync_exits_cleanly_when_no_backend_configured() {
    let dir = TempDir::new().unwrap();
    let vault_dir = dir.path().join("vault");
    fs::create_dir_all(&vault_dir).unwrap();

    // Config with a vault that has NO sync block.
    let toml = format!(
        r#"[vaults.myvault]
path = "{}"
access = "read-write"
sync_mode = "auto"
default = true
"#,
        vault_dir.to_string_lossy()
    );
    let config_path = write_config(dir.path(), &toml);

    let mut cmd = Command::cargo_bin("engram").unwrap();
    cmd.args(["sync", "--vault", "myvault"])
        .env("ENGRAM_CONFIG_PATH", &config_path)
        // Provide a valid vault key so we bypass the key-derivation step and
        // reach the sync-credentials check.
        .env("ENGRAM_VAULT_KEY", DUMMY_VAULT_KEY)
        .env_remove("ENGRAM_VAULT_PASSPHRASE");

    let output = cmd.output().unwrap();

    // Must exit with a non-zero code (helpful error, not success).
    assert!(
        !output.status.success(),
        "expected non-zero exit code, got success"
    );

    let stderr = String::from_utf8_lossy(&output.stderr);
    let stdout = String::from_utf8_lossy(&output.stdout);
    let combined = format!("{}{}", stderr, stdout);

    // The output must contain at least one of the acceptable informative strings.
    let acceptable = ["No sync backend", "engram auth add", "not initialized"];
    let found = acceptable.iter().any(|s| combined.contains(s));
    assert!(
        found,
        "expected one of {:?} in output but got:\nstderr: {}\nstdout: {}",
        acceptable, stderr, stdout
    );

    // Must NOT contain keyring/keychain panic messages.
    assert!(
        !combined.contains("keyring"),
        "output must not mention keyring (old keychain code path)\nstderr: {}",
        stderr
    );
    assert!(
        !combined.contains("SecKeychainFind"),
        "output must not mention OS keychain calls\nstderr: {}",
        stderr
    );
}
