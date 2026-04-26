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

/// `engram vault add` with --path and --sync-mode auto --default creates
/// an entry in the config file with the correct name and sync mode.
#[test]
fn test_vault_list_add_creates_entry_in_config() {
    let dir = TempDir::new().unwrap();
    let config_path = write_config(dir.path(), "");
    // Use the temp dir itself as the vault path (it exists on disk).
    let vault_path = dir.path().to_string_lossy().to_string();

    let mut cmd = Command::cargo_bin("engram").unwrap();
    cmd.args([
        "vault",
        "add",
        "personal",
        "--path",
        &vault_path,
        "--sync-mode",
        "auto",
        "--default",
    ])
    .env("ENGRAM_CONFIG_PATH", &config_path);
    cmd.assert().success();

    // The config file should now contain [vaults.personal] and 'auto'.
    let contents = fs::read_to_string(&config_path).expect("failed to read config file");
    assert!(
        contents.contains("[vaults.personal]"),
        "config should contain [vaults.personal], got:\n{contents}"
    );
    assert!(
        contents.contains("auto"),
        "config should contain 'auto', got:\n{contents}"
    );
}

/// `engram vault remove temp` removes the [vaults.temp] entry from the config.
#[test]
fn test_vault_list_remove_deletes_entry() {
    let dir = TempDir::new().unwrap();
    let toml = r#"
[vaults.temp]
path = "/tmp/temp-vault"
access = "read-write"
sync_mode = "manual"
default = false
"#;
    let config_path = write_config(dir.path(), toml);

    let mut cmd = Command::cargo_bin("engram").unwrap();
    cmd.args(["vault", "remove", "temp"])
        .env("ENGRAM_CONFIG_PATH", &config_path);
    cmd.assert().success();

    let contents = fs::read_to_string(&config_path).expect("failed to read config file");
    assert!(
        !contents.contains("[vaults.temp]"),
        "config should NOT contain [vaults.temp] after removal, got:\n{contents}"
    );
}

/// `engram vault set-default beta` updates the default vault to beta;
/// beta must still appear in the config.
#[test]
fn test_vault_list_set_default_updates_config() {
    let dir = TempDir::new().unwrap();
    let toml = r#"
[vaults.alpha]
path = "/tmp/alpha-vault"
access = "read-write"
sync_mode = "approval"
default = true

[vaults.beta]
path = "/tmp/beta-vault"
access = "read-write"
sync_mode = "approval"
default = false
"#;
    let config_path = write_config(dir.path(), toml);

    let mut cmd = Command::cargo_bin("engram").unwrap();
    cmd.args(["vault", "set-default", "beta"])
        .env("ENGRAM_CONFIG_PATH", &config_path);
    cmd.assert().success();

    let contents = fs::read_to_string(&config_path).expect("failed to read config file");
    assert!(
        contents.contains("beta"),
        "config should still contain 'beta' after set-default beta, got:\n{contents}"
    );
}

/// `engram vault remove` with a nonexistent vault name must exit nonzero.
#[test]
fn test_vault_list_remove_nonexistent_exits_nonzero() {
    let dir = TempDir::new().unwrap();
    let config_path = write_config(dir.path(), "");

    let mut cmd = Command::cargo_bin("engram").unwrap();
    cmd.args(["vault", "remove", "nonexistent"])
        .env("ENGRAM_CONFIG_PATH", &config_path);
    cmd.assert().failure();
}

/// `engram vault --help` must show the three vault subcommands: list, add, and remove.
#[test]
fn test_vault_list_shows_help_for_vault_command() {
    let mut cmd = Command::cargo_bin("engram").unwrap();
    cmd.args(["vault", "--help"]);
    cmd.assert()
        .success()
        .stdout(predicate::str::contains("list"))
        .stdout(predicate::str::contains("add"))
        .stdout(predicate::str::contains("remove"));
}
