// Integration tests for sync mode enforcement: manual, approval, and --approve flag.
//
// Tests live inside `mod sync_approval` so that
// `cargo test -p engram sync_approval` selects all three tests.

use assert_cmd::Command;
use predicates::prelude::*;
use std::fs;
use std::path::Path;
use tempfile::TempDir;

// ─── helper ───────────────────────────────────────────────────────────────────

/// Write a TOML config file at `<dir>/config.toml` and return the full path
/// as a `String` suitable for passing to `ENGRAM_CONFIG_PATH`.
fn write_config(dir: &Path, toml: &str) -> String {
    let config_path = dir.join("config.toml");
    fs::write(&config_path, toml).expect("failed to write config file");
    config_path.to_string_lossy().to_string()
}

// ─── tests ────────────────────────────────────────────────────────────────────

mod sync_approval {
    use super::*;

    /// When the resolved vault has `sync_mode = "manual"`, `engram sync` must
    /// exit 0 and print an informational message that includes "manual sync mode".
    #[test]
    fn test_sync_manual_mode_prints_message_and_exits_zero() {
        let dir = TempDir::new().unwrap();
        let vault_dir = dir.path().join("vault");
        fs::create_dir_all(&vault_dir).unwrap();

        let toml = format!(
            r#"
[vaults.myvault]
path = "{}"
access = "read-write"
sync_mode = "manual"
default = true
"#,
            vault_dir.to_string_lossy()
        );
        let config_path = write_config(dir.path(), &toml);

        let mut cmd = Command::cargo_bin("engram").unwrap();
        cmd.args(["sync", "--vault", "myvault"])
            .env("ENGRAM_CONFIG_PATH", &config_path);

        cmd.assert()
            .success()
            .stdout(predicate::str::contains("manual sync mode"));
    }

    /// When the resolved vault has `sync_mode = "approval"` and `--approve` is
    /// NOT passed, `engram sync` must exit 0 and show either "approval required"
    /// or a "To push:" hint (listing pending changes).
    #[test]
    fn test_sync_approval_mode_shows_diff_without_approve_flag() {
        let dir = TempDir::new().unwrap();
        let vault_dir = dir.path().join("vault");
        fs::create_dir_all(&vault_dir).unwrap();

        let toml = format!(
            r#"
[vaults.myvault]
path = "{}"
access = "read-write"
sync_mode = "approval"
default = true
"#,
            vault_dir.to_string_lossy()
        );
        let config_path = write_config(dir.path(), &toml);

        let mut cmd = Command::cargo_bin("engram").unwrap();
        cmd.args(["sync", "--vault", "myvault"])
            .env("ENGRAM_CONFIG_PATH", &config_path);

        let output = cmd.output().unwrap();
        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);

        // Must exit 0.
        assert!(
            output.status.success(),
            "sync in approval mode (no --approve) should exit 0, stderr: {}",
            stderr
        );

        // Must print either "approval required" or "To push:" to stdout.
        assert!(
            stdout.contains("approval required") || stdout.contains("To push:"),
            "approval mode without --approve should show 'approval required' or 'To push:', got stdout: {}\nstderr: {}",
            stdout,
            stderr
        );
    }

    /// `engram sync --help` must list both `--approve` and `--vault` flags.
    #[test]
    fn test_sync_approval_help_shows_approve_flag() {
        let mut cmd = Command::cargo_bin("engram").unwrap();
        cmd.args(["sync", "--help"]);

        cmd.assert()
            .success()
            .stdout(predicate::str::contains("--approve"))
            .stdout(predicate::str::contains("--vault"));
    }
}
