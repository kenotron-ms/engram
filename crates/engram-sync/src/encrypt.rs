// crates/engram-sync/src/encrypt.rs

use crate::backend::SyncError;
use bytes::Bytes;
use engram_core::crypto::{decrypt, encrypt, EngramKey};

pub const MAGIC: &[u8] = b"ENGRAM_V1:";

/// Encrypt plaintext for cloud sync.
///
/// Prepends a `ENGRAM_V1:` magic prefix so encrypted blobs are
/// identifiable without attempting decryption. The actual encryption
/// is delegated to `engram_core::crypto::encrypt` (XChaCha20-Poly1305).
pub fn encrypt_for_sync(key: &EngramKey, plaintext: &[u8]) -> Result<Bytes, SyncError> {
    let ciphertext = encrypt(key, plaintext).map_err(|e| SyncError::Encryption(e.to_string()))?;
    let mut output = Vec::with_capacity(MAGIC.len() + ciphertext.len());
    output.extend_from_slice(MAGIC);
    output.extend_from_slice(&ciphertext);
    Ok(Bytes::from(output))
}

/// Decrypt a blob produced by `encrypt_for_sync`.
///
/// Returns an error if the magic prefix is missing (blob was not produced
/// by this system) or if decryption fails (wrong key or tampered data).
pub fn decrypt_from_sync(key: &EngramKey, data: &[u8]) -> Result<Vec<u8>, SyncError> {
    if !data.starts_with(MAGIC) {
        return Err(SyncError::Encryption(
            "Not an engram-encrypted blob (missing magic prefix)".into(),
        ));
    }
    let ciphertext = &data[MAGIC.len()..];
    decrypt(key, ciphertext).map_err(|e| SyncError::Encryption(e.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use engram_core::crypto::EngramKey;

    fn test_key() -> EngramKey {
        // EngramKey::derive takes &[u8] (byte slice), not &str
        EngramKey::derive(b"sync-test-password", &[0u8; 16]).unwrap()
    }

    #[test]
    fn test_encrypt_decrypt_roundtrip() {
        let key = test_key();
        let plaintext = b"# Sofia.md\n\nSofia is vegetarian.";
        let encrypted = encrypt_for_sync(&key, plaintext).unwrap();
        let decrypted = decrypt_from_sync(&key, &encrypted).unwrap();
        assert_eq!(decrypted, plaintext);
    }

    #[test]
    fn test_encrypted_has_magic_prefix() {
        let key = test_key();
        let encrypted = encrypt_for_sync(&key, b"hello").unwrap();
        assert!(encrypted.starts_with(MAGIC));
    }

    #[test]
    fn test_missing_magic_returns_error() {
        let key = test_key();
        let result = decrypt_from_sync(&key, b"raw-bytes-without-prefix");
        assert!(result.is_err());
    }

    #[test]
    fn test_tampered_ciphertext_returns_error() {
        let key = test_key();
        let mut encrypted = encrypt_for_sync(&key, b"secret").unwrap().to_vec();
        // Flip a bit in the ciphertext region (past the 10-byte magic + 24-byte nonce)
        encrypted[40] ^= 0xFF;
        assert!(decrypt_from_sync(&key, &encrypted).is_err());
    }
}
