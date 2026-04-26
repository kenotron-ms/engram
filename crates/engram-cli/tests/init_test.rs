// Integration tests for `engram init`
//
// These tests run the compiled binary and verify that the init command:
//   - creates the config with a salt field
//   - is idempotent (second run does not change the salt)
//   - sets 0600 permissions on the config file (unix only)

use assert_cmd::Command;
use std::fs;
use std::path::Path;
use tempfile::TempDir;

// ── helper ──────────────────────────────────────────────────────────────────

/// Build a `Command` for the `engram` binary pre-configured with:
///   - `ENGRAM_CONFIG_PATH` pointing at `<dir>/config.toml`
///   - `ENGRAM_VAULT_PASSPHRASE=test-init-passphrase`
///   - `ENGRAM_VAULT_KEY` removed (so tier-1 resolution is skipped)
///
/// Returns the command and the config path string.
fn engram_with_config(dir: &Path) -> (Command, String) {
    let config_path = dir.join("config.toml").to_string_lossy().to_string();
    let mut cmd = Command::cargo_bin("engram").unwrap();
    cmd.env("ENGRAM_CONFIG_PATH", &config_path)
        .env("ENGRAM_VAULT_PASSPHRASE", "test-init-passphrase")
        .env_remove("ENGRAM_VAULT_KEY");
    (cmd, config_path)
}

// ── tests ────────────────────────────────────────────────────────────────────

/// `engram init` must succeed, write a config.toml that contains a `salt` field
/// under `[key]`, and must NOT store any vault_key or passphrase in the file.
#[test]
fn test_init_creates_config_with_salt() {
    let dir = TempDir::new().unwrap();
    let (mut cmd, config_path) = engram_with_config(dir.path());

    cmd.arg("init").assert().success();

    // Config file must exist.
    assert!(
        Path::new(&config_path).exists(),
        "config.toml was not created at {config_path}"
    );

    let contents = fs::read_to_string(&config_path).expect("failed to read config.toml");

    // Must contain the [key] section with a salt.
    assert!(
        contents.contains("salt"),
        "config.toml must contain a 'salt' field, got:\n{contents}"
    );

    // Must NOT store the raw passphrase or vault key.
    assert!(
        !contents.contains("passphrase"),
        "config.toml must NOT contain 'passphrase', got:\n{contents}"
    );
    assert!(
        !contents.contains("vault_key"),
        "config.toml must NOT contain 'vault_key', got:\n{contents}"
    );
    assert!(
        !contents.contains("password"),
        "config.toml must NOT contain 'password', got:\n{contents}"
    );
}

/// Running `engram init` twice must be idempotent: the second run must print
/// "Vault already initialized." and must NOT change the salt.
#[test]
fn test_init_is_idempotent() {
    let dir = TempDir::new().unwrap();

    // First init.
    let (mut cmd1, config_path) = engram_with_config(dir.path());
    cmd1.arg("init").assert().success();

    let salt_after_first =
        fs::read_to_string(&config_path).expect("failed to read config after first init");

    // Second init — must succeed and print the idempotency message.
    let (mut cmd2, _) = engram_with_config(dir.path());
    cmd2.arg("init")
        .assert()
        .success()
        .stdout(predicates::str::contains("Vault already initialized."));

    let salt_after_second =
        fs::read_to_string(&config_path).expect("failed to read config after second init");

    // The config file (and therefore the salt) must be unchanged.
    assert_eq!(
        salt_after_first, salt_after_second,
        "config.toml must not change on the second 'engram init'"
    );
}

/// After `engram init`, the config file must have mode 0600 (owner read/write
/// only).  This is a unix-only test because Windows does not use POSIX modes.
#[test]
#[cfg(unix)]
fn test_init_sets_0600_permissions() {
    use std::os::unix::fs::PermissionsExt;

    let dir = TempDir::new().unwrap();
    let (mut cmd, config_path) = engram_with_config(dir.path());

    cmd.arg("init").assert().success();

    let meta = fs::metadata(&config_path).expect("failed to stat config.toml");
    let mode = meta.permissions().mode() & 0o777;
    assert_eq!(
        mode, 0o600,
        "config.toml must have permissions 0600, got {mode:04o}"
    );
}
