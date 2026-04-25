// crates/engram-sync/src/s3.rs

use async_trait::async_trait;
use bytes::Bytes;
use futures::StreamExt;
use object_store::{aws::AmazonS3Builder, path::Path, ObjectStore};
use std::sync::Arc;
use crate::backend::{SyncBackend, SyncError};

pub struct S3Backend {
    pub(crate) store: Arc<dyn ObjectStore>,
    pub(crate) bucket: String,
}

impl S3Backend {
    /// Create an S3Backend connected to a real S3-compatible endpoint.
    pub fn new(
        endpoint: &str,
        bucket: &str,
        access_key: &str,
        secret_key: &str,
    ) -> Result<Self, SyncError> {
        let store = AmazonS3Builder::new()
            .with_endpoint(endpoint)
            .with_bucket_name(bucket)
            .with_access_key_id(access_key)
            .with_secret_access_key(secret_key)
            .with_allow_http(true)
            .build()
            .map_err(|e| SyncError::Backend(e.to_string()))?;
        Ok(Self {
            store: Arc::new(store),
            bucket: bucket.to_string(),
        })
    }

    /// Create an S3Backend from an existing ObjectStore. Used in tests and
    /// the integration test harness where LocalFileSystem stands in for S3.
    pub fn from_store(store: Arc<dyn ObjectStore>, bucket: String) -> Self {
        Self { store, bucket }
    }
}

#[async_trait]
impl SyncBackend for S3Backend {
    async fn push(&self, path: &str, data: Bytes) -> Result<(), SyncError> {
        self.store
            .put(&Path::from(path), data.into())
            .await
            .map_err(|e| SyncError::Backend(e.to_string()))?;
        Ok(())
    }

    async fn pull(&self, path: &str) -> Result<Bytes, SyncError> {
        let result = self.store
            .get(&Path::from(path))
            .await
            .map_err(|e| {
                let msg = e.to_string();
                if msg.contains("404") || msg.contains("not found") || msg.contains("No such file") {
                    SyncError::NotFound(path.to_string())
                } else {
                    SyncError::Backend(msg)
                }
            })?;
        let bytes = result
            .bytes()
            .await
            .map_err(|e| SyncError::Backend(e.to_string()))?;
        Ok(bytes)
    }

    async fn list(&self, prefix: &str) -> Result<Vec<String>, SyncError> {
        let prefix_path = Path::from(prefix);
        let results: Vec<_> = self.store
            .list(Some(&prefix_path))
            .collect()
            .await;
        results
            .into_iter()
            .map(|r| {
                r.map(|meta| meta.location.to_string())
                    .map_err(|e| SyncError::Backend(e.to_string()))
            })
            .collect()
    }

    async fn delete(&self, path: &str) -> Result<(), SyncError> {
        self.store
            .delete(&Path::from(path))
            .await
            .map_err(|e| SyncError::Backend(e.to_string()))?;
        Ok(())
    }

    fn backend_name(&self) -> &'static str {
        "s3"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bytes::Bytes;
    use object_store::local::LocalFileSystem;
    use std::sync::Arc;
    use tempfile::TempDir;

    fn local_backend(dir: &TempDir) -> S3Backend {
        let store = LocalFileSystem::new_with_prefix(dir.path()).unwrap();
        S3Backend::from_store(Arc::new(store), "test-bucket".to_string())
    }

    #[tokio::test]
    async fn test_push_and_pull_bytes() {
        let dir = TempDir::new().unwrap();
        let backend = local_backend(&dir);
        let data = Bytes::from(b"encrypted-payload".to_vec());
        backend.push("vault/test.md", data.clone()).await.unwrap();
        let retrieved = backend.pull("vault/test.md").await.unwrap();
        assert_eq!(retrieved, data);
    }

    #[tokio::test]
    async fn test_list_returns_pushed_paths() {
        let dir = TempDir::new().unwrap();
        let backend = local_backend(&dir);
        backend.push("vault/People/Sofia.md", Bytes::from("a")).await.unwrap();
        backend.push("vault/People/Chris.md", Bytes::from("b")).await.unwrap();
        let paths = backend.list("vault/People").await.unwrap();
        assert_eq!(paths.len(), 2);
    }

    #[tokio::test]
    async fn test_delete_removes_object() {
        let dir = TempDir::new().unwrap();
        let backend = local_backend(&dir);
        backend.push("vault/temp.md", Bytes::from("x")).await.unwrap();
        backend.delete("vault/temp.md").await.unwrap();
        assert!(backend.pull("vault/temp.md").await.is_err());
    }

    #[test]
    fn test_backend_name() {
        let dir = TempDir::new().unwrap();
        let backend = local_backend(&dir);
        assert_eq!(backend.backend_name(), "s3");
    }
}
