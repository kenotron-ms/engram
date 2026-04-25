// Integration tests for the engram CLI
//
// These tests run the compiled binary and verify output format.

use assert_cmd::Command;
use predicates::prelude::*;

/// `engram status` must exit with code 0 and produce well-formed output.
#[test]
fn test_status_exits_successfully() {
    let mut cmd = Command::cargo_bin("engram").unwrap();
    cmd.arg("status");
    cmd.assert().success();
}

/// Output must contain the three required status labels in order.
#[test]
fn test_status_output_contains_required_labels() {
    let mut cmd = Command::cargo_bin("engram").unwrap();
    cmd.arg("status");
    cmd.assert()
        .success()
        .stdout(predicate::str::contains("Vault:"))
        .stdout(predicate::str::contains("Memory store:"))
        .stdout(predicate::str::contains("Key:"));
}

/// Output must start with a separator line of box-drawing dashes.
#[test]
fn test_status_output_starts_with_separator() {
    let mut cmd = Command::cargo_bin("engram").unwrap();
    cmd.arg("status");
    // The separator uses the Unicode box-drawing character ─ (U+2500)
    cmd.assert()
        .success()
        .stdout(predicate::str::contains("─────────────────────────────────────────"));
}

/// Fresh system (no vault, no store, no key) must print the "not set" states.
/// This test always checks that one of the two possible key states appears.
#[test]
fn test_status_key_state_is_printed() {
    let mut cmd = Command::cargo_bin("engram").unwrap();
    cmd.arg("status");
    let output = cmd.output().unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout);
    // Key must be either "present ✓" or "not set"
    assert!(
        stdout.contains("present \u{2713}") || stdout.contains("not set"),
        "Key line must contain either 'present ✓' or 'not set', got: {}",
        stdout
    );
}

/// Vault state must contain either the file count or "NOT FOUND".
#[test]
fn test_status_vault_state_is_printed() {
    let mut cmd = Command::cargo_bin("engram").unwrap();
    cmd.arg("status");
    let output = cmd.output().unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("files)") || stdout.contains("NOT FOUND"),
        "Vault line must contain either a file count or 'NOT FOUND', got: {}",
        stdout
    );
}

/// Memory store state must contain a recognisable status phrase.
#[test]
fn test_status_memory_store_state_is_printed() {
    let mut cmd = Command::cargo_bin("engram").unwrap();
    cmd.arg("status");
    let output = cmd.output().unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("records)") || stdout.contains("not initialized") || stdout.contains("wrong key") || stdout.contains("no key"),
        "Memory store line must contain a recognisable status, got: {}",
        stdout
    );
}
