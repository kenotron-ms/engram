// crates/engram-sync/src/lib.rs

pub mod auth;
pub mod azure;
pub mod backend;
pub mod encrypt;
pub mod gcs;
pub mod manifest;
pub mod onedrive;
pub mod s3;

pub use backend::{SyncBackend, SyncError};
pub use bytes::Bytes;
pub use manifest::{BiSyncState, RemoteFileEntry, FileChange, ChangeKind, classify_changes};
