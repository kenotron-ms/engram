// crates/engram-sync/src/lib.rs

    pub mod auth;
    pub mod backend;
    pub mod encrypt;
    pub mod s3;
    pub mod azure;
    pub mod gcs;
    pub mod onedrive;

    pub use backend::{SyncBackend, SyncError};
    pub use bytes::Bytes;
    