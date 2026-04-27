// crates/engram-sync/src/onedrive.rs

use crate::backend::{SyncBackend, SyncError};
use async_trait::async_trait;
use bytes::Bytes;
use reqwest::{Client, StatusCode};

const GRAPH_DRIVE_ROOT: &str = "https://graph.microsoft.com/v1.0/me/drive/root:";

const MICROSOFT_TOKEN_URL: &str =
    "https://login.microsoftonline.com/common/oauth2/v2.0/token";

// The engram OneDrive app client_id — public client, no secret needed for token refresh.
// TODO: replace with actual app client_id registered for engram
const ONEDRIVE_CLIENT_ID: &str = "07393f4b-b8c7-4e52-b5e4-1fe2df8c6b4c";

/// Internal token state, shared behind a Mutex for interior mutability.
/// Using std::sync::Mutex (not tokio) is intentional: we never hold the guard
/// across an await point, so a lightweight sync mutex is correct here.
struct Tokens {
    access_token: String,
    refresh_token: Option<String>,
}

pub struct OneDriveBackend {
    client: Client,
    tokens: std::sync::Mutex<Tokens>,
    folder: String, // e.g. "/Apps/Engram/vault"
}

/// Build the URL-encoded body for an OAuth 2.0 refresh-token grant.
///
/// Microsoft refresh tokens are opaque base64url strings (A-Za-z0-9-_.) —
/// all characters are safe in application/x-www-form-urlencoded without
/// additional percent-encoding.
fn build_token_refresh_body(refresh_token: &str) -> String {
    format!(
        "client_id={}&refresh_token={}&grant_type=refresh_token\
         &scope=Files.ReadWrite.All+offline_access",
        ONEDRIVE_CLIENT_ID, refresh_token
    )
}

impl OneDriveBackend {
    /// Create a backend with only an access token (no auto-refresh on 401).
    pub fn new(access_token: &str, folder: &str) -> Self {
        Self::with_refresh_token(access_token, None, folder)
    }

    /// Create a backend with both an access token and a refresh token.
    /// Enables automatic token refresh on 401 responses.
    pub fn with_refresh_token(
        access_token: &str,
        refresh_token: Option<&str>,
        folder: &str,
    ) -> Self {
        Self {
            client: Client::new(),
            tokens: std::sync::Mutex::new(Tokens {
                access_token: access_token.to_string(),
                refresh_token: refresh_token.map(|t| t.to_string()),
            }),
            folder: folder.to_string(),
        }
    }

    /// Combine the configured folder with a relative path, ensuring no double slashes.
    fn full_path(&self, path: &str) -> String {
        format!("{}/{}", self.folder.trim_end_matches('/'), path)
    }

    /// Build the Graph API URL for file content operations.
    pub(crate) fn item_url(&self, path: &str) -> String {
        format!("{}{}:/content", GRAPH_DRIVE_ROOT, self.full_path(path))
    }

    fn auth_header(&self) -> String {
        let tokens = self.tokens.lock().unwrap();
        format!("Bearer {}", tokens.access_token)
    }

    /// Exchange the stored refresh token for a new access token.
    /// Updates the stored access_token (and refresh_token if rotated) in place.
    ///
    /// The lock is never held across an await point: we clone what we need,
    /// drop the guard, perform the async I/O, then re-acquire to write.
    async fn refresh_access_token(&self) -> Result<(), SyncError> {
        // Phase 1: read the refresh token under the lock, then release it.
        let refresh_token = {
            let tokens = self.tokens.lock().unwrap();
            tokens.refresh_token.clone().ok_or_else(|| {
                SyncError::Auth("no refresh token available for OneDrive".to_string())
            })?
        };

        // Phase 2: async network call — no lock held.
        let body = build_token_refresh_body(&refresh_token);
        let resp: serde_json::Value = self
            .client
            .post(MICROSOFT_TOKEN_URL)
            .header("Content-Type", "application/x-www-form-urlencoded")
            .body(body)
            .send()
            .await?
            .json()
            .await?;

        let new_token = resp["access_token"]
            .as_str()
            .ok_or_else(|| SyncError::Auth(format!("token refresh failed: {resp}")))?
            .to_string();

        let new_refresh = resp["refresh_token"].as_str().map(|s| s.to_string());

        // Phase 3: write the updated tokens under the lock.
        let mut tokens = self.tokens.lock().unwrap();
        tokens.access_token = new_token;
        if let Some(r) = new_refresh {
            tokens.refresh_token = Some(r);
        }
        Ok(())
    }
}

#[async_trait]
impl SyncBackend for OneDriveBackend {
    async fn push(&self, path: &str, data: Bytes) -> Result<(), SyncError> {
        let url = self.item_url(path);
        let response = self
            .client
            .put(&url)
            .header("Authorization", self.auth_header())
            .header("Content-Type", "application/octet-stream")
            .body(data.clone()) // clone is O(1) — Bytes is Arc-backed
            .send()
            .await?;

        if response.status() == StatusCode::UNAUTHORIZED {
            // Token expired — refresh once and retry.
            self.refresh_access_token().await?;
            let response = self
                .client
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
        } else if !response.status().is_success() {
            return Err(SyncError::Backend(format!(
                "OneDrive push failed: {}",
                response.status()
            )));
        }
        Ok(())
    }

    async fn pull(&self, path: &str) -> Result<Bytes, SyncError> {
        let url = self.item_url(path);
        let response = self
            .client
            .get(&url)
            .header("Authorization", self.auth_header())
            .send()
            .await?;

        if response.status() == StatusCode::UNAUTHORIZED {
            // Token expired — refresh once and retry.
            self.refresh_access_token().await?;
            let response = self
                .client
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
            return Ok(response.bytes().await?);
        }

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
        let url = format!("{}{}:/children", GRAPH_DRIVE_ROOT, self.full_path(prefix));
        let response = self
            .client
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
            .filter_map(|item| item["name"].as_str().map(|s| format!("{}/{}", prefix, s)))
            .collect();
        Ok(names)
    }

    async fn delete(&self, path: &str) -> Result<(), SyncError> {
        // Use the items endpoint for delete (the content endpoint doesn't support DELETE)
        let url = format!("{}{}", GRAPH_DRIVE_ROOT, self.full_path(path));
        let response = self
            .client
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
mod refresh_tests {
    use super::*;

    #[test]
    fn refresh_url_is_correct() {
        assert_eq!(
            MICROSOFT_TOKEN_URL,
            "https://login.microsoftonline.com/common/oauth2/v2.0/token"
        );
    }

    #[test]
    fn build_refresh_request_body() {
        let body = build_token_refresh_body("test-refresh-token");
        assert!(body.contains("grant_type=refresh_token"));
        assert!(body.contains("refresh_token=test-refresh-token"));
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

    #[test]
    fn test_with_refresh_token_constructor() {
        let backend =
            OneDriveBackend::with_refresh_token("access", Some("refresh"), "/vault");
        assert_eq!(backend.auth_header(), "Bearer access");
        assert_eq!(
            backend.tokens.lock().unwrap().refresh_token.as_deref(),
            Some("refresh")
        );
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
