// crates/engram-sync/src/backend.rs

use async_trait::async_trait;
use bytes::Bytes;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum SyncError {
    #[error("Backend error: {0}")]
    Backend(String),
    #[error("Auth error: {0}")]
    Auth(String),
    #[error("Not found: {0}")]
    NotFound(String),
    #[error("Encryption error: {0}")]
    Encryption(String),
    #[error("Network error: {0}")]
    Network(#[from] reqwest::Error),
}

#[async_trait]
pub trait SyncBackend: Send + Sync {
    async fn push(&self, path: &str, data: Bytes) -> Result<(), SyncError>;
    async fn pull(&self, path: &str) -> Result<Bytes, SyncError>;
    async fn list(&self, prefix: &str) -> Result<Vec<String>, SyncError>;
    async fn delete(&self, path: &str) -> Result<(), SyncError>;
    fn backend_name(&self) -> &'static str;
}
