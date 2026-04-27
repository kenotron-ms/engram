# Engram — macOS Keychain Passphrase Fallback

> **For execution:** Run the subagent-driven-development recipe.

**Goal:** Add macOS Keychain as tier 3 in key resolution — after env vars, before interactive prompt. Store passphrase once with `security add-generic-password`, then `engram sync` and Amplifier hooks work silently forever with no daemon required.

**Architecture:** One `#[cfg(target_os = "macos")]` block in `resolve_vault_key()` calls `security find-generic-password` via subprocess. Falls through silently if not found. Works on Linux/Windows (cfg guard). `engram doctor` updated to show which tier is active and print setup tip when Keychain not configured.

**Tech Stack:** std::process::Command (no new deps), #[cfg(target_os = "macos")] conditional compilation

---

## Prerequisites

All four tasks touch a single file — `crates/engram-cli/src/main.rs` — plus a new test file and the README. Read `crates/engram-cli/src/main.rs` fully before starting; it is large (~2400 lines). All line numbers below were captured from the current `main` branch; verify them with your editor before each step.

---

## Task 1: Add macOS Keychain tier to `resolve_vault_key()`

**Files:**
- Modify: `crates/engram-cli/src/main.rs` (lines 309–353)

### Step 1: Locate the insertion point

Open `crates/engram-cli/src/main.rs`. Find the function `resolve_vault_key()` starting near line 318. Confirm the structure matches:

```
line ~309: /// Resolve the vault encryption key using a three-tier fallback strategy.
line ~311: /// Tier 1 — `ENGRAM_VAULT_KEY` env var ...
line ~313: /// Tier 2 — `ENGRAM_VAULT_PASSPHRASE` env var + salt ...
line ~315: /// Tier 3 — Interactive `rpassword` prompt + salt ...
line ~318: fn resolve_vault_key() -> Result<engram_core::crypto::EngramKey, String> {
...
line ~338: // ── Tier 2: ENGRAM_VAULT_PASSPHRASE env var + config salt
line ~339: if let Ok(passphrase) = std::env::var("ENGRAM_VAULT_PASSPHRASE") {
line ~340:     let salt = load_salt().ok_or_else(...)?;
line ~342:     return engram_core::crypto::EngramKey::derive(passphrase.as_bytes(), &salt)...;
line ~344: }
line ~346: // ── Tier 3: interactive rpassword prompt + config salt
line ~347: let salt = load_salt().ok_or_else(...)?;
line ~349: let passphrase = rpassword::prompt_password("Vault passphrase: ")...;
line ~351: engram_core::crypto::EngramKey::derive(passphrase.as_bytes(), &salt)...
line ~353: }
```

### Step 2: Update the doc comment

Replace the doc comment above `resolve_vault_key()` (the `/// Tier N —` lines, currently listing three tiers) with the four-tier version:

Find:
```rust
/// Resolve the vault encryption key using a three-tier fallback strategy.
///
/// Tier 1 — `ENGRAM_VAULT_KEY` env var: base64-encoded 32 bytes decoded directly
///   into an [`engram_core::crypto::EngramKey`].
/// Tier 2 — `ENGRAM_VAULT_PASSPHRASE` env var + salt from config: the passphrase is
///   derived using Argon2id with the salt stored in the engram config file.
/// Tier 3 — Interactive `rpassword` prompt + salt from config.
///
/// Never panics. Returns a human-friendly `Err(String)` on failure.
```

Replace with:
```rust
/// Resolve the vault encryption key using a four-tier fallback strategy.
///
/// Tier 1 — `ENGRAM_VAULT_KEY` env var: base64-encoded 32 bytes decoded directly
///   into an [`engram_core::crypto::EngramKey`].
/// Tier 2 — `ENGRAM_VAULT_PASSPHRASE` env var + salt from config: the passphrase is
///   derived using Argon2id with the salt stored in the engram config file.
/// Tier 3 — macOS Keychain via `security find-generic-password` (macOS only).
///   Store with: `security add-generic-password -a engram -s engram-vault -w "passphrase"`
/// Tier 4 — Interactive `rpassword` prompt + salt from config.
///
/// Never panics. Returns a human-friendly `Err(String)` on failure.
```

### Step 3: Insert the Keychain block

Immediately after the closing `}` of the Tier 2 block (the line containing just `}` after the `return engram_core::crypto::EngramKey::derive(...)` call) and before the `// ── Tier 3: interactive rpassword prompt` comment, insert:

