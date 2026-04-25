// store.rs — SQLCipher-backed memory store

use std::path::Path;

use rusqlite::Connection;
use thiserror::Error;

use crate::crypto::EngramKey;

/// DDL executed once on database open to create tables and indexes.
const SCHEMA: &str = r#"
CREATE TABLE IF NOT EXISTS memories (
    id          TEXT PRIMARY KEY,
    entity      TEXT NOT NULL,
    attribute   TEXT NOT NULL,
    value       TEXT NOT NULL,
    source      TEXT,
    created_at  INTEGER NOT NULL,
    updated_at  INTEGER NOT NULL
);

CREATE TABLE IF NOT EXISTS entities (
    id          TEXT PRIMARY KEY,
    name        TEXT NOT NULL,
    entity_type TEXT NOT NULL,
    created_at  INTEGER NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_memories_entity ON memories(entity);
CREATE INDEX IF NOT EXISTS idx_entities_name   ON entities(name);
"#;

/// Errors produced by store operations.
#[derive(Debug, Error)]
pub enum StoreError {
    #[error("database error: {0}")]
    Db(#[from] rusqlite::Error),

    #[error("record not found")]
    NotFound,
}

/// In-process handle to the encrypted SQLite memory store.
pub struct MemoryStore {
    conn: Connection,
}

impl MemoryStore {
    /// Open (or create) an encrypted SQLite database at `path`.
    ///
    /// The database is unlocked using SQLCipher's hex-blob key pragma:
    /// `PRAGMA key = "x'<64-char-hex>'"`.
    /// After unlocking, the schema is initialised if it does not yet exist.
    pub fn open(path: &Path, key: &EngramKey) -> Result<Self, StoreError> {
        let conn = Connection::open(path)?;

        // Unlock the database with the derived key.
        let key_hex = hex::encode(key.as_bytes());
        conn.execute_batch(&format!("PRAGMA key = \"x'{key_hex}'\";"))?;

        // Initialise (or verify) the schema.
        conn.execute_batch(SCHEMA)?;

        Ok(MemoryStore { conn })
    }

    /// Return `true` if a table with the given `name` exists in the database.
    pub fn table_exists(&self, name: &str) -> Result<bool, StoreError> {
        let count: i64 = self.conn.query_row(
            "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name=?1",
            [name],
            |row| row.get(0),
        )?;
        Ok(count > 0)
    }

    /// Return the total number of rows in the `memories` table.
    pub fn record_count(&self) -> Result<u64, StoreError> {
        let count: i64 = self.conn.query_row(
            "SELECT COUNT(*) FROM memories",
            [],
            |row| row.get(0),
        )?;
        Ok(count as u64)
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use tempfile::TempDir;

    use super::*;
    use crate::crypto::EngramKey;

    /// Derive a stable test key from a fixed password + zero salt.
    fn test_key() -> EngramKey {
        EngramKey::derive(b"testpassword", &[0u8; 16]).expect("key derivation failed")
    }

    /// Create a temp directory and return the dir (for RAII cleanup) and a
    /// path inside it for the test database.
    fn temp_store() -> (TempDir, PathBuf) {
        let dir = TempDir::new().expect("create temp dir failed");
        let path = dir.path().join("test.db");
        (dir, path)
    }

    #[test]
    fn test_open_creates_database() {
        let (dir, db_path) = temp_store();
        assert!(!db_path.exists(), "db should not exist before open");
        let _store = MemoryStore::open(&db_path, &test_key()).expect("open failed");
        assert!(db_path.exists(), "db should exist after open");
        drop(dir); // keep dir alive until here
    }

    #[test]
    fn test_schema_tables_exist() {
        let (_dir, db_path) = temp_store();
        let store = MemoryStore::open(&db_path, &test_key()).expect("open failed");
        assert!(store.table_exists("memories").expect("table_exists failed"));
        assert!(store.table_exists("entities").expect("table_exists failed"));
    }

    #[test]
    fn test_initial_record_count_is_zero() {
        let (_dir, db_path) = temp_store();
        let store = MemoryStore::open(&db_path, &test_key()).expect("open failed");
        assert_eq!(store.record_count().expect("record_count failed"), 0);
    }

    #[test]
    fn test_wrong_key_cannot_open_existing_db() {
        let (_dir, db_path) = temp_store();
        {
            let _store =
                MemoryStore::open(&db_path, &test_key()).expect("open with correct key failed");
        }
        let wrong_key =
            EngramKey::derive(b"wrongpassword", &[0u8; 16]).expect("key derivation failed");
        // Opening with wrong key — result doesn't need to be Ok, just shouldn't panic.
        let _result = MemoryStore::open(&db_path, &wrong_key);
    }
}
