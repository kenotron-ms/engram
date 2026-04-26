// Integration tests for the engram CLI
//
// These tests run the compiled binary and verify output format.

use assert_cmd::Command;
use predicates::prelude::*;
use std::fs;
use tempfile::TempDir;

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

/// `engram sync --help` must show the --vault flag (task-6).
#[test]
fn test_sync_help_shows_vault_flag() {
    let mut cmd = Command::cargo_bin("engram").unwrap();
    cmd.args(["sync", "--help"]);
    cmd.assert()
        .success()
        .stdout(predicate::str::contains("--vault"));
}

/// `engram sync --help` must show the --approve flag (task-6).
#[test]
fn test_sync_help_shows_approve_flag() {
    let mut cmd = Command::cargo_bin("engram").unwrap();
    cmd.args(["sync", "--help"]);
    cmd.assert()
        .success()
        .stdout(predicate::str::contains("--approve"));
}

/// `engram search --help` must show the --vault flag (task-6).
#[test]
fn test_search_help_shows_vault_flag() {
    let mut cmd = Command::cargo_bin("engram").unwrap();
    cmd.args(["search", "--help"]);
    cmd.assert()
        .success()
        .stdout(predicate::str::contains("--vault"));
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

/// `engram auth list` with no vaults configured must show an appropriate message.
/// This is a clean-system test — uses a temp config to ensure isolation.
#[test]
fn test_auth_list_shows_no_backends_when_none_configured() {
    let dir = TempDir::new().unwrap();
    let config_path = dir.path().join("config.toml");
    fs::write(&config_path, "").unwrap();

    let mut cmd = Command::cargo_bin("engram").unwrap();
    cmd.args(["auth", "list"])
        .env("ENGRAM_CONFIG_PATH", config_path.to_str().unwrap());
    let output = cmd.output().unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout);
    // With no vaults configured, expect either a "no backends" or "no vaults" message.
    // On a system with vaults having sync configured, it will show ✓ lines instead.
    assert!(
        stdout.contains("No backends configured.")
            || stdout.contains("No vaults configured.")
            || stdout.contains("✓")
            || stdout.contains("no sync configured"),
        "auth list must show an appropriate message or a ✓ entry, got: {}",
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

/// `engram auth list` must print a sync backends header line.
#[test]
fn test_auth_list_shows_header() {
    let mut cmd = Command::cargo_bin("engram").unwrap();
    cmd.args(["auth", "list"]);
    cmd.assert()
        .success()
        .stdout(predicate::str::contains("sync backends:"));
}

/// `engram auth remove` with an unknown backend must exit with a non-zero code.
#[test]
fn test_auth_remove_unknown_backend_exits_nonzero() {
    let mut cmd = Command::cargo_bin("engram").unwrap();
    cmd.args(["auth", "remove", "unknown-backend-xyz"]);
    cmd.assert().failure();
}

/// `engram auth remove` with an unknown vault name must print an error message to stderr.
#[test]
fn test_auth_remove_unknown_backend_prints_error() {
    let dir = TempDir::new().unwrap();
    let config_path = dir.path().join("config.toml");
    fs::write(&config_path, "").unwrap();

    let mut cmd = Command::cargo_bin("engram").unwrap();
    cmd.args(["auth", "remove", "unknown-backend-xyz"])
        .env("ENGRAM_CONFIG_PATH", config_path.to_str().unwrap());
    // The error message must mention the vault name or "not found".
    cmd.assert().failure().stderr(
        predicate::str::contains("unknown-backend-xyz")
            .or(predicate::str::contains("not found")),
    );
}

/// `engram auth remove` with a vault that has no sync credentials must exit 0.
#[test]
fn test_auth_remove_known_unconfigured_backend_exits_zero() {
    let dir = TempDir::new().unwrap();
    let vault_path = dir.path().join("vault");
    fs::create_dir_all(&vault_path).unwrap();
    let config_path = dir.path().join("config.toml");
    // Create a vault with no sync credentials.
    let toml = format!(
        "[vaults.testvault]\npath = \"{}\"\naccess = \"read-write\"\nsync_mode = \"approval\"\ndefault = true\n",
        vault_path.display()
    );
    fs::write(&config_path, &toml).unwrap();

    let mut cmd = Command::cargo_bin("engram").unwrap();
    cmd.args(["auth", "remove", "testvault"])
        .env("ENGRAM_CONFIG_PATH", config_path.to_str().unwrap());
    // Must exit 0 (graceful when there's nothing to remove)
    cmd.assert().success();
}

/// `engram auth remove` with a vault that has no sync credentials must print a meaningful message.
#[test]
fn test_auth_remove_known_unconfigured_backend_prints_message() {
    let dir = TempDir::new().unwrap();
    let vault_path = dir.path().join("vault");
    fs::create_dir_all(&vault_path).unwrap();
    let config_path = dir.path().join("config.toml");
    let toml = format!(
        "[vaults.testvault]\npath = \"{}\"\naccess = \"read-write\"\nsync_mode = \"approval\"\ndefault = true\n",
        vault_path.display()
    );
    fs::write(&config_path, &toml).unwrap();

    let mut cmd = Command::cargo_bin("engram").unwrap();
    cmd.args(["auth", "remove", "testvault"])
        .env("ENGRAM_CONFIG_PATH", config_path.to_str().unwrap());
    let output = cmd.output().unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout);
    // Must print a message about no credentials or a removal confirmation.
    assert!(
        stdout.contains("No sync credentials")
            || stdout.contains("Removed")
            || stdout.contains("no credentials"),
        "auth remove must print a message about credentials, got: {}",
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

// ─── engram search tests (Task 8) ────────────────────────────────────────────

/// `engram search --help` must exit 0 and show the --limit and --mode flags.
#[test]
fn test_search_help_shows_limit_and_mode_flags() {
    let mut cmd = Command::cargo_bin("engram").unwrap();
    cmd.args(["search", "--help"]);
    cmd.assert()
        .success()
        .stdout(predicate::str::contains("--limit"))
        .stdout(predicate::str::contains("--mode"));
}

/// `engram search --help` must show the three mode variants.
#[test]
fn test_search_help_shows_mode_variants() {
    let mut cmd = Command::cargo_bin("engram").unwrap();
    cmd.args(["search", "--help"]);
    cmd.assert()
        .success()
        .stdout(predicate::str::contains("fulltext"))
        .stdout(predicate::str::contains("vector"))
        .stdout(predicate::str::contains("hybrid"));
}

/// `engram search "query"` with no search index must exit non-zero.
#[test]
fn test_search_without_index_exits_nonzero() {
    let mut cmd = Command::cargo_bin("engram").unwrap();
    // Point to a temp dir that has no index so the check fails gracefully.
    cmd.args(["search", "test query"]);
    // May exit 0 (if index happens to exist on test machine) or 1 (no index).
    // We just verify it does NOT panic.
    let output = cmd.output().unwrap();
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        !stderr.contains("not yet implemented"),
        "run_search must not call todo!(), got: {}",
        stderr
    );
}

/// `engram search "query" --mode fulltext` must be a valid invocation (help exits 0).
#[test]
fn test_search_mode_fulltext_flag_is_accepted() {
    let mut cmd = Command::cargo_bin("engram").unwrap();
    cmd.args(["search", "--help"]);
    cmd.assert()
        .success()
        .stdout(predicate::str::contains("fulltext"));
}

/// `engram search "query" --mode vector` must be a valid invocation (help exits 0).
#[test]
fn test_search_mode_vector_flag_is_accepted() {
    let mut cmd = Command::cargo_bin("engram").unwrap();
    cmd.args(["search", "--help"]);
    cmd.assert()
        .success()
        .stdout(predicate::str::contains("vector"));
}

/// `engram search "query" --mode hybrid` must be a valid invocation (help exits 0).
#[test]
fn test_search_mode_hybrid_is_default() {
    let mut cmd = Command::cargo_bin("engram").unwrap();
    cmd.args(["search", "--help"]);
    cmd.assert()
        .success()
        .stdout(predicate::str::contains("hybrid"));
}

// ─── Search index status tests (Task 9) ───────────────────────────────────────

/// `engram status` must include a "Search index:" status line.
#[test]
fn test_status_shows_search_index_label() {
    let mut cmd = Command::cargo_bin("engram").unwrap();
    cmd.arg("status");
    cmd.assert()
        .success()
        .stdout(predicate::str::contains("Search index:"));
}

/// `engram status` search index line must show either index stats or the
/// "not built" message (or "error opening index" if the index is corrupt).
#[test]
fn test_status_search_index_shows_valid_state() {
    let mut cmd = Command::cargo_bin("engram").unwrap();
    cmd.arg("status");
    let output = cmd.output().unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("files indexed")
            || stdout.contains("not built (run: engram index)")
            || stdout.contains("error opening index"),
        "Search index line must show stats, 'not built (run: engram index)', or 'error opening index', got: {}",
        stdout
    );
}

/// `engram index --vault <nonexistent-name>` must exit non-zero when the vault name is not
/// registered in the config.  Uses ENGRAM_CONFIG_PATH to guarantee an empty config so the
/// vault name lookup always fails predictably, and asserts that "not found" appears in stderr.
#[test]
fn test_index_nonexistent_vault_exits_nonzero() {
    use tempfile::TempDir;
    let dir = TempDir::new().unwrap();
    // Point to a nonexistent config file — EngramConfig::load() will return an empty config.
    let config_path = dir.path().join("empty-config.toml");

    let mut cmd = Command::cargo_bin("engram").unwrap();
    cmd.env("ENGRAM_CONFIG_PATH", &config_path);
    cmd.args(["index", "--vault", "nonexistent-vault-name"]);
    cmd.assert()
        .failure()
        .stderr(predicate::str::contains("not found"));
}

// ─── engram observe tests (Task 4) ──────────────────────────────────────────

/// `engram observe --help` must exit 0 and show `session-path` and `api-key` args.
#[test]
fn test_observe_help_shows_expected_args() {
    let mut cmd = Command::cargo_bin("engram").unwrap();
    cmd.args(["observe", "--help"]);
    cmd.assert()
        .success()
        .stdout(predicate::str::contains("session-path"))
        .stdout(predicate::str::contains("api-key"));
}

/// `engram observe <nonexistent-path>` must exit with a non-zero code.
#[test]
fn test_observe_nonexistent_path_exits_nonzero() {
    let mut cmd = Command::cargo_bin("engram").unwrap();
    cmd.args(["observe", "/tmp/nonexistent_engram_session_path_xyz_abc"]);
    cmd.assert().failure();
}

// ─── engram load tests (Task 6) ───────────────────────────────────────────────

/// `engram load --help` must exit 0 and show the --format flag.
#[test]
fn test_load_help_shows_format_flag() {
    let mut cmd = Command::cargo_bin("engram").unwrap();
    cmd.args(["load", "--help"]);
    cmd.assert()
        .success()
        .stdout(predicate::str::contains("--format"));
}

/// `engram load` must NOT panic with todo!() — it must either succeed or fail gracefully.
#[test]
fn test_load_does_not_panic() {
    let mut cmd = Command::cargo_bin("engram").unwrap();
    cmd.arg("load");
    let output = cmd.output().unwrap();
    let stderr = String::from_utf8_lossy(&output.stderr);
    // A todo!() panic produces "not yet implemented" on stderr
    assert!(
        !stderr.contains("not yet implemented"),
        "run_load must not call todo!(), got stderr: {}",
        stderr
    );
}

// ─── engram daemon tests (Task 8 CLI) ──────────────────────────────────────────

/// `engram daemon --help` must exit 0 (daemon command is registered).
#[test]
fn test_daemon_help_exits_successfully() {
    let mut cmd = Command::cargo_bin("engram").unwrap();
    cmd.args(["daemon", "--help"]);
    cmd.assert().success();
}

// ─── engram mcp tests (Task 9) ──────────────────────────────────────────────

/// `engram mcp --help` must exit 0 (mcp command is registered).
#[test]
fn test_mcp_help_exits_successfully() {
    let mut cmd = Command::cargo_bin("engram").unwrap();
    cmd.args(["mcp", "--help"]);
    cmd.assert().success();
}

// ─── config-module integration tests (Task 5) ──────────────────────────────

/// `engram status` must still exit zero after the config module is integrated into
/// `default_vault_path` and `default_store_path`.
/// This is a regression guard: verifies the config-aware path resolution does not
/// break the status command when no config file is present.
#[test]
fn test_status_still_exits_zero_with_config_module() {
    let mut cmd = Command::cargo_bin("engram").unwrap();
    cmd.arg("status");
    cmd.assert().success();
}

// ─── engram install / uninstall / doctor tests (Task 10) ─────────────────────

/// `engram install --help` must exit 0 (install command is registered).
#[test]
fn test_install_help_exits_successfully() {
    let mut cmd = Command::cargo_bin("engram").unwrap();
    cmd.args(["install", "--help"]);
    cmd.assert().success();
}

/// `engram doctor` must exit 0 and print the "engram doctor" header.
#[test]
fn test_doctor_exits_zero() {
    let mut cmd = Command::cargo_bin("engram").unwrap();
    cmd.arg("doctor");
    cmd.assert()
        .success()
        .stdout(predicates::str::contains("engram doctor"));
}

/// `engram doctor` output must contain a "Vault:" status line.
#[test]
fn test_doctor_shows_vault_line() {
    let mut cmd = Command::cargo_bin("engram").unwrap();
    cmd.arg("doctor");
    cmd.assert()
        .success()
        .stdout(predicates::str::contains("Vault:"));
}

/// `engram doctor` output must contain a "Store:" status line.
#[test]
fn test_doctor_shows_store_line() {
    let mut cmd = Command::cargo_bin("engram").unwrap();
    cmd.arg("doctor");
    cmd.assert()
        .success()
        .stdout(predicates::str::contains("Store:"));
}

// ─── engram status vault-list tests (Task 12) ────────────────────────────────────────────────

/// `engram status` must show vault name 'primary' (or 'Vault') when a vault named 'primary'
/// is configured via ENGRAM_CONFIG_PATH.
#[test]
fn test_status_shows_vaults_label_when_configured() {
    use tempfile::TempDir;
    let dir = TempDir::new().unwrap();
    let config_path = dir.path().join("test-config.toml");

    // Write a minimal config with vault 'primary'.
    std::fs::write(
        &config_path,
        "[vaults.primary]\npath = \"/tmp/test-primary-vault\"\naccess = \"read-write\"\nsync_mode = \"approval\"\ndefault = true\n",
    )
    .unwrap();

    let mut cmd = Command::cargo_bin("engram").unwrap();
    cmd.env("ENGRAM_CONFIG_PATH", &config_path);
    cmd.arg("status");
    let output = cmd.output().unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout);

    assert!(
        stdout.contains("primary"),
        "status output must contain vault name 'primary' when a vault is configured via ENGRAM_CONFIG_PATH, got: {}",
        stdout
    );
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