```rust
    // ── Tier 3 (macOS only): read passphrase from macOS Keychain via security CLI ──────
    // Store with: security add-generic-password -a engram -s engram-vault -w "passphrase"
    #[cfg(target_os = "macos")]
    {
        let output = std::process::Command::new("security")
            .args(["find-generic-password", "-a", "engram", "-s", "engram-vault", "-w"])
            .output();
        if let Ok(out) = output {
            if out.status.success() {
                let passphrase = String::from_utf8_lossy(&out.stdout).trim().to_string();
                if !passphrase.is_empty() {
                    let salt = load_salt()
                        .ok_or_else(|| "No salt found in config. Run: engram init".to_string())?;
                    return engram_core::crypto::EngramKey::derive(passphrase.as_bytes(), &salt)
                        .map_err(|e| format!("Key derivation failed: {}", e));
                }
            }
        }
        // security CLI not found or no matching entry — fall through to interactive prompt
    }

```

> **Note on `salt`:** The Keychain block loads the salt independently via `load_salt()`. Do not reference the `salt` variable from Tier 4 (it is declared after this block). The `load_salt` closure is defined earlier in the function and is in scope here.

The updated Tier 4 comment line immediately below the new block should also be updated. Find:
```rust
    // ── Tier 3: interactive rpassword prompt + config salt
```
Change to:
```rust
    // ── Tier 4: interactive rpassword prompt + config salt
```

### Step 4: Verify the build compiles on all platforms

Run:
```bash
cd ~/workspace/ms/engram
cargo build --workspace 2>&1 | head -40
```
Expected: zero errors, zero warnings. The `#[cfg(target_os = "macos")]` guard ensures the block compiles away on Linux and Windows.

### Step 5: Run the existing unit tests

Run:
```bash
cargo test -p engram-cli --lib -- resolve_vault_key --nocapture 2>&1
```
Expected: all four existing `resolve_vault_key` tests pass (they manipulate env vars and a temp config path; the new Keychain tier falls through when `security` finds no entry named `engram-vault` in a fresh test environment).

> **Local dev caveat:** If you have previously run `security add-generic-password -a engram -s engram-vault -w "..."` on this machine, the test `test_resolve_key_fails_gracefully_when_not_initialized` will now succeed at Tier 3 instead of failing at the salt check, breaking its assertion. Delete the Keychain entry before running tests: `security delete-generic-password -a engram -s engram-vault`.

### Step 6: Commit

```bash
git add crates/engram-cli/src/main.rs
git commit -m "feat(cli): add macOS Keychain as tier 3 in resolve_vault_key()"
```

---

## Task 2: Update `run_doctor()` to show Keychain status and print setup tip

**Files:**
- Modify: `crates/engram-cli/src/main.rs` (lines ~1674–1684)

### Step 1: Locate the key method block

Find `run_doctor()` (starts near line 1644). Scroll to the `// ── Key method` comment (near line 1674). Confirm the current block looks like:

```rust
    // ── Key method ──────────────────────────────────────────────────────────────────
    let key_method = if std::env::var("ENGRAM_VAULT_KEY").is_ok() {
        "ENGRAM_VAULT_KEY"
    } else if std::env::var("ENGRAM_VAULT_PASSPHRASE").is_ok() {
        "ENGRAM_VAULT_PASSPHRASE"
    } else if config.key.salt.is_some() {
        "config salt"
    } else {
        "not initialized"
    };
    println!("Key:               {}", key_method);
```

### Step 2: Replace the key method block

Replace the entire block (from the `// ── Key method` comment through the `println!("Key: ...")` line) with:

```rust
    // ── Key method ──────────────────────────────────────────────────────────────────
    #[cfg(target_os = "macos")]
    let keychain_available = std::process::Command::new("security")
        .args(["find-generic-password", "-a", "engram", "-s", "engram-vault", "-w"])
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false);

    let key_method: String = if std::env::var("ENGRAM_VAULT_KEY").is_ok() {
        "ENGRAM_VAULT_KEY env var ✓".to_string()
    } else if std::env::var("ENGRAM_VAULT_PASSPHRASE").is_ok() {
        "ENGRAM_VAULT_PASSPHRASE env var ✓".to_string()
    } else {
        #[cfg(target_os = "macos")]
        if keychain_available {
            "macOS Keychain (security CLI) ✓".to_string()
        } else if config.key.salt.is_some() {
            "interactive passphrase prompt (salt configured) ✓".to_string()
        } else {
            "not initialized ✗ — run: engram init".to_string()
        }
        #[cfg(not(target_os = "macos"))]
        {
            if config.key.salt.is_some() {
                "interactive passphrase prompt (salt configured) ✓".to_string()
            } else {
                "not initialized ✗ — run: engram init".to_string()
            }
        }
    };
    println!("Key:               {}", key_method);

    // Tip: when on macOS and no Keychain entry is configured, guide the user.
    #[cfg(target_os = "macos")]
    if !std::env::var("ENGRAM_VAULT_KEY").is_ok()
        && !std::env::var("ENGRAM_VAULT_PASSPHRASE").is_ok()
        && !keychain_available
    {
        println!(
            "Tip: store passphrase in macOS Keychain for silent operation:"
        );
        println!(
            "  security add-generic-password -a engram -s engram-vault -w \"your-passphrase\""
        );
    }
```

