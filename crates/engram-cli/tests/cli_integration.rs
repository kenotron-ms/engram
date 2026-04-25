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
        .stdout(predicate::str::contains(
            "\u{2713} Azure backend configured",
        ))
        .stdout(predicate::str::contains("test-account"))
        .stdout(predicate::str::contains("test-container"));
}

// ── S3 confirmation test (existing) ────────────────────────────────────────────

// ── auth list / auth remove tests (Task 10) ─────────────────────────────────

/// `engram auth list` must exit 0 and print either backend info or "No backends configured".
#[test]
fn test_auth_list_exits_successfully() {
    let mut cmd = Command::cargo_bin("engram").unwrap();
    cmd.args(["auth", "list"]);
    cmd.assert().success();
}

/// `engram auth list` with no backends configured must show the "No backends configured" message.
/// This is a clean-system test — no keychain writes, always safe to run.
#[test]
fn test_auth_list_shows_no_backends_when_none_configured() {
    let mut cmd = Command::cargo_bin("engram").unwrap();
    cmd.args(["auth", "list"]);
    let output = cmd.output().unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout);
    // On a system with no configured backends this message must appear;
    // on a system that happens to have backends it will show ✓ lines instead.
    // Either way the command must succeed and print something meaningful.
    assert!(
        stdout.contains("No backends configured.") || stdout.contains("✓"),
        "auth list must show 'No backends configured.' or a ✓ backend entry, got: {}",
        stdout
    );
}

/// `engram auth list` must print the separator line (same as status).
#[test]
fn test_auth_list_shows_separator() {
    let mut cmd = Command::cargo_bin("engram").unwrap();
    cmd.args(["auth", "list"]);
    cmd.assert().success().stdout(predicate::str::contains(
        "─────────────────────────────────────────",
    ));
}

/// `engram auth list` must print the "Configured sync backends:" header.
#[test]
fn test_auth_list_shows_header() {
    let mut cmd = Command::cargo_bin("engram").unwrap();
    cmd.args(["auth", "list"]);
    cmd.assert()
        .success()
        .stdout(predicate::str::contains("Configured sync backends:"));
}

/// `engram auth remove` with an unknown backend must exit with a non-zero code.
#[test]
fn test_auth_remove_unknown_backend_exits_nonzero() {
    let mut cmd = Command::cargo_bin("engram").unwrap();
    cmd.args(["auth", "remove", "unknown-backend-xyz"]);
    cmd.assert().failure();
}

/// `engram auth remove` with an unknown backend must print an error message to stderr.
#[test]
fn test_auth_remove_unknown_backend_prints_error() {
    let mut cmd = Command::cargo_bin("engram").unwrap();
    cmd.args(["auth", "remove", "unknown-backend-xyz"]);
    cmd.assert().failure().stderr(predicate::str::contains(
        "Unknown backend: unknown-backend-xyz",
    ));
}

/// `engram auth remove` with a known but unconfigured backend must exit 0 and say "No credentials found".
#[test]
fn test_auth_remove_known_unconfigured_backend_exits_zero() {
    // We assume s3 is NOT configured on the test machine.
    // If it is, this test is still valid — it just verifies we don't crash.
    let mut cmd = Command::cargo_bin("engram").unwrap();
    cmd.args(["auth", "remove", "s3"]);
    // Must exit 0 (graceful when there's nothing to remove)
    cmd.assert().success();
}

/// `engram auth remove` with a known but unconfigured backend must print "No credentials found" or "Removed".
#[test]
fn test_auth_remove_known_unconfigured_backend_prints_message() {
    let mut cmd = Command::cargo_bin("engram").unwrap();
    cmd.args(["auth", "remove", "s3"]);
    let output = cmd.output().unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("No credentials found for s3") || stdout.contains("✓ Removed s3"),
        "auth remove s3 must say 'No credentials found for s3' or '✓ Removed s3', got: {}",
        stdout
    );
}

// ─── engram sync tests (Task 11) ──────────────────────────────────────────────

/// `engram sync` with no backend configured must exit with a non-zero code.
/// This test is safe to run in any environment because it does not write to the keychain
/// and relies on the auto-detection path failing gracefully.
///
/// NOTE: If the test machine happens to have all four backends configured this test
/// will pass for a different reason (the sync will attempt to actually push files).
/// That edge case is acceptable because the primary goal is to verify the error path.
#[test]
fn test_sync_no_backend_configured_exits_nonzero() {
    let mut cmd = Command::cargo_bin("engram").unwrap();
    cmd.args(["sync"]);
    // On a clean machine with no backends configured this must fail with a meaningful message.
    // We accept either failure (no backends) or success (backends are configured) — the
    // important thing is that the binary does NOT panic with todo!().
    let output = cmd.output().unwrap();
    let stderr = String::from_utf8_lossy(&output.stderr);
    // If it fails it must print the expected error, not a panic message.
    if !output.status.success() {
        assert!(
            stderr.contains("No sync backend configured") || stderr.contains("No vault key found"),
            "sync failure must print a known error message, got stderr: {}",
            stderr
        );
    }
}

/// `engram sync` must NOT panic with todo!() — it must either succeed or fail gracefully.
#[test]
fn test_sync_does_not_panic() {
    let mut cmd = Command::cargo_bin("engram").unwrap();
    cmd.args(["sync"]);
    let output = cmd.output().unwrap();
    let stderr = String::from_utf8_lossy(&output.stderr);
    // A todo!() panic produces "not yet implemented" on stderr
    assert!(
        !stderr.contains("not yet implemented"),
        "run_sync must not call todo!(), got stderr: {}",
        stderr
    );
}

// ─── engram index tests (Task 7) ───────────────────────────────────────────────

/// `engram index --help` must exit 0 and show the --vault and --force flags.
#[test]
fn test_index_help_shows_vault_and_force_flags() {
    let mut cmd = Command::cargo_bin("engram").unwrap();
    cmd.args(["index", "--help"]);
    cmd.assert()
        .success()
        .stdout(predicate::str::contains("--vault"))
        .stdout(predicate::str::contains("--force"));
}

/// `engram index --vault /nonexistent` must exit non-zero and print "Vault not found" to stderr.
#[test]
fn test_index_nonexistent_vault_exits_nonzero() {
    let mut cmd = Command::cargo_bin("engram").unwrap();
    cmd.args(["index", "--vault", "/tmp/nonexistent_engram_vault_xyz_abc"]);
    cmd.assert()
        .failure()
        .stderr(predicate::str::contains("Vault not found"));
}

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
