// daemon.rs — file watcher

use notify::{Config, Event, EventKind, RecommendedWatcher, RecursiveMode, Watcher};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::mpsc;
use std::time::{Duration, Instant};
use thiserror::Error;

/// Errors that can occur during daemon / file-watching operations.
#[derive(Debug, Error)]
pub enum DaemonError {
    #[error("notify error: {0}")]
    Notify(#[from] notify::Error),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
}

/// Duration used to debounce repeated events for the same path.
const DEBOUNCE_DURATION: Duration = Duration::from_secs(5);

/// Returns `true` if `event` is a `Modify` or `Create` event and at least one
/// of its paths has the file name `transcript.jsonl`.
pub fn is_transcript_event(event: &Event) -> bool {
    let is_relevant_kind = matches!(event.kind, EventKind::Modify(_) | EventKind::Create(_));
    if !is_relevant_kind {
        return false;
    }
    event.paths.iter().any(|p| {
        p.file_name()
            .map(|name| name == "transcript.jsonl")
            .unwrap_or(false)
    })
}

/// Watch `watch_dir` recursively. When a `transcript.jsonl` file is modified or
/// created, sends its [`PathBuf`] through `tx` (debounced per-path to at most
/// one send per [`DEBOUNCE_DURATION`]).
///
/// The returned [`RecommendedWatcher`] must be kept alive by the caller; dropping
/// it stops the underlying OS watcher and terminates the background thread.
pub fn watch_sessions(
    watch_dir: &Path,
    tx: mpsc::Sender<PathBuf>,
) -> Result<RecommendedWatcher, DaemonError> {
    let (notify_tx, notify_rx) = mpsc::channel::<notify::Result<Event>>();

    let mut watcher = RecommendedWatcher::new(notify_tx, Config::default())?;
    watcher.watch(watch_dir, RecursiveMode::Recursive)?;

    std::thread::spawn(move || {
        let mut last_seen: HashMap<PathBuf, Instant> = HashMap::new();

        // Re-check per-path: is_transcript_event guarantees at least one transcript path,
        // but a single notify event can carry multiple paths — only send transcript paths.
        for event in notify_rx.into_iter().flatten() {
            if is_transcript_event(&event) {
                let now = Instant::now();
                for path in &event.paths {
                    if path
                        .file_name()
                        .map(|n| n == "transcript.jsonl")
                        .unwrap_or(false)
                    {
                        let should_send = last_seen
                            .get(path)
                            .map(|last| now.duration_since(*last) >= DEBOUNCE_DURATION)
                            .unwrap_or(true);

                        if should_send {
                            last_seen.insert(path.clone(), now);
                            let _ = tx.send(path.clone());
                        }
                    }
                }
            }
        }
    });

    Ok(watcher)
}

#[cfg(test)]
mod tests {
    use super::*;
    use notify::event::{AccessKind, CreateKind, ModifyKind};

    /// Helper: build a notify `Event` with the given `kind` and `paths`.
    fn make_event(kind: EventKind, paths: Vec<PathBuf>) -> Event {
        paths
            .into_iter()
            .fold(Event::new(kind), |e, p| e.add_path(p))
    }

    #[test]
    fn test_is_transcript_event_true_for_modify_transcript() {
        let event = make_event(
            EventKind::Modify(ModifyKind::Any),
            vec![PathBuf::from("/sessions/abc/transcript.jsonl")],
        );
        assert!(
            is_transcript_event(&event),
            "Modify event on transcript.jsonl should return true"
        );
    }

    #[test]
    fn test_is_transcript_event_true_for_create_transcript() {
        let event = make_event(
            EventKind::Create(CreateKind::File),
            vec![PathBuf::from("/sessions/abc/transcript.jsonl")],
        );
        assert!(
            is_transcript_event(&event),
            "Create event on transcript.jsonl should return true"
        );
    }

    #[test]
    fn test_is_transcript_event_false_for_non_transcript_file() {
        let event = make_event(
            EventKind::Modify(ModifyKind::Any),
            vec![PathBuf::from("/sessions/abc/events.jsonl")],
        );
        assert!(
            !is_transcript_event(&event),
            "Modify event on a non-transcript file should return false"
        );
    }

    #[test]
    fn test_is_transcript_event_false_for_access_event() {
        let event = make_event(
            EventKind::Access(AccessKind::Any),
            vec![PathBuf::from("/sessions/abc/transcript.jsonl")],
        );
        assert!(
            !is_transcript_event(&event),
            "Access event (not Modify/Create) on transcript.jsonl should return false"
        );
    }

    #[test]
    fn test_is_transcript_event_false_for_empty_paths() {
        let event = make_event(EventKind::Modify(ModifyKind::Any), vec![]);
        assert!(
            !is_transcript_event(&event),
            "Modify event with no paths should return false"
        );
    }
}
