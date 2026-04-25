// Integration tests — Full Phase 1 round-trip
//
// These tests exercise the complete Phase 1 stack end-to-end:
//   - Key derivation → MemoryStore CRUD (test_full_memory_round_trip)
//   - Encrypt/decrypt round-trip with a known salt (test_encrypt_vault_file_round_trip)
//   - Vault write / read / list (test_vault_reads_after_write)

use engram_core::{
    crypto::{decrypt, encrypt, generate_salt, EngramKey},
    store::{Memory, MemoryStore},
    vault::Vault,
};
use tempfile::TempDir;

/// Derive key with a random salt, open a MemoryStore, insert a Memory, and
/// verify the retrieved record has the expected field values.
#[test]
fn test_full_memory_round_trip() {
    let dir = TempDir::new().expect("create tempdir failed");
    let db_path = dir.path().join("memories.db");

    // Derive key from a randomly-generated salt
    let salt = generate_salt();
    let key = EngramKey::derive(b"integration-test-password", &salt)
        .expect("key derivation failed");

    let store = MemoryStore::open(&db_path, &key).expect("open store failed");

    let memory = Memory::new(
        "Ken",
        "preference",
        "small focused components",
        Some("2026-04-14 Chris Park transcript"),
    );
    store.insert(&memory).expect("insert failed");

    let got = store
        .get(&memory.id)
        .expect("get failed")
        .expect("memory should be present");

    assert_eq!(got.entity, "Ken");
    assert_eq!(got.value, "small focused components");
    assert_eq!(
        got.source,
        Some("2026-04-14 Chris Park transcript".to_string()),
        "source should match the original"
    );
}

/// Derive a key from a fixed salt, encrypt a markdown string, assert the
/// ciphertext is longer than the plaintext, then decrypt and compare bytes.
#[test]
fn test_encrypt_vault_file_round_trip() {
    let salt = [7u8; 16];
    let key = EngramKey::derive(b"integration-test-password", &salt)
        .expect("key derivation failed");

    let plaintext = "# People/Sofia.md\n\nSofia is vegetarian.\n";
    let plaintext_bytes = plaintext.as_bytes();

    let ciphertext = encrypt(&key, plaintext_bytes).expect("encrypt failed");

    assert!(
        ciphertext.len() > plaintext_bytes.len(),
        "ciphertext ({} bytes) must be longer than plaintext ({} bytes)",
        ciphertext.len(),
        plaintext_bytes.len(),
    );

    let decrypted = decrypt(&key, &ciphertext).expect("decrypt failed");

    assert_eq!(
        decrypted, plaintext_bytes,
        "decrypted bytes must match original plaintext"
    );
}

/// Write three markdown files into a TempDir-backed Vault, verify that
/// `list_markdown` returns exactly three entries, and that reading
/// `People/Sofia.md` back yields the expected content.
#[test]
fn test_vault_reads_after_write() {
    let dir = TempDir::new().expect("create tempdir failed");
    let vault = Vault::new(dir.path());

    vault
        .write("People/Sofia.md", "# Sofia\n\nSofia is vegetarian.")
        .expect("write People/Sofia.md failed");
    vault
        .write(
            "Work/Notes/meeting.md",
            "# Meeting Notes\n\nDiscussed project timeline.",
        )
        .expect("write Work/Notes/meeting.md failed");
    vault
        .write("Tasks.md", "# Tasks\n\n- [ ] Review PRs")
        .expect("write Tasks.md failed");

    let files = vault.list_markdown().expect("list_markdown failed");
    assert_eq!(
        files.len(),
        3,
        "expected 3 markdown files, got: {:?}",
        files
    );

    let content = vault
        .read("People/Sofia.md")
        .expect("read People/Sofia.md failed");
    assert_eq!(
        content,
        "# Sofia\n\nSofia is vegetarian.",
        "People/Sofia.md content should match what was written"
    );
}
