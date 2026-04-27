//! Bidirectional sync algorithm for engram vaults.
//!
//! Conflict resolution strategy: newer-wins, loser saved as a .conflict copy.
//! Deletion propagation: files deleted on one side are deleted on the other.

use crate::{
    encrypt::{decrypt_from_sync, encrypt_for_sync},
    manifest::{classify_changes, BiSyncState, ChangeKind, FileEntry, SyncManifest},
    SyncBackend, SyncError,
};
use engram_core::crypto::EngramKey;
use std::path::Path;

/// Generate a conflict copy filename.
///
/// "notes/entry.md" + 1714176000 → "notes/entry.conflict-2024-04-27-000000.md"
pub fn conflict_copy_name(relative_path: &str, mtime_secs: u64) -> String {
    let (year, month, day, hour, min, sec) = seconds_to_ymd_hms(mtime_secs);
    let timestamp = format!("{year:04}-{month:02}-{day:02}-{hour:02}{min:02}{sec:02}");

    let p = Path::new(relative_path);
    let stem = p.file_stem().and_then(|s| s.to_str()).unwrap_or("file");
    let parent = p.parent().and_then(|p| p.to_str()).unwrap_or("");
    let new_name = format!("{stem}.conflict-{timestamp}.md");
    if parent.is_empty() {
        new_name
    } else {
        format!("{parent}/{new_name}")
    }
}

/// Minimal UTC seconds → (year, month, day, hour, min, sec).
///
/// Uses Howard Hinnant's civil-calendar algorithm to avoid a `chrono` dependency.
fn seconds_to_ymd_hms(secs: u64) -> (u32, u32, u32, u32, u32, u32) {
    let sec = (secs % 60) as u32;
    let min = ((secs / 60) % 60) as u32;
    let hour = ((secs / 3600) % 24) as u32;
    let days = secs / 86400;
    let z = days as i64 + 719468;
    let era = if z >= 0 { z } else { z - 146096 } / 146097;
    let doe = (z - era * 146097) as u64;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let y = yoe as i64 + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = if m <= 2 { y + 1 } else { y };
    (y as u32, m as u32, d as u32, hour, min, sec)
}

/// Scan a vault directory and build a `SyncManifest` reflecting the current
/// local state (all `.md` files with their sizes, mtimes, and content hashes).
fn scan_vault_manifest(vault_path: &Path) -> Result<SyncManifest, SyncError> {
    let mut manifest = SyncManifest::default();
    scan_dir(vault_path, vault_path, &mut manifest)?;
    Ok(manifest)
}

fn scan_dir(
    vault_root: &Path,
    dir: &Path,
    manifest: &mut SyncManifest,
) -> Result<(), SyncError> {
    let entries = std::fs::read_dir(dir)
        .map_err(|e| SyncError::Io(format!("read_dir {}: {e}", dir.display())))?;

    for entry in entries {
        let entry =
            entry.map_err(|e| SyncError::Io(format!("dir entry in {}: {e}", dir.display())))?;
        let path = entry.path();

        if path.is_dir() {
            scan_dir(vault_root, &path, manifest)?;
        } else if path.extension().and_then(|e| e.to_str()) == Some("md") {
            let relative = path
                .strip_prefix(vault_root)
                .map_err(|e| SyncError::Io(format!("strip_prefix: {e}")))?
                .to_string_lossy()
                .into_owned();

            let meta = std::fs::metadata(&path)
                .map_err(|e| SyncError::Io(format!("metadata {}: {e}", path.display())))?;
            let size = meta.len();
            let (mtime_secs, mtime_nanos) = meta
                .modified()
                .ok()
                .map(SyncManifest::mtime_components)
                .unwrap_or((0, 0));

            let content = std::fs::read(&path)
                .map_err(|e| SyncError::Io(format!("read {}: {e}", path.display())))?;
            let hash = SyncManifest::content_hash(&content);

            manifest.files.insert(
                relative,
                FileEntry {
                    size,
                    mtime_secs,
                    mtime_nanos,
                    hash,
                },
            );
        }
    }
    Ok(())
}

/// Summary of a completed bisync run.
#[derive(Debug, Default)]
pub struct BiSyncResult {
    pub uploaded: usize,
    pub downloaded: usize,
    pub conflicts_resolved: usize,
    pub deleted_local: usize,
    pub deleted_remote: usize,
    pub errors: Vec<String>,
}

