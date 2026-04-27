//! Vault file watcher for the engram daemon.
//!
//! Watches configured vault paths for *.md file changes and emits
//! VaultEvent on a channel. The daemon loop uses these events to
//! trigger incremental reindexing and auto-sync.

use notify::{Event, EventKind, RecommendedWatcher, RecursiveMode, Watcher};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::mpsc;
use std::time::{Duration, Instant};

const DEBOUNCE_DURATION: Duration = Duration::from_secs(3);

#[derive(Debug, Clone)]
pub struct VaultEvent {
    /// Name of the vault (key in EngramConfig::vaults)
    pub vault_name: String,
    /// Absolute path to the changed .md file
    pub path: PathBuf,
    /// Whether this was a deletion
    pub deleted: bool,
}

#[derive(Debug, thiserror::Error)]
pub enum DaemonError {
    #[error("file watcher error: {0}")]
    Watch(#[from] notify::Error),
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
}

/// Returns true if the path is a vault .md file that should trigger
/// reindexing/sync. Excludes: hidden files, .conflict copies, non-.md files.
///
/// `vault_root` is used to strip the vault prefix before checking for hidden
/// components, so a vault stored inside a hidden directory (e.g. `~/.vaults/main`)
/// is not incorrectly rejected.
pub fn is_vault_md_event_path(vault_root: &Path, path: &Path) -> bool {
    // Must have .md extension
    if path.extension().and_then(|e| e.to_str()) != Some("md") {
        return false;
    }
    // Only check components RELATIVE to vault root, not absolute path components.
    // This allows vault roots that live inside hidden directories (e.g. ~/.vaults/).
    let relative = path.strip_prefix(vault_root).unwrap_or(path);
    for component in relative.components() {
        if let std::path::Component::Normal(name) = component {
            let s = name.to_string_lossy();
            if s.starts_with('.') {
                return false;
            }
        }
    }
    // Reject .conflict copies: filename stem contains ".conflict-"
    if let Some(stem) = path.file_stem().and_then(|s| s.to_str()) {
        if stem.contains(".conflict-") {
            return false;
        }
    }
    true
}

/// Start a file watcher on `vault_path`, emitting `VaultEvent` on `tx`
/// for every debounced *.md Create/Modify/Remove event.
///
/// Returns the watcher (caller must keep it alive for events to fire).
pub fn watch_vault(
    vault_name: String,
    vault_path: &Path,
    tx: mpsc::Sender<VaultEvent>,
) -> Result<RecommendedWatcher, DaemonError> {
    let (event_tx, event_rx) = mpsc::channel::<notify::Result<Event>>();

    let mut watcher = RecommendedWatcher::new(
        move |res| {
            let _ = event_tx.send(res);
        },
        notify::Config::default(),
    )?;

    watcher.watch(vault_path, RecursiveMode::Recursive)?;

    let vault_root = vault_path.to_path_buf();
    std::thread::spawn(move || {
        let mut last_seen: HashMap<PathBuf, Instant> = HashMap::new();

        for result in event_rx {
            let event = match result {
                Ok(e) => e,
                Err(_) => continue,
            };

            let deleted = matches!(event.kind, EventKind::Remove(_));
            let relevant = matches!(
                event.kind,
                EventKind::Create(_) | EventKind::Modify(_) | EventKind::Remove(_)
            );
            if !relevant {
                continue;
            }

            for path in event.paths {
                if !is_vault_md_event_path(&vault_root, &path) {
                    continue;
                }
                let now = Instant::now();
                let last = last_seen
                    .get(&path)
                    .copied()
                    .unwrap_or(now - DEBOUNCE_DURATION * 2);
                if now.duration_since(last) < DEBOUNCE_DURATION {
                    continue;
                }
                last_seen.insert(path.clone(), now);
                let _ = tx.send(VaultEvent {
                    vault_name: vault_name.clone(),
                    path,
                    deleted,
                });
            }
        }
    });

    Ok(watcher)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn vault_event_identifies_md_files() {
        let root = Path::new("/vault");
        assert!(is_vault_md_event_path(root, Path::new("/vault/notes.md")));
        assert!(is_vault_md_event_path(root, Path::new("/vault/subdir/entry.md")));
        assert!(!is_vault_md_event_path(root, Path::new("/vault/notes.txt")));
        assert!(!is_vault_md_event_path(root, Path::new("/vault/.DS_Store")));
        assert!(!is_vault_md_event_path(root, Path::new("/vault/transcript.jsonl")));
    }

    #[test]
    fn vault_event_ignores_hidden_files() {
        let root = Path::new("/vault");
        assert!(!is_vault_md_event_path(root, Path::new("/vault/.hidden.md")));
        assert!(!is_vault_md_event_path(root, Path::new("/vault/.git/COMMIT_EDITMSG")));
    }

    #[test]
    fn conflict_copies_excluded_from_watch() {
        let root = Path::new("/vault");
        assert!(!is_vault_md_event_path(root, Path::new(
            "/vault/note.conflict-2026-04-26-120000.md"
        )));
    }

    #[test]
    fn vault_in_hidden_dir_still_watched() {
        // Vault root is in a hidden directory — files inside should still be indexed.
        // This test exposes the bug: the current implementation rejects any path
        // that has a dot-prefixed component anywhere, including the vault root itself.
        let root = Path::new("/home/user/.vaults/main");
        assert!(is_vault_md_event_path(root, Path::new("/home/user/.vaults/main/notes.md")));
        assert!(is_vault_md_event_path(root, Path::new("/home/user/.vaults/main/subdir/entry.md")));
        // But hidden files WITHIN the vault are still excluded
        assert!(!is_vault_md_event_path(root, Path::new("/home/user/.vaults/main/.hidden.md")));
    }
}
