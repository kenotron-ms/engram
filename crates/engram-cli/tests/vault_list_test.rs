// Integration tests for `engram vault list`
//
// These tests run the compiled binary and verify output format.

use assert_cmd::Command;
use predicates::prelude::*;
use std::fs;
use std::path::Path;
use tempfile::TempDir;

// ── helper ─────────────────────────────────────────────────────────────────

/// Write a TOML config file at `<dir>/config.toml` and return the full path
/// as a `String` suitable for passing to `ENGRAM_CONFIG_PATH`.
fn write_config(dir: &Path, toml: &str) -> String {
    let config_path = dir.join("config.toml");
    fs::write(&config_path, toml).expect("failed to write config file");
    config_path.to_string_lossy().to_string()
}

// ── tests ───────────────────────────────────────────────────────────────────

/// `engram vault list` must exit with code 0.
#[test]
fn test_vault_list_exits_zero() {
    let dir = TempDir::new().unwrap();
    let config_path = write_config(dir.path(), "");

    let mut cmd = Command::cargo_bin("engram").unwrap();
    cmd.args(["vault", "list"])
        .env("ENGRAM_CONFIG_PATH", &config_path);
    cmd.assert().success();
}

/// `engram vault list` with an empty (no vaults) config must print
/// "No vaults configured".
#[test]
fn test_vault_list_shows_no_vaults_when_empty() {
    let dir = TempDir::new().unwrap();
    let config_path = write_config(dir.path(), "");

    let mut cmd = Command::cargo_bin("engram").unwrap();
    cmd.args(["vault", "list"])
        .env("ENGRAM_CONFIG_PATH", &config_path);
    cmd.assert()
        .success()
        .stdout(predicate::str::contains("No vaults configured"));
}

/// `engram vault list` with a configured vault must show the vault name and
/// its access mode ("read-write").
#[test]
fn test_vault_list_shows_configured_vault() {
    let dir = TempDir::new().unwrap();
    let toml = r#"
[vaults.personal]
path = "/home/user/personal"
access = "read-write"
sync_mode = "approval"
default = true
"#;
    let config_path = write_config(dir.path(), toml);

    let mut cmd = Command::cargo_bin("engram").unwrap();
    cmd.args(["vault", "list"])
        .env("ENGRAM_CONFIG_PATH", &config_path);
    cmd.assert()
        .success()
        .stdout(predicate::str::contains("personal"))
        .stdout(predicate::str::contains("read-write"));
}
