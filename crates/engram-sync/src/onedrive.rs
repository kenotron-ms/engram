// crates/engram-sync/src/onedrive.rs

use async_trait::async_trait;
use bytes::Bytes;
use reqwest::{Client, StatusCode};
use crate::backend::{SyncBackend, SyncError};

pub struct OneDriveBackend {
    client: Client,
    access_token: String,
    folder: String, // e.g. "/Apps/Engram/vault"
}

impl OneDriveBackend {
    pub fn new(access_token: &str, folder: &str) -> Self {
        Self {
            client: Client::new(),
            access_token: access_token.to_string(),
            folder: folder.to_string(),
        }
    }

    /// Build the Graph API URL for file content operations.
    pub(crate) fn item_url(&self, path: &str) -> String {
        let full_path = format!(
            "{}/{}",
            self.folder.trim_end_matches('/'),
            path
        );
        format!(
            "https://graph.microsoft.com/v1.0/me/drive/root:{}:/content",
            full_path
        )
    }

    fn auth_header(&self) -> String {
        format!("Bearer {}", self.access_token)
    }
}

#[async_trait]
impl SyncBackend for OneDriveBackend {
    async fn push(&self, path: &str, data: Bytes) -> Result<(), SyncError> {
        let url = self.item_url(path);
        let response = self.client
            .put(&url)
            .header("Authorization", self.auth_header())
            .header("Content-Type", "application/octet-stream")
            .body(data)
            .send()
            .await?;
        if !response.status().is_success() {
            return Err(SyncError::Backend(format!(
                "OneDrive push failed: {}",
                response.status()
            )));
        }
        Ok(())
    }

    async fn pull(&self, path: &str) -> Result<Bytes, SyncError> {
        let url = self.item_url(path);
        let response = self.client
            .get(&url)
            .header("Authorization", self.auth_header())
            .send()
            .await?;
        if response.status() == StatusCode::NOT_FOUND {
            return Err(SyncError::NotFound(path.to_string()));
        }
        if !response.status().is_success() {
            return Err(SyncError::Backend(format!(
                "OneDrive pull failed: {}",
                response.status()
            )));
        }
        Ok(response.bytes().await?)
    }

    async fn list(&self, prefix: &str) -> Result<Vec<String>, SyncError> {
        let folder_path = format!(
            "{}/{}",
            self.folder.trim_end_matches('/'),
            prefix
        );
        let url = format!(
            "https://graph.microsoft.com/v1.0/me/drive/root:{}:/children",
            folder_path
        );
        let response = self.client
            .get(&url)
            .header("Authorization", self.auth_header())
            .send()
            .await?;
        if !response.status().is_success() {
            return Err(SyncError::Backend(format!(
                "OneDrive list failed: {}",
                response.status()
            )));
        }
        let json: serde_json::Value = response
            .json()
            .await
            .map_err(|e| SyncError::Backend(e.to_string()))?;
        let names = json["value"]
            .as_array()
            .unwrap_or(&vec![])
            .iter()
            .filter_map(|item| {
                item["name"].as_str().map(|s| format!("{}/{}", prefix, s))
            })
            .collect();
        Ok(names)
    }

    async fn delete(&self, path: &str) -> Result<(), SyncError> {
        // Use the items endpoint for delete (the content endpoint doesn't support DELETE)
        let folder_path = format!(
            "{}/{}",
            self.folder.trim_end_matches('/'),
            path
        );
        let url = format!(
            "https://graph.microsoft.com/v1.0/me/drive/root:{}",
            folder_path
        );
        let response = self.client
            .delete(&url)
            .header("Authorization", self.auth_header())
            .send()
            .await?;
        if !response.status().is_success() && response.status() != StatusCode::NOT_FOUND {
            return Err(SyncError::Backend(format!(
                "OneDrive delete failed: {}",
                response.status()
            )));
        }
        Ok(())
    }

    fn backend_name(&self) -> &'static str {
        "onedrive"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_item_url_construction() {
        let backend = OneDriveBackend::new("token", "/Apps/Engram/vault");
        let url = backend.item_url("People/Sofia.md");
        assert_eq!(
            url,
            "https://graph.microsoft.com/v1.0/me/drive/root:/Apps/Engram/vault/People/Sofia.md:/content"
        );
    }

    #[test]
    fn test_item_url_no_double_slash_when_folder_has_trailing_slash() {
        let backend = OneDriveBackend::new("token", "/Apps/Engram/vault/");
        let url = backend.item_url("Tasks.md");
        assert_eq!(
            url,
            "https://graph.microsoft.com/v1.0/me/drive/root:/Apps/Engram/vault/Tasks.md:/content"
        );
    }

    #[test]
    fn test_backend_name() {
        let backend = OneDriveBackend::new("token", "/Apps/Engram/vault");
        assert_eq!(backend.backend_name(), "onedrive");
    }

    #[tokio::test]
    #[ignore = "requires real OneDrive access token — set ONEDRIVE_TOKEN env var"]
    async fn test_push_pull_real() {
        use bytes::Bytes;
        let token = std::env::var("ONEDRIVE_TOKEN").expect("ONEDRIVE_TOKEN not set");
        let backend = OneDriveBackend::new(&token, "/Apps/Engram/test");
        backend.push("test.md", Bytes::from("hello")).await.unwrap();
        let pulled = backend.pull("test.md").await.unwrap();
        assert_eq!(pulled.as_ref(), b"hello");
        backend.delete("test.md").await.unwrap();
    }
}