/// Run bisync for a single vault.
///
/// 1. Scans local vault for the current `SyncManifest`.
/// 2. Lists remote files to build a remote map.
/// 3. Loads the saved `BiSyncState` (common ancestor baseline).
/// 4. Classifies changes across local/remote/baseline.
/// 5. Executes each change (upload, download, conflict resolution, deletion).
/// 6. Saves updated `BiSyncState` for the next run.
pub async fn run_bisync<B: SyncBackend>(
    vault_path: &Path,
    state_path: &Path,
    key: &EngramKey,
    backend: &B,
) -> Result<BiSyncResult, SyncError> {
    let mut result = BiSyncResult::default();

    // 1. Build current local manifest by scanning the vault directory.
    let local = scan_vault_manifest(vault_path)?;

    // 2. List remote files and build a remote map.
    //    Object-store list() returns only paths; we set size/mtime to 0 because
    //    most backends don't expose per-file metadata in the listing API.
    //    The conflict classifier falls back to treating unknown remote metadata
    //    as "changed" — so local always wins when metadata is unavailable.
    let remote_list = backend.list("").await?;
    let mut remote_map = std::collections::HashMap::new();
    for remote_path in &remote_list {
        remote_map.insert(
            remote_path.clone(),
            crate::manifest::RemoteFileEntry {
                size: 0,
                mtime_secs: 0,
                etag: None,
            },
        );
    }

    // 3. Load baseline bisync state (empty on first run).
    let state = BiSyncState::load(state_path);

    // 4. Classify changes against the baseline.
    let changes = classify_changes(&state.baseline, &local, &remote_map);

    // 5. Execute each change.
    for change in &changes {
        let local_file = vault_path.join(&change.path);

        match &change.kind {
            ChangeKind::LocalOnly | ChangeKind::NewLocal => {
                let content = std::fs::read(&local_file).map_err(|e| {
                    SyncError::Io(format!("read {}: {e}", change.path))
                })?;
                let encrypted = encrypt_for_sync(key, &content)?;
                backend.push(&change.path, encrypted).await?;
                result.uploaded += 1;
            }

            ChangeKind::RemoteOnly | ChangeKind::NewRemote => {
                let encrypted = backend.pull(&change.path).await?;
                let content = decrypt_from_sync(key, &encrypted)?;
                if let Some(parent) = local_file.parent() {
                    std::fs::create_dir_all(parent).ok();
                }
                std::fs::write(&local_file, &content).map_err(|e| {
                    SyncError::Io(format!("write {}: {e}", change.path))
                })?;
                result.downloaded += 1;
            }

            ChangeKind::Conflict {
                local_mtime,
                remote_mtime,
            } => {
                if local_mtime >= remote_mtime {
                    // Local is newer (or tied): save remote as a conflict copy, upload local.
                    let conflict_path = conflict_copy_name(&change.path, *remote_mtime);
                    let remote_encrypted = backend.pull(&change.path).await?;
                    let remote_content = decrypt_from_sync(key, &remote_encrypted)?;
                    let conflict_local = vault_path.join(&conflict_path);
                    if let Some(parent) = conflict_local.parent() {
                        std::fs::create_dir_all(parent).ok();
                    }
                    std::fs::write(&conflict_local, &remote_content).ok();

                    let content = std::fs::read(&local_file).map_err(|e| {
                        SyncError::Io(format!("read {}: {e}", change.path))
                    })?;
                    let encrypted = encrypt_for_sync(key, &content)?;
                    backend.push(&change.path, encrypted).await?;
                } else {
                    // Remote is newer: save local as a conflict copy, download remote.
                    let conflict_path = conflict_copy_name(&change.path, *local_mtime);
                    let conflict_local = vault_path.join(&conflict_path);
                    if let Some(parent) = conflict_local.parent() {
                        std::fs::create_dir_all(parent).ok();
                    }
                    std::fs::copy(&local_file, &conflict_local).ok();

                    let encrypted = backend.pull(&change.path).await?;
                    let content = decrypt_from_sync(key, &encrypted)?;
                    std::fs::write(&local_file, &content).map_err(|e| {
                        SyncError::Io(format!("write {}: {e}", change.path))
                    })?;
                }
                result.conflicts_resolved += 1;
                eprintln!("  conflict resolved: {} (newer-wins)", change.path);
            }

            ChangeKind::DeletedLocally => {
                backend.delete(&change.path).await?;
                result.deleted_remote += 1;
            }

            ChangeKind::DeletedRemotely => {
                let _ = std::fs::remove_file(&local_file);
                result.deleted_local += 1;
            }
        }
    }

    // 6. Persist updated bisync state for the next run.
    let new_state = BiSyncState {
        baseline: local,
        remote: remote_map,
    };
    new_state
        .save(state_path)
        .map_err(|e| SyncError::Io(e.to_string()))?;

    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn conflict_filename_format() {
        let name = conflict_copy_name("notes/entry.md", 1714176000);
        // 1714176000 = 2024-04-27 00:00:00 UTC
        assert_eq!(name, "notes/entry.conflict-2024-04-27-000000.md");
    }

    #[test]
    fn conflict_filename_preserves_subdir() {
        let name = conflict_copy_name("Personal/Notes/2026-04-26 -- Something.md", 1714176000);
        assert!(name.starts_with("Personal/Notes/"));
        assert!(name.contains(".conflict-"));
        assert!(name.ends_with(".md"));
    }
}
