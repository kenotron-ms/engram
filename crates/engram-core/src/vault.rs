// vault.rs — Vault file I/O module

use std::fs;
use std::path::{Path, PathBuf};
use thiserror::Error;

/// Errors produced by Vault operations.
#[derive(Debug, Error)]
pub enum VaultError {
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    #[error("path escapes vault root")]
    PathEscape,
}

/// A handle to a directory-backed vault.
pub struct Vault {
    root: PathBuf,
}

impl Vault {
    /// Create a new `Vault` rooted at `root`.
    pub fn new(root: impl AsRef<Path>) -> Self {
        Self {
            root: root.as_ref().to_path_buf(),
        }
    }

    /// Return the vault root path.
    pub fn root(&self) -> &Path {
        &self.root
    }

    /// Read a file at `relative_path` inside the vault.
    pub fn read(&self, relative_path: &str) -> Result<String, VaultError> {
        let full = self.root.join(relative_path);
        let content = fs::read_to_string(full)?;
        Ok(content)
    }

    /// Write `content` to `relative_path` inside the vault,
    /// creating any missing parent directories.
    pub fn write(&self, relative_path: &str, content: &str) -> Result<(), VaultError> {
        let full = self.root.join(relative_path);
        if let Some(parent) = full.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(full, content)?;
        Ok(())
    }

    /// Recursively list all `.md` files under the vault root,
    /// returning paths relative to the root with forward slashes.
    pub fn list_markdown(&self) -> Result<Vec<String>, VaultError> {
        let mut results = Vec::new();
        self.walk_dir(&self.root.clone(), &mut results)?;
        Ok(results)
    }

    // --- private helpers ---

    fn walk_dir(&self, dir: &Path, results: &mut Vec<String>) -> Result<(), VaultError> {
        for entry in fs::read_dir(dir)? {
            let entry = entry?;
            let path = entry.path();
            if path.is_dir() {
                self.walk_dir(&path, results)?;
            } else if let Some(ext) = path.extension() {
                if ext.eq_ignore_ascii_case("md") {
                    // Build a relative path with forward slashes for cross-platform consistency
                    let relative = path
                        .strip_prefix(&self.root)
                        .expect("walk_dir always descends into root");
                    let rel_str = relative
                        .components()
                        .map(|c| c.as_os_str().to_string_lossy())
                        .collect::<Vec<_>>()
                        .join("/");
                    results.push(rel_str);
                }
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn make_vault() -> (Vault, TempDir) {
        let dir = TempDir::new().unwrap();
        let vault = Vault::new(dir.path());
        (vault, dir)
    }

    #[test]
    fn test_write_and_read_file() {
        let (vault, _dir) = make_vault();
        vault.write("test.md", "hello world").unwrap();
        let content = vault.read("test.md").unwrap();
        assert_eq!(content, "hello world");
    }

    #[test]
    fn test_read_missing_file_returns_error() {
        let (vault, _dir) = make_vault();
        let result = vault.read("missing.md");
        assert!(result.is_err());
    }

    #[test]
    fn test_list_markdown_returns_md_files() {
        let (vault, _dir) = make_vault();
        vault.write("note1.md", "first").unwrap();
        vault.write("note2.md", "second").unwrap();
        vault.write("image.png", "not markdown").unwrap();
        let mut files = vault.list_markdown().unwrap();
        files.sort();
        assert_eq!(files, vec!["note1.md", "note2.md"]);
    }

    #[test]
    fn test_write_creates_parent_directories() {
        let (vault, _dir) = make_vault();
        vault.write("deep/nested/dir/file.md", "nested content").unwrap();
        let content = vault.read("deep/nested/dir/file.md").unwrap();
        assert_eq!(content, "nested content");
    }
}
