// Integration tests for access control enforcement in engram CLI.
//
// These tests verify that read-only vaults block write operations (observe, sync)
// and read-write vaults pass the access check.
//
// All tests live inside `mod access_control` so that
// `cargo test -p engram access_control` selects all three tests.

use assert_cmd::Command;
use predicates::prelude::*;
use std::fs;
use std::path::Path;
use tempfile::TempDir;

// ─── helper ─────────────────────────────────────────────────────────────────

/// Write a TOML config file at `<dir>/config.toml` and return the full path
/// as a `String` suitable for passing to `ENGRAM_CONFIG_PATH`.
fn write_config(dir: &Path, toml: &str) -> String {
    let config_path = dir.join("config.toml");
    fs::write(&config_path, toml).expect("failed to write config file");
    config_path.to_string_lossy().to_string()
}

// ─── tests ──────────────────────────────────────────────────────────────────

mod access_control {
    use super::*;

    /// `engram observe` must fail with 'read-only' in stderr when the resolved
    /// vault (default vault) is configured with access = "read".
    ///
    /// Note: `engram observe` does not have a --vault flag; it uses resolve_vault(None)
    /// which picks up the default vault from the config. We configure "readonly" as
    /// the default vault so the access check fires.
    #[test]
    fn test_observe_blocked_on_read_only_vault() {
        let dir = TempDir::new().unwrap();
        let toml = r#"
[vaults.readonly]
path = "/tmp/readonly-vault"
access = "read"
sync_mode = "approval"
default = true
"#;
        let config_path = write_config(dir.path(), toml);

        let mut cmd = Command::cargo_bin("engram").unwrap();
        cmd.args(["observe", "/nonexistent/session.jsonl"])
            .env("ENGRAM_CONFIG_PATH", &config_path);
        cmd.assert()
            .failure()
            .stderr(predicate::str::contains("read-only"));
    }

    /// `engram sync --vault readonly-sync` must fail with 'read-only' in stderr
    /// when the named vault is configured with access = "read".
    #[test]
    fn test_sync_push_blocked_on_read_only_vault() {
        let dir = TempDir::new().unwrap();
        let toml = r#"
[vaults.readonly-sync]
path = "/tmp/readonly-sync-vault"
access = "read"
sync_mode = "approval"
default = true
"#;
        let config_path = write_config(dir.path(), toml);

        let mut cmd = Command::cargo_bin("engram").unwrap();
        cmd.args(["sync", "--vault", "readonly-sync"])
            .env("ENGRAM_CONFIG_PATH", &config_path);
        cmd.assert()
            .failure()
            .stderr(predicate::str::contains("read-only"));
    }

    /// `engram sync --vault rw-vault` must NOT produce 'read-only' in stderr when
    /// the named vault is configured with access = "read-write". The command may
    /// still fail (e.g., no backend configured), but the failure must not be the
    /// read-only access check.
    #[test]
    fn test_read_write_vault_passes_access_check_for_sync() {
        let dir = TempDir::new().unwrap();
        let toml = r#"
[vaults.rw-vault]
path = "/tmp/rw-vault"
access = "read-write"
sync_mode = "approval"
default = true
"#;
        let config_path = write_config(dir.path(), toml);

        let mut cmd = Command::cargo_bin("engram").unwrap();
        cmd.args(["sync", "--vault", "rw-vault"])
            .env("ENGRAM_CONFIG_PATH", &config_path);
        let output = cmd.output().unwrap();
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(
            !stderr.contains("read-only"),
            "read-write vault should not produce 'read-only' error, stderr: {}",
            stderr
        );
    }
}