> **Note on `config`:** The variable `config` is already loaded at the top of `run_doctor()` (`let config = EngramConfig::load();`). Use it directly — do not call `EngramConfig::load()` again inside this block.

> **Note on `key_method` type:** The old code inferred `&str`; the new code uses `String`. The `println!` macro works with both, so the print line is unchanged.

### Step 3: Verify the build

```bash
cargo build --workspace 2>&1 | head -40
```
Expected: zero errors, zero warnings.

### Step 4: Smoke test `engram doctor` output

```bash
cargo run -p engram-cli -- doctor 2>&1
```
Verify the `Key:` line reflects your current setup:
- If `ENGRAM_VAULT_KEY` is set → `ENGRAM_VAULT_KEY env var ✓`
- If `ENGRAM_VAULT_PASSPHRASE` is set → `ENGRAM_VAULT_PASSPHRASE env var ✓`
- If Keychain entry exists → `macOS Keychain (security CLI) ✓`
- If neither env var nor Keychain → key method shows `interactive passphrase prompt` or `not initialized`, and the tip is printed

### Step 5: Commit

```bash
git add crates/engram-cli/src/main.rs
git commit -m "feat(cli): show Keychain tier in engram doctor, print setup tip"
```

---

## Task 3: Add integration test file for Keychain fallback

**Files:**
- Create: `crates/engram-cli/tests/keychain_test.rs`

### Step 1: Write the test file

Create `crates/engram-cli/tests/keychain_test.rs` with the following content:

```rust
// Keychain integration tests — macOS only, #[ignore] by default.
//
// These tests require real macOS Keychain access and must be run manually
// on a developer machine. They document the intended behavior of Tier 3
// key resolution and serve as a runbook for manual verification.
//
// Before running:
//   security add-generic-password -a engram -s engram-vault -w "keychain-test-pass"
// After running (cleanup):
//   security delete-generic-password -a engram -s engram-vault

/// Tier 3 (macOS Keychain) resolves the key silently without a prompt.
///
/// Manual verification steps:
/// 1. Ensure no ENGRAM_VAULT_KEY or ENGRAM_VAULT_PASSPHRASE env var is set.
/// 2. Ensure the Keychain entry exists:
///      security add-generic-password -a engram -s engram-vault -w "keychain-test-pass"
/// 3. Ensure engram is initialized with the same passphrase:
///      ENGRAM_VAULT_PASSPHRASE=keychain-test-pass engram init
/// 4. Unset ENGRAM_VAULT_PASSPHRASE.
/// 5. Run `engram status` — it should succeed without prompting.
/// 6. Run `engram doctor` — Key line should read "macOS Keychain (security CLI) ✓".
/// 7. Delete the Keychain entry:
///      security delete-generic-password -a engram -s engram-vault
/// 8. Run `engram doctor` again — should fall through to interactive prompt tier
///    and print the Keychain setup tip.
#[test]
#[cfg(target_os = "macos")]
#[ignore = "requires real macOS Keychain access — run manually"]
fn test_keychain_passphrase_fallback_manual_runbook() {
    println!("Manual test: see doc comment above for step-by-step verification.");
    println!("This test exists to document intended behavior and cannot be");
    println!("automated without a real Keychain entry in the test environment.");
}

/// Verify that the `security` CLI is available on this macOS machine.
///
/// If this test fails, the Keychain tier will always be skipped at runtime.
#[test]
#[cfg(target_os = "macos")]
fn test_security_cli_is_present() {
    let output = std::process::Command::new("security")
        .arg("--version")
        .output();
    assert!(
        output.is_ok(),
        "`security` CLI not found — macOS Keychain tier will not function"
    );
}

/// Verify that a missing Keychain entry causes `security` to exit non-zero
/// (i.e., the fall-through behaviour is correct).
///
/// Uses a service name that should never exist: "engram-vault-definitely-does-not-exist".
#[test]
#[cfg(target_os = "macos")]
fn test_security_cli_returns_nonzero_for_missing_entry() {
    let output = std::process::Command::new("security")
        .args([
            "find-generic-password",
            "-a",
            "engram",
            "-s",
            "engram-vault-definitely-does-not-exist",
            "-w",
        ])
        .output()
        .expect("`security` CLI must be present on macOS");

    assert!(
        !output.status.success(),
        "security should return non-zero for a missing Keychain entry"
    );
}
```

