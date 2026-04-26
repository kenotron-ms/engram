// Integration tests for multi-vault scenarios: vault list, awareness output, and sync approval.
//
// All tests live inside `mod multi_vault` so that
// `cargo test -p engram multi_vault` selects all six tests.

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

// ─── tests ───────────────────────────────────────────────────────────────────

mod multi_vault {
    use super::*;

    /// `engram vault list` with two configured vaults (alpha with read access and auto sync,
    /// beta with read-write access and approval sync) must show both vault names and the
    /// 'read' access mode.
    #[test]
    fn test_vault_list_shows_both_configured_vaults() {
        let dir = TempDir::new().unwrap();
        let toml = r#"
[vaults.alpha]
path = "/tmp/alpha-vault"
access = "read"
sync_mode = "auto"
default = true

[vaults.beta]
path = "/tmp/beta-vault"
access = "read-write"
sync_mode = "approval"
default = false
"#;
        let config_path = write_config(dir.path(), toml);

        let mut cmd = Command::cargo_bin("engram").unwrap();
        cmd.args(["vault", "list"])
            .env("ENGRAM_CONFIG_PATH", &config_path);

        cmd.assert()
            .success()
            .stdout(predicate::str::contains("alpha"))
            .stdout(predicate::str::contains("beta"))
            .stdout(predicate::str::contains("read"));
    }

    /// `engram awareness` with two configured vaults must show both vault names,
    /// domain counts (Work (2) and Design (1)), and the context wrapper tags.
    #[test]
    fn test_awareness_shows_both_vault_sections() {
        let dir = TempDir::new().unwrap();

        // Create vault A with Work/ directory containing 2 markdown files.
        let vault_a = dir.path().join("vault_a");
        let work_dir = vault_a.join("Work");
        fs::create_dir_all(&work_dir).unwrap();
        fs::write(work_dir.join("note1.md"), "# Note 1\nWork content one.").unwrap();
        fs::write(work_dir.join("note2.md"), "# Note 2\nWork content two.").unwrap();

        // Create vault B with Design/ directory containing 1 markdown file.
        let vault_b = dir.path().join("vault_b");
        let design_dir = vault_b.join("Design");
        fs::create_dir_all(&design_dir).unwrap();
        fs::write(design_dir.join("mockup.md"), "# Mockup\nDesign content.").unwrap();

        let toml = format!(
            r#"
[vaults.workvault]
path = "{}"
access = "read-write"
sync_mode = "approval"
default = true

[vaults.designvault]
path = "{}"
access = "read-write"
sync_mode = "manual"
default = false
"#,
            vault_a.to_string_lossy(),
            vault_b.to_string_lossy()
        );
        let config_path = write_config(dir.path(), &toml);

        let mut cmd = Command::cargo_bin("engram").unwrap();
        cmd.arg("awareness")
            .env("ENGRAM_CONFIG_PATH", &config_path);

        cmd.assert()
            .success()
            .stdout(predicate::str::contains("<engram-context>"))
            .stdout(predicate::str::contains("</engram-context>"))
            .stdout(predicate::str::contains("Work (2)"))
            .stdout(predicate::str::contains("Design (1)"))
            .stdout(predicate::str::contains("workvault"))
            .stdout(predicate::str::contains("designvault"));
    }

    /// `engram awareness` with a vault containing `_context/about.md` must include
    /// the file's content in the output (Layer 2 context files).
    #[test]
    fn test_awareness_layer2_context_files_appear_in_output() {
        let dir = TempDir::new().unwrap();
        let vault_dir = dir.path().join("vault");
        let context_dir = vault_dir.join("_context");
        fs::create_dir_all(&context_dir).unwrap();
        fs::write(context_dir.join("about.md"), "My personal knowledge vault").unwrap();

        let toml = format!(
            r#"
[vaults.personal]
path = "{}"
access = "read-write"
sync_mode = "approval"
default = true
"#,
            vault_dir.to_string_lossy()
        );
        let config_path = write_config(dir.path(), &toml);

        let mut cmd = Command::cargo_bin("engram").unwrap();
        cmd.arg("awareness")
            .env("ENGRAM_CONFIG_PATH", &config_path);

        cmd.assert()
            .success()
            .stdout(predicate::str::contains("My personal knowledge vault"));
    }

    /// `engram sync --vault myvault` with `sync_mode = "approval"` and NO `--approve` flag
    /// must exit 0 and show either "approval required" or "To push:".
    #[test]
    fn test_sync_approval_mode_blocks_push_without_approve_flag() {
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

        // Must show either "approval required" or "To push:".
        assert!(
            stdout.contains("approval required") || stdout.contains("To push:"),
            "approval mode without --approve should show 'approval required' or 'To push:', \
             got stdout: {}\nstderr: {}",
            stdout,
            stderr
        );
    }

    /// `engram sync --vault myvault` with `sync_mode = "manual"` must exit 0 and print a
    /// message containing "manual sync mode".
    #[test]
    fn test_sync_manual_mode_shows_informational_message() {
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

    /// `engram sync --vault readonly` must exit non-zero and print "read-only" in stderr
    /// when the vault is configured with `access = "read"`.
    #[test]
    fn test_access_control_blocks_sync_on_read_only_vault() {
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
        cmd.args(["sync", "--vault", "readonly"])
            .env("ENGRAM_CONFIG_PATH", &config_path);

        cmd.assert()
            .failure()
            .stderr(predicate::str::contains("read-only"));
    }
}
