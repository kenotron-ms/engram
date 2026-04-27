// Integration tests for `engram auth add` — credentials-file-based credential storage
//
// These tests verify that `engram auth add s3` writes credentials to the credentials
// file instead of config.toml, and that the --vault flag is supported.

use assert_cmd::Command;
use std::fs;
use std::path::Path;
use tempfile::TempDir;

// ── helpers ──────────────────────────────────────────────────────────────────────

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

/// Return the credentials file path for the given temp dir.
fn credentials_path(dir: &Path) -> String {
    dir.join("credentials").to_string_lossy().to_string()
}

// ── tests ────────────────────────────────────────────────────────────────────────────

/// `engram auth add s3 --vault <name>` must write credentials to the credentials
/// file, NOT to config.toml.
/// After running the command:
///  - config.toml must NOT contain backend or AKID1234
///  - credentials file MUST contain backend = "s3" and AKID1234
#[test]
fn test_auth_add_s3_writes_to_credentials_not_config() {
    let dir = TempDir::new().unwrap();
    let vault_path = dir.path().join("vault");
    fs::create_dir_all(&vault_path).unwrap();
    let config_path = write_config_with_vault(dir.path(), "myvault", &vault_path.to_string_lossy());
    let creds_path = credentials_path(dir.path());

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
    .env("ENGRAM_CREDENTIALS_PATH", &creds_path)
    .env_remove("ENGRAM_VAULT_KEY")
    .env_remove("ENGRAM_VAULT_PASSPHRASE");

    cmd.assert().success();

    // config.toml must NOT contain credentials.
    let config_contents = fs::read_to_string(&config_path).expect("failed to read config.toml");
    assert!(
        !config_contents.contains("AKID1234"),
        "config.toml must NOT contain the access_key 'AKID1234', got:\n{config_contents}"
    );
    assert!(
        !config_contents.contains("backend"),
        "config.toml must NOT contain 'backend', got:\n{config_contents}"
    );

    // credentials file MUST contain credentials.
    let creds_contents = fs::read_to_string(&creds_path).expect("failed to read credentials file");
    assert!(
        creds_contents.contains("backend = \"s3\"") || creds_contents.contains("backend = 's3'"),
        "credentials file must contain backend = \"s3\", got:\n{creds_contents}"
    );
    assert!(
        creds_contents.contains("AKID1234"),
        "credentials file must contain the access_key 'AKID1234', got:\n{creds_contents}"
    );
    assert!(
        creds_contents.contains("s3.example.com"),
        "credentials file must contain the endpoint 's3.example.com', got:\n{creds_contents}"
    );
    assert!(
        creds_contents.contains("test-bucket"),
        "credentials file must contain the bucket 'test-bucket', got:\n{creds_contents}"
    );
}

/// After `engram auth add s3`, the credentials file must have mode 0600.
#[test]
#[cfg(unix)]
fn test_auth_add_s3_sets_0600_permissions() {
    use std::os::unix::fs::PermissionsExt;

    let dir = TempDir::new().unwrap();
    let vault_path = dir.path().join("vault");
    fs::create_dir_all(&vault_path).unwrap();
    let config_path = write_config_with_vault(dir.path(), "myvault", &vault_path.to_string_lossy());
    let creds_path = credentials_path(dir.path());

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
    .env("ENGRAM_CREDENTIALS_PATH", &creds_path)
    .env_remove("ENGRAM_VAULT_KEY")
    .env_remove("ENGRAM_VAULT_PASSPHRASE");

    cmd.assert().success();

    let meta = fs::metadata(&creds_path).expect("failed to stat credentials file");
    let mode = meta.permissions().mode() & 0o777;
    assert_eq!(
        mode, 0o600,
        "credentials file must have permissions 0600 after auth add, got {mode:04o}"
    );
}

/// `engram auth add s3 --vault nonexistent` must exit with a non-zero status
/// when the named vault does not exist in the config.
#[test]
fn test_auth_add_s3_fails_for_unknown_vault() {
    let dir = TempDir::new().unwrap();
    let config_path = write_empty_config(dir.path());
    let creds_path = credentials_path(dir.path());

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
    .env("ENGRAM_CREDENTIALS_PATH", &creds_path)
    .env_remove("ENGRAM_VAULT_KEY")
    .env_remove("ENGRAM_VAULT_PASSPHRASE");

    // Must exit with non-zero status code.
    cmd.assert().failure();
}
