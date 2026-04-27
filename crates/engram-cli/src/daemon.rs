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
pub fn is_vault_md_event_path(path: &Path) -> bool {
    // Must have .md extension
    if path.extension().and_then(|e| e.to_str()) != Some("md") {
        return false;
    }
    // Reject hidden files and files inside hidden directories
    for component in path.components() {
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
                if !is_vault_md_event_path(&path) {
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
        assert!(is_vault_md_event_path(Path::new("/vault/notes.md")));
        assert!(is_vault_md_event_path(Path::new("/vault/subdir/entry.md")));
        assert!(!is_vault_md_event_path(Path::new("/vault/notes.txt")));
        assert!(!is_vault_md_event_path(Path::new("/vault/.DS_Store")));
        assert!(!is_vault_md_event_path(Path::new("/vault/transcript.jsonl")));
    }

    #[test]
    fn vault_event_ignores_hidden_files() {
        assert!(!is_vault_md_event_path(Path::new("/vault/.hidden.md")));
        assert!(!is_vault_md_event_path(Path::new("/vault/.git/COMMIT_EDITMSG")));
    }

    #[test]
    fn conflict_copies_excluded_from_watch() {
        assert!(!is_vault_md_event_path(Path::new(
            "/vault/note.conflict-2026-04-26-120000.md"
        )));
    }
}
