// Integration tests for the `engram awareness` subcommand.

use assert_cmd::Command;
use predicates::prelude::*;
use std::fs;
use tempfile::TempDir;

/// `engram awareness` must exit with code 0.
#[test]
fn test_awareness_exits_zero() {
    let mut cmd = Command::cargo_bin("engram").unwrap();
    cmd.arg("awareness");
    cmd.assert().success();
}

/// Output must contain the XML-style context wrapper tags.
#[test]
fn test_awareness_output_contains_context_tags() {
    let mut cmd = Command::cargo_bin("engram").unwrap();
    cmd.arg("awareness");
    cmd.assert()
        .success()
        .stdout(predicate::str::contains("<engram-context>"))
        .stdout(predicate::str::contains("</engram-context>"));
}

/// Domain counts are shown correctly for a populated vault.
///
/// Vault layout:
/// ```
/// Work/
///   note1.md
///   note2.md
/// People/
///   alice.md
/// ```
/// Expected output: contains "Work (2)" and "People (1)".
#[test]
fn test_awareness_shows_domain_counts() {
    let tmp = TempDir::new().unwrap();

    // Create Work/ with 2 markdown files
    let work_dir = tmp.path().join("Work");
    fs::create_dir_all(&work_dir).unwrap();
    fs::write(work_dir.join("note1.md"), "# Note 1\nWork content one.").unwrap();
    fs::write(work_dir.join("note2.md"), "# Note 2\nWork content two.").unwrap();

    // Create People/ with 1 markdown file
    let people_dir = tmp.path().join("People");
    fs::create_dir_all(&people_dir).unwrap();
    fs::write(people_dir.join("alice.md"), "# Alice\nPerson profile.").unwrap();

    let mut cmd = Command::cargo_bin("engram").unwrap();
    cmd.args(["awareness", "--vault", tmp.path().to_str().unwrap()]);
    cmd.assert()
        .success()
        .stdout(predicate::str::contains("Work (2)"))
        .stdout(predicate::str::contains("People (1)"));
}

/// Directories starting with `_` must not appear in domain counts.
///
/// Vault layout:
/// ```
/// _context/
///   system.md
/// ```
/// Expected: "_context" does not appear in the domains line.
#[test]
fn test_awareness_skips_underscore_directories() {
    let tmp = TempDir::new().unwrap();

    // Create _context/ with a markdown file — should be excluded from domain counts
    let context_dir = tmp.path().join("_context");
    fs::create_dir_all(&context_dir).unwrap();
    fs::write(context_dir.join("system.md"), "# System context").unwrap();

    let mut cmd = Command::cargo_bin("engram").unwrap();
    cmd.args(["awareness", "--vault", tmp.path().to_str().unwrap()]);

    let output = cmd.output().unwrap();
    assert!(
        output.status.success(),
        "engram awareness should exit 0, stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        !stdout.contains("_context"),
        "domain listing must not include '_context', got:\n{}",
        stdout
    );
}