### Step 2: Run the non-ignored tests

```bash
cargo test -p engram-cli --test keychain_test 2>&1
```
Expected output: two tests pass (`test_security_cli_is_present`, `test_security_cli_returns_nonzero_for_missing_entry`), one test ignored (`test_keychain_passphrase_fallback_manual_runbook`).

On Linux/Windows: all three tests are excluded by `#[cfg(target_os = "macos")]` and show as filtered/not collected — that is correct.

### Step 3: Verify full test suite still passes

```bash
cargo test --workspace 2>&1 | tail -20
```
Expected: no failures.

### Step 4: Commit

```bash
git add crates/engram-cli/tests/keychain_test.rs
git commit -m "test(cli): add macOS Keychain integration test file"
```

---

## Task 4: Update README with Passphrase Setup section

**Files:**
- Modify: `README.md`

### Step 1: Locate the insertion point

Open `README.md`. Find the end of the `## Quick Start` section — the closing triple-backtick of the code block (near line 28), just before `## CLI Reference` (near line 30).

### Step 2: Insert the new section

After the closing ` ``` ` of the Quick Start code block and before `## CLI Reference`, insert:

```markdown

## Passphrase Setup

engram uses a passphrase to derive your vault encryption key. On macOS, store it once in Keychain for silent operation — no prompt needed for `engram sync`, daemon, or Amplifier hooks:

```bash
# Store passphrase in macOS Keychain (terminal only, no GUI popup)
security add-generic-password -a engram -s engram-vault -w "your-passphrase"

# engram now works silently — no prompt needed
engram sync
engram awareness
```

**Other options:**
```bash
# Env var (good for remote servers, CI, and Docker)
export ENGRAM_VAULT_PASSPHRASE="your-passphrase"

# Pre-derived raw key (good for CI/automation — generate once, store securely)
export ENGRAM_VAULT_KEY="$(engram init --print-key)"   # if supported, or see engram doctor
```

Run `engram doctor` to see which tier is active and get setup instructions.

```

The full section, shown in context (surrounding lines included for reference — do not duplicate them):

```
...
engram search "Sofia dietary needs"  # hybrid semantic + full-text search
```                                  ← end of Quick Start block

## Passphrase Setup               ← INSERT HERE

engram uses a passphrase ...
...
Run `engram doctor` ...

## CLI Reference                  ← existing section continues unchanged
```

### Step 3: Verify README renders correctly

```bash
cat README.md | head -80
```
Confirm the new section appears between Quick Start and CLI Reference with correct markdown fencing.

### Step 4: Final build and lint check

```bash
cd ~/workspace/ms/engram
cargo build --workspace && \
cargo clippy --workspace --all-targets -- -D warnings && \
cargo fmt --all -- --check
```
Expected: all pass with zero errors.

### Step 5: Final test run

```bash
cargo test --workspace 2>&1 | tail -30
```
Expected: all tests pass, no regressions.

### Step 6: Push

```bash
git add README.md
git commit -m "docs: add Passphrase Setup section with macOS Keychain instructions"
git pull --rebase origin main && git push origin main
```

---

## Summary of Changes

| File | Change |
|------|--------|
| `crates/engram-cli/src/main.rs` | Updated `resolve_vault_key()` doc comment (3→4 tiers); inserted `#[cfg(target_os = "macos")]` Keychain block between Tier 2 and Tier 4; updated `run_doctor()` key method to `String` type with Keychain detection and setup tip |
| `crates/engram-cli/tests/keychain_test.rs` | New file: `security` CLI availability test, missing-entry fall-through test, manual runbook (ignored) |
| `README.md` | New `## Passphrase Setup` section between Quick Start and CLI Reference |

**No new dependencies.** The `security` CLI is a macOS system binary available on all modern macOS versions. All new code is behind `#[cfg(target_os = "macos")]`; Linux and Windows builds are unaffected.
