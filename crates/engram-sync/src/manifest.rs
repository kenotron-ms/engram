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
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
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
            .map_err(std::io::Error::other)?;
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

/// Remote file metadata (from cloud listing).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct RemoteFileEntry {
    pub size: u64,
    pub mtime_secs: u64,
    pub etag: Option<String>,
}

/// Persistent bisync state stored at ~/.engram/<vault>/bisync-state.json.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct BiSyncState {
    /// Snapshot of local+remote state at last successful sync — the "common ancestor"
    pub baseline: SyncManifest,
    /// Last-known remote file listing (refreshed at start of each bisync)
    pub remote: HashMap<String, RemoteFileEntry>,
}

impl BiSyncState {
    pub fn load(state_path: &std::path::Path) -> Self {
        match std::fs::read_to_string(state_path) {
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Self::default(),
            Err(e) => {
                eprintln!("engram: bisync-state unreadable at {}, starting fresh: {e}", state_path.display());
                Self::default()
            }
            Ok(s) => match serde_json::from_str(&s) {
                Ok(state) => state,
                Err(e) => {
                    eprintln!("engram: bisync-state corrupt at {}, starting fresh (backup recommended): {e}", state_path.display());
                    Self::default()
                }
            },
        }
    }

    pub fn save(&self, state_path: &std::path::Path) -> std::io::Result<()> {
        // Ensure parent directory exists (first bisync on a fresh vault has no state dir yet).
        if let Some(parent) = state_path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let tmp = state_path.with_extension("tmp");
        let json = serde_json::to_string_pretty(self).map_err(|e| {
            std::io::Error::new(std::io::ErrorKind::InvalidData, e)
        })?;
        std::fs::write(&tmp, json)?;
        std::fs::rename(&tmp, state_path)
    }
}

/// What changed for a single file during bisync classification.
#[derive(Debug, Clone, PartialEq)]
pub struct FileChange {
    pub path: String,
    pub kind: ChangeKind,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ChangeKind {
    LocalOnly,
    RemoteOnly,
    Conflict { local_mtime: u64, remote_mtime: u64 },
    DeletedLocally,
    DeletedRemotely,
    NewLocal,
    NewRemote,
}

/// Classify all files across local manifest, remote listing, and baseline.
pub fn classify_changes(
    baseline: &SyncManifest,
    local: &SyncManifest,
    remote: &HashMap<String, RemoteFileEntry>,
) -> Vec<FileChange> {
    let mut changes = Vec::new();
    let all_paths: std::collections::HashSet<&String> = baseline.files.keys()
        .chain(local.files.keys())
        .chain(remote.keys())
        .collect();

    for path in all_paths {
        let in_local = local.files.get(path);
        let in_remote = remote.get(path);

        let local_changed = match (baseline.files.get(path), in_local) {
            (Some(b), Some(l)) => b.hash != l.hash,
            (None, Some(_)) => true,
            (Some(_), None) => true,
            (None, None) => false,
        };

        let remote_changed = match (baseline.files.get(path), in_remote) {
            (Some(b), Some(r)) => b.size != r.size || b.mtime_secs != r.mtime_secs,
            (None, Some(_)) => true,
            (Some(_), None) => true,
            (None, None) => false,
        };

        let in_baseline = baseline.files.contains_key(path);

        let kind = match (in_baseline, in_local, in_remote, local_changed, remote_changed) {
            (false, Some(_), None, _, _) => ChangeKind::NewLocal,
            (false, None, Some(_), _, _) => ChangeKind::NewRemote,
            (true, None, Some(_), _, _) => ChangeKind::DeletedLocally,
            (true, Some(_), None, _, _) => ChangeKind::DeletedRemotely,
            (_, Some(l), Some(r), true, true) => ChangeKind::Conflict {
                local_mtime: l.mtime_secs,
                remote_mtime: r.mtime_secs,
            },
            (_, _, _, true, false) => ChangeKind::LocalOnly,
            (_, _, _, false, true) => ChangeKind::RemoteOnly,
            // Both sides unchanged, or both sides deleted — nothing to do
            _ => continue,
        };

        changes.push(FileChange { path: path.clone(), kind });
    }

    changes
}

#[cfg(test)]
mod bisync_tests {
    use super::*;

    /// BiSyncState::save must succeed even when the parent directory does not exist yet.
    /// This is the case on the very first bisync for a fresh vault that has never synced.
    #[test]
    fn bisync_state_save_creates_missing_parent_dir() {
        let dir = tempfile::TempDir::new().expect("tmpdir");
        // Point to a path nested two levels deep that doesn't exist yet.
        let state_path = dir.path().join("fresh-vault").join("bisync-state.json");
        assert!(!state_path.parent().unwrap().exists(), "parent should not exist before save");
        let state = BiSyncState::default();
        state.save(&state_path).expect("save must succeed even with missing parent dir");
        assert!(state_path.exists(), "state file must be written");
    }

    #[test]
    fn bisync_state_roundtrips_json() {
        let mut state = BiSyncState::default();
        state.baseline.files.insert(
            "notes/test.md".to_string(),
            FileEntry {
                size: 100,
                mtime_secs: 1700000000,
                mtime_nanos: 0,
                hash: "abc123".to_string(),
            },
        );
        state.remote.insert(
            "notes/test.md".to_string(),
            RemoteFileEntry {
                size: 100,
                mtime_secs: 1700000000,
                etag: Some("etag-abc".to_string()),
            },
        );
        let json = serde_json::to_string(&state).unwrap();
        let restored: BiSyncState = serde_json::from_str(&json).unwrap();
        assert_eq!(restored.baseline.files.len(), 1);
        assert_eq!(restored.remote.len(), 1);
    }

    #[test]
    fn classify_local_only_change() {
        let mut baseline = SyncManifest::default();
        baseline.files.insert("a.md".to_string(), FileEntry {
            size: 10, mtime_secs: 100, mtime_nanos: 0, hash: "hash1".to_string()
        });
        let local = {
            let mut m = baseline.clone();
            m.files.get_mut("a.md").unwrap().mtime_secs = 200;
            m.files.get_mut("a.md").unwrap().hash = "hash2".to_string();
            m
        };
        let remote: std::collections::HashMap<String, RemoteFileEntry> = {
            let mut m = std::collections::HashMap::new();
            m.insert("a.md".to_string(), RemoteFileEntry {
                size: 10, mtime_secs: 100, etag: None
            });
            m
        };
        let changes = classify_changes(&baseline, &local, &remote);
        assert_eq!(changes.len(), 1);
        assert!(matches!(changes[0].kind, ChangeKind::LocalOnly));
    }

    #[test]
    fn classify_conflict() {
        let mut baseline = SyncManifest::default();
        baseline.files.insert("a.md".to_string(), FileEntry {
            size: 10, mtime_secs: 100, mtime_nanos: 0, hash: "hash1".to_string()
        });
        let local = {
            let mut m = baseline.clone();
            m.files.get_mut("a.md").unwrap().mtime_secs = 200;
            m.files.get_mut("a.md").unwrap().hash = "hash_local".to_string();
            m
        };
        let remote: std::collections::HashMap<String, RemoteFileEntry> = {
            let mut m = std::collections::HashMap::new();
            m.insert("a.md".to_string(), RemoteFileEntry {
                size: 15, mtime_secs: 150, etag: None
            });
            m
        };
        let changes = classify_changes(&baseline, &local, &remote);
        assert_eq!(changes.len(), 1);
        assert!(matches!(changes[0].kind, ChangeKind::Conflict { .. }));
    }
}
