// Integration tests for macOS Keychain fallback tier
//
// All tests are gated by `#[cfg(target_os = "macos")]` so they are
// compiled and collected only on macOS.  On Linux and Windows the entire
// file is a no-op — `cargo test --workspace` will not fail there.

// ── macOS-only block ────────────────────────────────────────────────────────

#[cfg(target_os = "macos")]
mod macos {
    use std::process::Command;

    /// Manual runbook for verifying the full Keychain passphrase fallback
    /// flow end-to-end.  This test is intentionally `#[ignore]` because it
    /// requires manual setup steps that cannot be automated safely in a CI
    /// environment (interactive GUI session, real keychain writes, etc.).
    ///
    /// # Verification steps
    ///
    /// 1. Ensure no passphrase env vars are set:
    ///    ```sh
    ///    unset ENGRAM_VAULT_PASSPHRASE ENGRAM_VAULT_KEY
    ///    ```
    ///
    /// 2. Add a keychain entry via the `security` CLI:
    ///    ```sh
    ///    security add-generic-password -a engram -s engram-vault -w "keychain-test-pass"
    ///    ```
    ///
    /// 3. Initialise engram using the same passphrase:
    ///    ```sh
    ///    ENGRAM_VAULT_PASSPHRASE="keychain-test-pass" engram init
    ///    ```
    ///
    /// 4. Unset `ENGRAM_VAULT_PASSPHRASE` so the CLI falls through to the
    ///    Keychain tier:
    ///    ```sh
    ///    unset ENGRAM_VAULT_PASSPHRASE
    ///    ```
    ///
    /// 5. Verify that `engram status` succeeds **without prompting** for a
    ///    passphrase (Keychain tier supplies it automatically):
    ///    ```sh
    ///    engram status
    ///    # Expected: exits 0, no interactive prompt
    ///    ```
    ///
    /// 6. Verify that `engram doctor` shows the Keychain tier as active:
    ///    ```sh
    ///    engram doctor
    ///    # Expected output contains: macOS Keychain (security CLI) ✓
    ///    ```
    ///
    /// 7. Remove the keychain entry to verify fall-through:
    ///    ```sh
    ///    security delete-generic-password -a engram -s engram-vault
    ///    ```
    ///
    /// 8. Verify that `engram doctor` now falls through to the interactive
    ///    prompt tier and prints the tip to set `ENGRAM_VAULT_PASSPHRASE`:
    ///    ```sh
    ///    engram doctor
    ///    # Expected: key method shows interactive prompt tier, tip is printed
    ///    ```
    #[test]
    #[ignore = "manual runbook — requires GUI session and real keychain writes; \
                run with: cargo test -- --include-ignored test_keychain_passphrase_fallback_manual_runbook"]
    fn test_keychain_passphrase_fallback_manual_runbook() {
        // This test is documentation only.  All steps must be performed
        // manually in a GUI session that has access to the login keychain.
        // See the doc-comment above for the full step-by-step procedure.
        unimplemented!(
            "manual runbook — follow the steps in the doc-comment above to verify \
             end-to-end Keychain fallback behaviour"
        );
    }

    /// Asserts that the `security` CLI is present and can be invoked.
    ///
    /// On every supported macOS version `/usr/bin/security` ships as part
    /// of the OS.  If this test fails the Keychain fallback tier will not
    /// work on this machine.
    ///
    /// Note: `security --version` is not a recognised flag and returns exit
    /// status 2, but `Command::output()` returning `Ok(_)` is sufficient to
    /// confirm the binary is present and executable.  An `Err(_)` result
    /// would mean the OS could not find or spawn the binary at all.
    #[test]
    fn test_security_cli_is_present() {
        let result = Command::new("security").arg("--version").output();

        assert!(
            result.is_ok(),
            "`security` CLI must be present and executable on macOS; \
             `Command::output()` returned Err: {:?}",
            result.err()
        );
    }

    /// Confirms that `security find-generic-password` returns a non-zero
    /// exit status when the requested entry does not exist.
    ///
    /// This is the signal the Keychain fallback tier relies on to detect a
    /// cache miss and fall through to the next resolution tier (interactive
    /// prompt).  If this contract breaks, the fall-through logic will also
    /// break.
    #[test]
    fn test_security_cli_returns_nonzero_for_missing_entry() {
        let output = Command::new("security")
            .args([
                "find-generic-password",
                "-a",
                "engram",
                "-s",
                "engram-vault-definitely-does-not-exist",
                "-w",
            ])
            .output()
            .expect("failed to spawn `security find-generic-password`");

        assert!(
            !output.status.success(),
            "`security find-generic-password` must exit non-zero when the \
             entry does not exist; got exit status: {}",
            output.status
        );
    }
}
