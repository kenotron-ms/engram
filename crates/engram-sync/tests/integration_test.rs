// Integration tests for engram-sync.
//
// These tests use LocalFileSystem as a stand-in for real cloud backends.
// The ObjectStore trait is identical; only the transport changes in production.
// No cloud credentials are required to run these tests.

use engram_core::crypto::EngramKey;
use engram_core::vault::Vault;
use engram_sync::{
    encrypt::{decrypt_from_sync, encrypt_for_sync},
    s3::S3Backend,
    SyncBackend,
};
use object_store::local::LocalFileSystem;
use std::sync::Arc;
use tempfile::TempDir;

fn test_key_a() -> EngramKey {
    // EngramKey::derive takes &[u8], not &str
    EngramKey::derive(b"integration-test-sync-key-a", &[1u8; 16]).unwrap()
}

fn test_key_b() -> EngramKey {
    EngramKey::derive(b"integration-test-sync-key-b", &[2u8; 16]).unwrap()
}

fn make_backend(dir: &TempDir) -> S3Backend {
    let store = LocalFileSystem::new_with_prefix(dir.path()).unwrap();
    S3Backend::from_store(Arc::new(store), "integration-test".to_string())
}

#[tokio::test]
async fn test_encrypt_push_pull_decrypt_roundtrip() {
    let store_dir = TempDir::new().unwrap();
    let vault_dir = TempDir::new().unwrap();

    // Write a markdown file to the vault
    let vault = Vault::new(vault_dir.path());
    vault
        .write(
            "People/Sofia.md",
            "Sofia is vegetarian.\nSofia lives in Seattle.",
        )
        .unwrap();

    let key = test_key_a();
    let backend = make_backend(&store_dir);

    // Encrypt and push
    let content = vault.read("People/Sofia.md").unwrap();
    let encrypted = encrypt_for_sync(&key, content.as_bytes()).unwrap();
    backend.push("People/Sofia.md", encrypted).await.unwrap();

    // Pull and decrypt
    let pulled = backend.pull("People/Sofia.md").await.unwrap();
    let decrypted = decrypt_from_sync(&key, &pulled).unwrap();
    let decrypted_str = std::str::from_utf8(&decrypted).unwrap();

    assert_eq!(
        decrypted_str,
        "Sofia is vegetarian.\nSofia lives in Seattle."
    );
}

#[tokio::test]
async fn test_wrong_key_cannot_decrypt() {
    let store_dir = TempDir::new().unwrap();
    let backend = make_backend(&store_dir);

    let key_a = test_key_a();
    let key_b = test_key_b();

    // Encrypt with key_a, attempt decrypt with key_b
    let encrypted = encrypt_for_sync(&key_a, b"secret content").unwrap();
    backend.push("secret.md", encrypted).await.unwrap();

    let pulled = backend.pull("secret.md").await.unwrap();
    assert!(
        decrypt_from_sync(&key_b, &pulled).is_err(),
        "Decrypting with the wrong key must fail"
    );
}

#[tokio::test]
async fn test_list_after_multiple_pushes() {
    let dir = TempDir::new().unwrap();
    let backend = make_backend(&dir);
    let key = test_key_a();

    backend
        .push(
            "vault/Work/notes.md",
            encrypt_for_sync(&key, b"notes").unwrap(),
        )
        .await
        .unwrap();
    backend
        .push(
            "vault/People/Sofia.md",
            encrypt_for_sync(&key, b"sofia").unwrap(),
        )
        .await
        .unwrap();
    backend
        .push("vault/Tasks.md", encrypt_for_sync(&key, b"tasks").unwrap())
        .await
        .unwrap();

    let all = backend.list("vault").await.unwrap();
    assert_eq!(
        all.len(),
        3,
        "Expected 3 objects under vault/, got: {:?}",
        all
    );
}

#[tokio::test]
async fn test_encrypted_blob_not_plaintext() {
    let dir = TempDir::new().unwrap();
    let backend = make_backend(&dir);
    let key = test_key_a();

    let plaintext = b"Personal: Sofia is vegetarian.";
    let encrypted = encrypt_for_sync(&key, plaintext).unwrap();
    backend.push("People/Sofia.md", encrypted).await.unwrap();

    let stored = backend.pull("People/Sofia.md").await.unwrap();

    // The stored bytes must NOT contain the plaintext
    assert!(
        !stored.windows(plaintext.len()).any(|w| w == plaintext),
        "Stored blob must not contain plaintext"
    );
    // But must contain the ENGRAM_V1 magic prefix
    assert!(
        stored.starts_with(b"ENGRAM_V1:"),
        "Stored blob must start with ENGRAM_V1: magic prefix"
    );
}
