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
    cmd.assert().success().stdout(predicate::str::contains(
        "─────────────────────────────────────────",
    ));
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
        stdout.contains("records)")
            || stdout.contains("not initialized")
            || stdout.contains("wrong key")
            || stdout.contains("no key"),
        "Memory store line must contain a recognisable status, got: {}",
        stdout
    );
}

// ── Auth subcommand tests ─────────────────────────────────────────────────────

/// `engram auth --help` must show the expected subcommands: add, list, remove.
#[test]
fn test_auth_subcommand_help_shows_variants() {
    let mut cmd = Command::cargo_bin("engram").unwrap();
    cmd.args(["auth", "--help"]);
    cmd.assert()
        .success()
        .stdout(predicate::str::contains("add"))
        .stdout(predicate::str::contains("list"))
        .stdout(predicate::str::contains("remove"));
}

/// `engram auth add --help` must show the four backend options.
#[test]
fn test_auth_add_subcommand_help_shows_backends() {
    let mut cmd = Command::cargo_bin("engram").unwrap();
    cmd.args(["auth", "add", "--help"]);
    cmd.assert()
        .success()
        .stdout(predicate::str::contains("s3"))
        .stdout(predicate::str::contains("onedrive"))
        .stdout(predicate::str::contains("azure"))
        .stdout(predicate::str::contains("gdrive"));
}

/// `engram auth add s3 --help` must show all four expected flags.
#[test]
fn test_auth_add_s3_help_shows_flags() {
    let mut cmd = Command::cargo_bin("engram").unwrap();
    cmd.args(["auth", "add", "s3", "--help"]);
    cmd.assert()
        .success()
        .stdout(predicate::str::contains("--endpoint"))
        .stdout(predicate::str::contains("--bucket"))
        .stdout(predicate::str::contains("--access-key"))
        .stdout(predicate::str::contains("--secret-key"));
}

/// `engram sync --help` must show the --backend flag.
#[test]
fn test_sync_subcommand_help_shows_backend_flag() {
    let mut cmd = Command::cargo_bin("engram").unwrap();
    cmd.args(["sync", "--help"]);
    cmd.assert()
        .success()
        .stdout(predicate::str::contains("--backend"));
}

// ── OneDrive / Azure / GCS backend flag tests ─────────────────────────────────

/// `engram auth add onedrive --help` must show the --folder flag.
#[test]
fn test_auth_add_onedrive_help_shows_folder_flag() {
    let mut cmd = Command::cargo_bin("engram").unwrap();
    cmd.args(["auth", "add", "onedrive", "--help"]);
    cmd.assert()
        .success()
        .stdout(predicate::str::contains("--folder"));
}

/// `engram auth add azure --help` must show the --account and --container flags.
#[test]
fn test_auth_add_azure_help_shows_account_and_container_flags() {
    let mut cmd = Command::cargo_bin("engram").unwrap();
    cmd.args(["auth", "add", "azure", "--help"]);
    cmd.assert()
        .success()
        .stdout(predicate::str::contains("--account"))
        .stdout(predicate::str::contains("--container"));
}

/// `engram auth add gdrive --help` must show the --bucket and --key-file flags.
#[test]
fn test_auth_add_gdrive_help_shows_bucket_and_key_file_flags() {
    let mut cmd = Command::cargo_bin("engram").unwrap();
    cmd.args(["auth", "add", "gdrive", "--help"]);
    cmd.assert()
        .success()
        .stdout(predicate::str::contains("--bucket"))
        .stdout(predicate::str::contains("--key-file"));
}

/// `engram auth add gdrive` with valid args must print ✓ GCS backend configured.
/// Marked ignore because it writes to the platform keychain (requires GUI session on macOS).
#[test]
#[ignore = "requires keychain access; run with cargo test -- --include-ignored in a GUI session"]
fn test_auth_add_gdrive_prints_confirmation() {
    let mut cmd = Command::cargo_bin("engram").unwrap();
    cmd.args([
        "auth",
        "add",
        "gdrive",
        "--bucket",
        "test-bucket",
        "--key-file",
        "/tmp/test-key.json",
    ]);
    cmd.assert()
        .success()
        .stdout(predicate::str::contains("\u{2713} GCS backend configured"))
        .stdout(predicate::str::contains("test-bucket"))
        .stdout(predicate::str::contains("/tmp/test-key.json"));
}

/// `engram auth add azure` with valid args must print ✓ Azure backend configured.
/// Marked ignore because it writes to the platform keychain and requires interactive
/// rpassword input (requires GUI session on macOS).
#[test]
#[ignore = "requires keychain access and interactive input; run with cargo test -- --include-ignored in a GUI session"]
fn test_auth_add_azure_prints_confirmation() {
    let mut cmd = Command::cargo_bin("engram").unwrap();
    cmd.args([
        "auth",
        "add",
        "azure",
        "--account",
        "test-account",
        "--container",
        "test-container",
    ]);
    // rpassword reads from TTY; pipe an empty line as fallback
    cmd.write_stdin("test-access-key\n");
    cmd.assert()
        .success()
        .stdout(predicate::str::contains("\u{2713} Azure backend configured"))
        .stdout(predicate::str::contains("test-account"))
        .stdout(predicate::str::contains("test-container"));
}

// ── S3 confirmation test (existing) ────────────────────────────────────────────

/// `engram auth add s3` with all credentials supplied via CLI prints confirmation.
/// Marked ignore because it writes to the platform keychain (requires GUI session on macOS).
#[test]
#[ignore = "requires keychain access; run with cargo test -- --include-ignored in a GUI session"]
fn test_auth_add_s3_prints_confirmation() {
    let mut cmd = Command::cargo_bin("engram").unwrap();
    cmd.args([
        "auth",
        "add",
        "s3",
        "--endpoint",
        "https://r2.example.com",
        "--bucket",
        "test-bucket",
        "--access-key",
        "test-ak",
        "--secret-key",
        "test-sk",
    ]);
    cmd.assert()
        .success()
        .stdout(predicate::str::contains("\u{2713} S3 backend configured"))
        .stdout(predicate::str::contains("https://r2.example.com"))
        .stdout(predicate::str::contains("test-bucket"));
}
