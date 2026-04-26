// Integration tests for `engram auth add` — config-based credential storage
//
// These tests verify that `engram auth add s3` writes credentials to config.toml
// instead of the OS keychain, and that the --vault flag is supported.

use assert_cmd::Command;
use std::fs;
use std::path::Path;
use tempfile::TempDir;

// ── helpers ──────────────────────────────────────────────────────────────────

/// Write a minimal config.toml with one vault entry at `<dir>/config.toml`.
/// Returns the config path string.
fn write_config_with_vault(dir: &Path, vault_name: &str, vault_path: &str) -> String {
    let config_path = dir.join("config.toml");
    let toml = format!(
        r#"[vaults.{name}]
path = "{path}"
access = "read-write"
sync_mode = "approval"
default = true
"#,
        name = vault_name,
        path = vault_path,
    );
    fs::write(&config_path, &toml).expect("failed to write config file");
    config_path.to_string_lossy().to_string()
}

/// Write an empty config (no vaults) at `<dir>/config.toml`.
/// Returns the config path string.
fn write_empty_config(dir: &Path) -> String {
    let config_path = dir.join("config.toml");
    fs::write(&config_path, "").expect("failed to write empty config file");
    config_path.to_string_lossy().to_string()
}

// ── tests ────────────────────────────────────────────────────────────────────

/// `engram auth add s3 --vault <name>` must write credentials to config.toml.
/// After running the command:
///  - the config must contain backend = "s3"
///  - the config must contain the access_key value
///  - the config must contain the endpoint value
#[test]
fn test_auth_add_s3_writes_to_config() {
    let dir = TempDir::new().unwrap();
    let vault_path = dir.path().join("vault");
    fs::create_dir_all(&vault_path).unwrap();
    let config_path = write_config_with_vault(
        dir.path(),
        "myvault",
        &vault_path.to_string_lossy(),
    );

    let mut cmd = Command::cargo_bin("engram").unwrap();
    cmd.args([
        "auth",
        "add",
        "s3",
        "--vault",
        "myvault",
        "--endpoint",
        "https://s3.example.com",
        "--bucket",
        "test-bucket",
        "--access-key",
        "AKID1234",
        "--secret-key",
        "secretkey5678",
    ])
    .env("ENGRAM_CONFIG_PATH", &config_path)
    .env_remove("ENGRAM_VAULT_KEY")
    .env_remove("ENGRAM_VAULT_PASSPHRASE");

    cmd.assert().success();

    // Read the updated config and verify credentials were written.
    let contents = fs::read_to_string(&config_path).expect("failed to read config.toml");

    assert!(
        contents.contains("backend = \"s3\"") || contents.contains("backend = 's3'"),
        "config.toml must contain backend = \"s3\", got:\n{contents}"
    );
    assert!(
        contents.contains("AKID1234"),
        "config.toml must contain the access_key 'AKID1234', got:\n{contents}"
    );
    assert!(
        contents.contains("s3.example.com"),
        "config.toml must contain the endpoint 's3.example.com', got:\n{contents}"
    );
    assert!(
        contents.contains("test-bucket"),
        "config.toml must contain the bucket 'test-bucket', got:\n{contents}"
    );
}

/// After `engram auth add s3`, the config file must have mode 0600.
#[test]
#[cfg(unix)]
fn test_auth_add_s3_sets_0600_permissions() {
    use std::os::unix::fs::PermissionsExt;

    let dir = TempDir::new().unwrap();
    let vault_path = dir.path().join("vault");
    fs::create_dir_all(&vault_path).unwrap();
    let config_path = write_config_with_vault(
        dir.path(),
        "myvault",
        &vault_path.to_string_lossy(),
    );

    let mut cmd = Command::cargo_bin("engram").unwrap();
    cmd.args([
        "auth",
        "add",
        "s3",
        "--vault",
        "myvault",
        "--endpoint",
        "https://s3.example.com",
        "--bucket",
        "test-bucket",
        "--access-key",
        "AKID1234",
        "--secret-key",
        "secretkey5678",
    ])
    .env("ENGRAM_CONFIG_PATH", &config_path)
    .env_remove("ENGRAM_VAULT_KEY")
    .env_remove("ENGRAM_VAULT_PASSPHRASE");

    cmd.assert().success();

    let meta = fs::metadata(&config_path).expect("failed to stat config.toml");
    let mode = meta.permissions().mode() & 0o777;
    assert_eq!(
        mode, 0o600,
        "config.toml must have permissions 0600 after auth add, got {mode:04o}"
    );
}

/// `engram auth add s3 --vault nonexistent` must exit with a non-zero status
/// when the named vault does not exist in the config.
#[test]
fn test_auth_add_s3_fails_for_unknown_vault() {
    let dir = TempDir::new().unwrap();
    let config_path = write_empty_config(dir.path());

    let mut cmd = Command::cargo_bin("engram").unwrap();
    cmd.args([
        "auth",
        "add",
        "s3",
        "--vault",
        "nonexistent",
        "--endpoint",
        "https://s3.example.com",
        "--bucket",
        "test-bucket",
        "--access-key",
        "AKID1234",
        "--secret-key",
        "secretkey5678",
    ])
    .env("ENGRAM_CONFIG_PATH", &config_path)
    .env_remove("ENGRAM_VAULT_KEY")
    .env_remove("ENGRAM_VAULT_PASSPHRASE");

    // Must exit with non-zero status code.
    cmd.assert().failure();
}
