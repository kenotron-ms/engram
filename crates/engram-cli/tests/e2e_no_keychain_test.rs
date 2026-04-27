// E2E tests — full flow without keychain
//
// These tests verify the exact scenario from the bug report: running engram in
// headless environments with ZERO OS keychain interaction.
//
// All tests use:
//   - ENGRAM_CONFIG_PATH  → isolated temp directory
//   - ENGRAM_VAULT_PASSPHRASE = "headless-test-passphrase"
//   - ENGRAM_VAULT_KEY removed (so tier-1 raw-key bypass is skipped)
//
// This ensures the three-tier key resolution falls into Tier 2 (passphrase
// + Argon2id derivation) and never attempts to open the OS keychain.

use assert_cmd::Command;
use std::fs;
use tempfile::TempDir;

// ── helper ──────────────────────────────────────────────────────────────────

/// Build a `Command` for the `engram` binary pre-configured for a fully
/// headless environment:
///
///   - `ENGRAM_CONFIG_PATH` → `config_path`  (isolated from ~/.engram)
///   - `ENGRAM_VAULT_PASSPHRASE` = `"headless-test-passphrase"` (Tier 2 key)
///   - `ENGRAM_VAULT_KEY` removed                                (no Tier 1)
///
/// No OS keychain is touched by any command run through this helper.
fn engram(config_path: &str) -> Command {
    let mut cmd = Command::cargo_bin("engram").unwrap();
    cmd.env("ENGRAM_CONFIG_PATH", config_path)
        .env("ENGRAM_VAULT_PASSPHRASE", "headless-test-passphrase")
        .env_remove("ENGRAM_VAULT_KEY");
    cmd
}

// ── tests ────────────────────────────────────────────────────────────────────

/// `engram init` must succeed in a headless environment.
///
/// After init the config file must:
///   - contain a `salt` field (key derivation anchor)
///   - NOT contain `vault_key`, `key_bytes`, or `passphrase` in plain text
///
/// This confirms that the raw secret is never written to disk.
#[test]
fn test_init_headless() {
    let dir = TempDir::new().unwrap();
    let config_path = dir.path().join("config.toml").to_string_lossy().to_string();

    // Init must succeed without touching the OS keychain.
    engram(&config_path).arg("init").assert().success();

    // Config file must exist.
    assert!(
        std::path::Path::new(&config_path).exists(),
        "config.toml was not created at {config_path}"
    );

    let contents = fs::read_to_string(&config_path).expect("failed to read config.toml");

    // Must contain the Argon2 salt anchor.
    assert!(
        contents.contains("salt"),
        "config.toml must contain a 'salt' field, got:\n{contents}"
    );

    // Must NOT store the raw key or passphrase.
    assert!(
        !contents.contains("vault_key"),
        "config.toml must NOT contain 'vault_key', got:\n{contents}"
    );
    assert!(
        !contents.contains("key_bytes"),
        "config.toml must NOT contain 'key_bytes', got:\n{contents}"
    );
    assert!(
        !contents.contains("passphrase"),
        "config.toml must NOT contain 'passphrase', got:\n{contents}"
    );
}

/// `engram vault add` must succeed in a headless environment.
///
/// After init + vault add the config must contain the vault name and the
/// requested sync mode (`approval`).
#[test]
fn test_vault_add_headless() {
    let dir = TempDir::new().unwrap();
    let config_path = dir.path().join("config.toml").to_string_lossy().to_string();

    // Create the vault directory on disk (vault add validates the path).
    let vault_path = dir.path().join("myvault");
    fs::create_dir_all(&vault_path).unwrap();
    let vault_path_str = vault_path.to_string_lossy().to_string();

    // Step 1 — init.
    engram(&config_path).arg("init").assert().success();

    // Step 2 — vault add.
    engram(&config_path)
        .args([
            "vault",
            "add",
            "myvault",
            "--path",
            &vault_path_str,
            "--sync-mode",
            "approval",
            "--default",
        ])
        .assert()
        .success();

    // The config must now contain the vault entry.
    let contents = fs::read_to_string(&config_path).expect("failed to read config.toml");

    assert!(
        contents.contains("myvault"),
        "config.toml must contain 'myvault' after vault add, got:\n{contents}"
    );
    assert!(
        contents.contains("approval"),
        "config.toml must contain 'approval' sync mode, got:\n{contents}"
    );
}

