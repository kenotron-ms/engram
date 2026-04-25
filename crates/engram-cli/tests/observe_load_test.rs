// Integration tests — observe + load round-trip
//
// These tests write `Memory` records directly into a temporary encrypted store,
// then run the compiled `engram` binary with the `ENGRAM_STORE_PATH` environment
// variable pointing at that store, and verify the output of
// `engram load --format=context`.
//
// Tests that require system-keychain access gracefully skip on headless machines
// (CI without a GUI session) where the platform keychain is unavailable.

use assert_cmd::Command;
use engram_core::crypto::{EngramKey, KeyStore};
use engram_core::store::{Memory, MemoryStore};
use tempfile::TempDir;

// ─── Helpers ────────────────────────────────────────────────────────────────

/// Derive a deterministic test encryption key from a fixed password and an
/// all-zero 16-byte salt.
fn test_key() -> EngramKey {
    EngramKey::derive(b"testpassword", &[0u8; 16]).expect("key derivation failed")
}

/// Create a temporary directory, open a `MemoryStore` inside it, and return the
/// absolute path as a `String`.
///
/// The returned `TempDir` **must** stay in scope for as long as the store (or
/// the path string) is needed; dropping it deletes the directory on disk.
fn temp_store_with_path() -> (TempDir, MemoryStore, String) {
    let dir = TempDir::new().expect("create temp dir failed");
    let path = dir.path().join("test.db");
    let path_str = path.to_str().expect("non-UTF-8 temp path").to_string();
    let store = MemoryStore::open(&path, &test_key()).expect("open store failed");
    (dir, store, path_str)
}

/// Attempt to write the test key into the platform keychain under the `"engram"`
/// service name.
///
/// Returns `true` on success.  Returns `false` when the keychain is unavailable
/// (headless CI, no unlocked macOS Keychain session, etc.).  Tests that need the
/// keychain call this helper and return early on `false`.
fn install_test_key() -> bool {
    KeyStore::new("engram").store(&test_key()).is_ok()
}

// ─── Tests ──────────────────────────────────────────────────────────────────

/// Write three facts for two entities, close the store, run `engram load`, and
/// verify that every expected piece of information appears in the output — and
/// that Sofia's two facts land on the same entity line.
#[test]
fn test_load_context_round_trip_three_facts_two_entities() {
    if !install_test_key() {
        eprintln!(
            "skipping test_load_context_round_trip_three_facts_two_entities: keychain unavailable"
        );
        return;
    }

    let (dir, store, path) = temp_store_with_path();

    store
        .insert(&Memory::new("Sofia", "dietary", "vegetarian", None))
        .expect("insert Sofia/dietary failed");
    store
        .insert(&Memory::new("Sofia", "location", "Seattle", None))
        .expect("insert Sofia/location failed");
    store
        .insert(&Memory::new(
            "Chris Park",
            "preference",
            "small focused components",
            None,
        ))
        .expect("insert Chris Park/preference failed");

    // Close the store before invoking the binary so SQLCipher can acquire an
    // exclusive write lock on its own.
    drop(store);

    let output = Command::cargo_bin("engram")
        .unwrap()
        .args(["load", "--format=context"])
        .env("ENGRAM_STORE_PATH", &path)
        .output()
        .expect("failed to run engram binary");

    let stdout = String::from_utf8_lossy(&output.stdout);

    assert!(
        stdout.contains("<engram-context>"),
        "output should contain <engram-context> opening tag, got: {}",
        stdout
    );
    assert!(
        stdout.contains("Sofia"),
        "output should contain 'Sofia', got: {}",
        stdout
    );
    assert!(
        stdout.contains("Chris Park"),
        "output should contain 'Chris Park', got: {}",
        stdout
    );
    assert!(
        stdout.contains("vegetarian"),
        "output should contain 'vegetarian', got: {}",
        stdout
    );
    assert!(
        stdout.contains("small focused components"),
        "output should contain 'small focused components', got: {}",
        stdout
    );

    // Sofia's two facts must appear on a *single* entity line (not split across lines).
    let sofia_line = stdout
        .lines()
        .find(|l| l.starts_with("- Sofia:"))
        .expect("output should have a '- Sofia:' entity line");

    assert!(
        sofia_line.contains("dietary: vegetarian"),
        "Sofia's entity line should contain 'dietary: vegetarian', got: {}",
        sofia_line
    );
    assert!(
        sofia_line.contains("location: Seattle"),
        "Sofia's entity line should contain 'location: Seattle', got: {}",
        sofia_line
    );

    drop(dir); // keep the temp directory alive until here
}

/// Write five distinct facts for the same entity (Sofia), run `engram load`, and
/// verify that all five values appear in the output and that Sofia occupies
/// exactly one entity line.
#[test]
fn test_load_context_groups_multiple_facts_for_same_entity() {
    if !install_test_key() {
        eprintln!("skipping test_load_context_groups_multiple_facts_for_same_entity: keychain unavailable");
        return;
    }

    let (dir, store, path) = temp_store_with_path();

    for (attr, val) in &[
        ("dietary", "vegetarian"),
        ("location", "Seattle"),
        ("team", "Team Pulse"),
        ("role", "senior engineer"),
        ("hobby", "bouldering"),
    ] {
        store
            .insert(&Memory::new("Sofia", attr, val, None))
            .unwrap_or_else(|_| panic!("insert Sofia/{attr} failed"));
    }

    drop(store);

    let output = Command::cargo_bin("engram")
        .unwrap()
        .args(["load", "--format=context"])
        .env("ENGRAM_STORE_PATH", &path)
        .output()
        .expect("failed to run engram binary");

    let stdout = String::from_utf8_lossy(&output.stdout);

    // Every value must appear somewhere in the output.
    for value in &[
        "vegetarian",
        "Seattle",
        "Team Pulse",
        "senior engineer",
        "bouldering",
    ] {
        assert!(
            stdout.contains(value),
            "output should contain '{}', got: {}",
            value,
            stdout
        );
    }

    // Exactly one line must begin with "- Sofia:".
    let sofia_line_count = stdout.lines().filter(|l| l.starts_with("- Sofia:")).count();
    assert_eq!(
        sofia_line_count, 1,
        "there should be exactly 1 Sofia entity line; found {}",
        sofia_line_count
    );

    drop(dir);
}

/// Open an empty store, run `engram load`, and verify the "No recent memories"
/// message appears in the output.
#[test]
fn test_load_context_empty_store_shows_no_memories_message() {
    if !install_test_key() {
        eprintln!(
            "skipping test_load_context_empty_store_shows_no_memories_message: keychain unavailable"
        );
        return;
    }

    let (dir, store, path) = temp_store_with_path();
    drop(store); // close without inserting anything

    let output = Command::cargo_bin("engram")
        .unwrap()
        .args(["load", "--format=context"])
        .env("ENGRAM_STORE_PATH", &path)
        .output()
        .expect("failed to run engram binary");

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("No recent memories"),
        "empty store should produce 'No recent memories' in output, got: {}",
        stdout
    );

    drop(dir);
}
