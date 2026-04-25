// crates/engram-sync/src/azure.rs

use crate::backend::{SyncBackend, SyncError};
use async_trait::async_trait;
use bytes::Bytes;
use futures::StreamExt;
use object_store::{azure::MicrosoftAzureBuilder, path::Path, ObjectStore};
use std::sync::Arc;

pub struct AzureBackend {
    pub(crate) store: Arc<dyn ObjectStore>,
}

impl AzureBackend {
    pub fn new(account: &str, access_key: &str, container: &str) -> Result<Self, SyncError> {
        let store = MicrosoftAzureBuilder::new()
            .with_account(account)
            .with_access_key(access_key)
            .with_container_name(container)
            .build()
            .map_err(|e| SyncError::Backend(e.to_string()))?;
        Ok(Self {
            store: Arc::new(store),
        })
    }
}

#[async_trait]
impl SyncBackend for AzureBackend {
    async fn push(&self, path: &str, data: Bytes) -> Result<(), SyncError> {
        self.store
            .put(&Path::from(path), data.into())
            .await
            .map_err(|e| SyncError::Backend(e.to_string()))?;
        Ok(())
    }

    async fn pull(&self, path: &str) -> Result<Bytes, SyncError> {
        let result = self
            .store
            .get(&Path::from(path))
            .await
            .map_err(|e| match e {
                object_store::Error::NotFound { .. } => SyncError::NotFound(path.to_string()),
                _ => SyncError::Backend(e.to_string()),
            })?;
        result
            .bytes()
            .await
            .map_err(|e| SyncError::Backend(e.to_string()))
    }

    async fn list(&self, prefix: &str) -> Result<Vec<String>, SyncError> {
        let results: Vec<_> = self.store.list(Some(&Path::from(prefix))).collect().await;
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
        "azure"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bytes::Bytes;
    use object_store::local::LocalFileSystem;
    use std::sync::Arc;
    use tempfile::TempDir;

    fn local_backend(dir: &TempDir) -> AzureBackend {
        let store = LocalFileSystem::new_with_prefix(dir.path()).unwrap();
        AzureBackend {
            store: Arc::new(store),
        }
    }

    #[tokio::test]
    async fn test_push_and_pull_bytes() {
        let dir = TempDir::new().unwrap();
        let backend = local_backend(&dir);
        let data = Bytes::from(b"azure-payload".to_vec());
        backend.push("vault/test.md", data.clone()).await.unwrap();
        let retrieved = backend.pull("vault/test.md").await.unwrap();
        assert_eq!(retrieved, data);
    }

    #[tokio::test]
    async fn test_list_returns_pushed_paths() {
        let dir = TempDir::new().unwrap();
        let backend = local_backend(&dir);
        backend
            .push("vault/Work/notes.md", Bytes::from("a"))
            .await
            .unwrap();
        backend
            .push("vault/Work/tasks.md", Bytes::from("b"))
            .await
            .unwrap();
        let paths = backend.list("vault/Work").await.unwrap();
        assert_eq!(paths.len(), 2);
    }

    #[tokio::test]
    async fn test_delete_removes_object() {
        let dir = TempDir::new().unwrap();
        let backend = local_backend(&dir);
        backend
            .push("vault/temp.md", Bytes::from("x"))
            .await
            .unwrap();
        backend.delete("vault/temp.md").await.unwrap();
        assert!(backend.pull("vault/temp.md").await.is_err());
    }

    #[test]
    fn test_backend_name() {
        let dir = TempDir::new().unwrap();
        let backend = local_backend(&dir);
        assert_eq!(backend.backend_name(), "azure");
    }
}