/// `engram auth add s3` must succeed in a headless environment.
///
/// This is the exact scenario from the bug report: `engram auth add s3` used
/// to panic with a `Keyring` error in CI / headless environments.  After the
/// fix the command must:
///   - exit 0
///   - write credentials to the credentials file (not config.toml, not the OS keychain)
///   - produce NO output that mentions "Keyring", "keyring", or
///     "SecKeychainFind" (the OS-level keychain API)
#[test]
fn test_auth_add_s3_headless_does_not_panic() {
    let dir = TempDir::new().unwrap();
    let config_path = dir.path().join("config.toml").to_string_lossy().to_string();
    let creds_path = dir.path().join("credentials").to_string_lossy().to_string();

    let vault_path = dir.path().join("myvault");
    fs::create_dir_all(&vault_path).unwrap();
    let vault_path_str = vault_path.to_string_lossy().to_string();

    // Step 1 — init.
    engram(&config_path).arg("init").assert().success();

    // Step 2 — vault add.
    engram(&config_path)
        .args([
            "vault",
            "add",
            "myvault",
            "--path",
            &vault_path_str,
            "--sync-mode",
            "approval",
            "--default",
        ])
        .assert()
        .success();

    // Step 3 — auth add s3 (the previously-panicking command).
    let output = engram(&config_path)
        .env("ENGRAM_CREDENTIALS_PATH", &creds_path)
        .args([
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
        .output()
        .unwrap();

    let stderr = String::from_utf8_lossy(&output.stderr);
    let stdout = String::from_utf8_lossy(&output.stdout);
    let combined = format!("{}{}", stderr, stdout);

    // Must exit 0 — no panic, no unhandled error.
    assert!(
        output.status.success(),
        "auth add s3 must succeed in headless env, stderr: {}\nstdout: {}",
        stderr,
        stdout
    );

    // Must NOT reference the OS keychain.
    assert!(
        !combined.contains("Keyring"),
        "output must not contain 'Keyring' error\nstderr: {}\nstdout: {}",
        stderr,
        stdout
    );
    assert!(
        !combined.contains("keyring"),
        "output must not contain 'keyring' error\nstderr: {}\nstdout: {}",
        stderr,
        stdout
    );
    assert!(
        !combined.contains("SecKeychainFind"),
        "output must not mention OS keychain calls\nstderr: {}\nstdout: {}",
        stderr,
        stdout
    );

    // Credentials must have been written to the credentials file (not config.toml).
    let creds_contents = fs::read_to_string(&creds_path).expect("failed to read credentials file");

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

/// `engram status` must succeed in a headless environment.
///
/// After init the status command must exit 0 and must NOT print any
/// "Keyring" or "keyring" string — confirming that the status check itself
/// does not attempt to open the OS keychain.
#[test]
fn test_status_headless() {
    let dir = TempDir::new().unwrap();
    let config_path = dir.path().join("config.toml").to_string_lossy().to_string();

    // Init first so the config has a salt.
    engram(&config_path).arg("init").assert().success();

    // Status must succeed.
    let output = engram(&config_path).arg("status").output().unwrap();

    let stderr = String::from_utf8_lossy(&output.stderr);
    let stdout = String::from_utf8_lossy(&output.stdout);
    let combined = format!("{}{}", stderr, stdout);

    assert!(
        output.status.success(),
        "status must succeed in headless env, stderr: {}\nstdout: {}",
        stderr,
        stdout
    );

    // Must NOT reference the OS keychain.
    assert!(
        !combined.contains("Keyring"),
        "status output must not reference 'Keyring'\nstderr: {}\nstdout: {}",
        stderr,
        stdout
    );
    assert!(
        !combined.contains("keyring"),
        "status output must not reference 'keyring'\nstderr: {}\nstdout: {}",
        stderr,
        stdout
    );
}

/// `engram doctor` must show the passphrase-based key method.
///
/// After init with `ENGRAM_VAULT_PASSPHRASE` set the doctor output must:
///   - contain `"passphrase"` or `"ENGRAM_VAULT_PASSPHRASE"` (key method line)
///   - NOT contain `"not initialized"` in the Key method line (key IS set up)
///
/// This confirms that the doctor command reads the key method from the
/// environment variable and reports it correctly, without falling back to
/// the "not initialized" state that would indicate broken configuration.
#[test]
fn test_doctor_shows_passphrase_method() {
    let dir = TempDir::new().unwrap();
    let config_path = dir.path().join("config.toml").to_string_lossy().to_string();

    // Init so the config has a salt.
    engram(&config_path).arg("init").assert().success();

    // Doctor must succeed.
    let output = engram(&config_path).arg("doctor").output().unwrap();

    let stderr = String::from_utf8_lossy(&output.stderr);
    let stdout = String::from_utf8_lossy(&output.stdout);

    assert!(
        output.status.success(),
        "doctor must succeed in headless env, stderr: {}\nstdout: {}",
        stderr,
        stdout
    );

    // Output must show the passphrase-based key method.
    assert!(
        stdout.contains("passphrase") || stdout.contains("ENGRAM_VAULT_PASSPHRASE"),
        "doctor output must contain 'passphrase' or 'ENGRAM_VAULT_PASSPHRASE' (key method), got:\n{}",
        stdout
    );

    // The Key: line must NOT say "not initialized".
    // Note: the Store line may legitimately say "(not initialized)" if no
    // memories have been written yet — we only check the Key: line here.
    let key_line = stdout
        .lines()
        .find(|l| l.trim_start().starts_with("Key:"))
        .unwrap_or("");

    assert!(
        !key_line.contains("not initialized"),
        "Key method must not be 'not initialized' after init with ENGRAM_VAULT_PASSPHRASE; \
         Key line: '{}'\nFull output:\n{}",
        key_line,
        stdout
    );
}
