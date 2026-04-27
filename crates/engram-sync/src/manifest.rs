// crates/engram-sync/src/manifest.rs — rclone-style delta detection for engram sync.
//
// Decision tree (cheapest → most expensive), mirroring rclone's equal() logic:
//
//   1. size + mtime  (one fs::metadata() syscall, zero file reads)
//      → unchanged → SKIP immediately
//
//   2. content hash  (file read + SHA-256, only when size/mtime differ)
//      → same hash → SKIP (file was touched by editor but content identical)
//      → different  → UPLOAD
//
// Because engram encrypts with a random nonce on every push, the remote-side
// ETag/hash is useless for deduplication — the same plaintext produces a
// different ciphertext every time.  The manifest IS the "remote state" from
// engram's perspective: it records what was successfully pushed and what the
// plaintext looked like at that moment.
//
// Stored at: ~/.engram/<vault_name>/sync-manifest.json

use std::collections::HashMap;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

/// Per-file record written after a successful push.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileEntry {
    /// File size in bytes at last successful push.
    pub size: u64,
    /// Modification time seconds component (UNIX epoch).
    pub mtime_secs: u64,
    /// Modification time nanoseconds sub-second component.
    pub mtime_nanos: u32,
    /// SHA-256 hex of the *plaintext* content at last successful push.
    pub hash: String,
}

/// Persistent manifest of what has been successfully synced.
#[derive(Debug, Default, Serialize, Deserialize)]
pub struct SyncManifest {
    /// relative_path → FileEntry
    pub files: HashMap<String, FileEntry>,
}

impl SyncManifest {
    // ── Storage ───────────────────────────────────────────────────────────────

    /// Returns the manifest path: `$HOME/.engram/<vault_name>/sync-manifest.json`.
    pub fn storage_path(vault_name: &str) -> PathBuf {
        let home = std::env::var("HOME")
            .map(PathBuf::from)
            .unwrap_or_else(|_| PathBuf::from("."));
        home.join(".engram")
            .join(vault_name)
            .join("sync-manifest.json")
    }

    /// Load manifest from disk.  Returns empty manifest if not found or corrupt.
    pub fn load(vault_name: &str) -> Self {
        let path = Self::storage_path(vault_name);
        match std::fs::read_to_string(&path) {
            Ok(data) => serde_json::from_str(&data).unwrap_or_default(),
            Err(_) => Self::default(),
        }
    }

    /// Persist manifest to disk, creating parent directories as needed.
    /// Called after each successful batch of uploads so partial progress is
    /// preserved when a sync is interrupted.
    pub fn save(&self, vault_name: &str) -> std::io::Result<()> {
        let path = Self::storage_path(vault_name);
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let data = serde_json::to_string_pretty(self)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;
        std::fs::write(path, data)
    }

    // ── Decision functions ────────────────────────────────────────────────────

    /// **Fast path** — size + mtime only, zero file reads.
    ///
    /// Returns `true` (skip) when size and mtime both match the manifest.
    /// This is the same check rclone performs in its default (non-`--checksum`) mode.
    pub fn is_fast_match(&self, path: &str, size: u64, mtime_secs: u64, mtime_nanos: u32) -> bool {
        self.files.get(path).is_some_and(|e| {
            e.size == size && e.mtime_secs == mtime_secs && e.mtime_nanos == mtime_nanos
        })
    }

    /// **Hash path** — content equality after reading the file.
    ///
    /// Returns `true` (skip) when the plaintext hash matches the manifest.
    /// Equivalent to rclone's `--checksum` fallback: mtime drifted (e.g. a
    /// `touch` or editor save with no real edit) but content is identical.
    pub fn is_hash_match(&self, path: &str, hash: &str) -> bool {
        self.files.get(path).is_some_and(|e| e.hash == hash)
    }

    // ── Mutation ──────────────────────────────────────────────────────────────

    /// Update size + mtime for a file whose content hash is unchanged.
    /// Avoids a re-read on the next sync when only the mtime drifted.
    pub fn update_fast_path(&mut self, path: String, size: u64, mtime_secs: u64, mtime_nanos: u32) {
        if let Some(entry) = self.files.get_mut(&path) {
            entry.size = size;
            entry.mtime_secs = mtime_secs;
            entry.mtime_nanos = mtime_nanos;
        }
    }

    /// Record a completed upload.
    pub fn mark_synced(&mut self, path: String, entry: FileEntry) {
        self.files.insert(path, entry);
    }

    // ── Utilities ─────────────────────────────────────────────────────────────

    /// SHA-256 hex digest of `content`.
    pub fn content_hash(content: &[u8]) -> String {
        let mut hasher = Sha256::new();
        hasher.update(content);
        hex::encode(hasher.finalize())
    }

    /// Extract (secs, nanos) from a `SystemTime`, returning (0, 0) on error.
    pub fn mtime_components(mtime: SystemTime) -> (u64, u32) {
        mtime
            .duration_since(UNIX_EPOCH)
            .map(|d| (d.as_secs(), d.subsec_nanos()))
            .unwrap_or((0, 0))
    }
}
