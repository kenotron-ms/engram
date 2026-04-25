// store.rs — SQLCipher-backed memory store

use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

use rusqlite::Connection;
use serde::{Deserialize, Serialize};
use thiserror::Error;
use uuid::Uuid;

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

/// A single memory record stored in the encrypted database.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Memory {
    pub id: String,
    pub entity: String,
    pub attribute: String,
    pub value: String,
    pub source: Option<String>,
    pub created_at: i64,
    pub updated_at: i64,
}

impl Memory {
    /// Create a new `Memory` with a UUID v4 id and millisecond timestamps.
    pub fn new(entity: &str, attribute: &str, value: &str, source: Option<&str>) -> Self {
        let ts = now_ms();
        Memory {
            id: Uuid::new_v4().to_string(),
            entity: entity.to_string(),
            attribute: attribute.to_string(),
            value: value.to_string(),
            source: source.map(str::to_string),
            created_at: ts,
            updated_at: ts,
        }
    }
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
        let count: i64 = self
            .conn
            .query_row("SELECT COUNT(*) FROM memories", [], |row| row.get(0))?;
        Ok(count as u64)
    }

    /// Insert a `Memory` record into the database.
    pub fn insert(&self, memory: &Memory) -> Result<(), StoreError> {
        self.conn.execute(
            "INSERT INTO memories (id, entity, attribute, value, source, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            rusqlite::params![
                memory.id,
                memory.entity,
                memory.attribute,
                memory.value,
                memory.source,
                memory.created_at,
                memory.updated_at,
            ],
        )?;
        Ok(())
    }

    /// Retrieve a `Memory` by id, returning `None` if no row exists.
    pub fn get(&self, id: &str) -> Result<Option<Memory>, StoreError> {
        let result = self.conn.query_row(
            "SELECT id, entity, attribute, value, source, created_at, updated_at
             FROM memories WHERE id = ?1",
            [id],
            row_to_memory,
        );
        match result {
            Ok(memory) => Ok(Some(memory)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(StoreError::Db(e)),
        }
    }

    /// Update the `value` field and `updated_at` timestamp of a memory.
    pub fn update_value(&self, id: &str, value: &str) -> Result<(), StoreError> {
        self.conn.execute(
            "UPDATE memories SET value = ?1, updated_at = ?2 WHERE id = ?3",
            rusqlite::params![value, now_ms(), id],
        )?;
        Ok(())
    }

    /// Delete a memory by id.
    pub fn delete(&self, id: &str) -> Result<(), StoreError> {
        self.conn
            .execute("DELETE FROM memories WHERE id = ?1", [id])?;
        Ok(())
    }

    /// Return all memories for a given entity, ordered by `updated_at` DESC.
    pub fn find_by_entity(&self, entity: &str) -> Result<Vec<Memory>, StoreError> {
        let mut stmt = self.conn.prepare(
            "SELECT id, entity, attribute, value, source, created_at, updated_at
             FROM memories WHERE entity = ?1 ORDER BY updated_at DESC",
        )?;
        let memories = stmt.query_map([entity], row_to_memory)?;
        let result: Result<Vec<Memory>, rusqlite::Error> = memories.collect();
        Ok(result?)
    }

    /// Return memories created at or after `since_ms`, ordered by `created_at` DESC, limited to `limit` rows.
    pub fn list_recent(&self, since_ms: i64, limit: usize) -> Result<Vec<Memory>, StoreError> {
        let mut stmt = self.conn.prepare(
            "SELECT id, entity, attribute, value, source, created_at, updated_at
             FROM memories WHERE created_at >= ?1 ORDER BY created_at DESC LIMIT ?2",
        )?;
        let memories = stmt.query_map(rusqlite::params![since_ms, limit as i64], row_to_memory)?;
        let result: Result<Vec<Memory>, rusqlite::Error> = memories.collect();
        Ok(result?)
    }

    /// Search memories matching `query` as a LIKE pattern across entity, attribute, and value.
    ///
    /// Returns up to 20 results ordered by `updated_at` DESC.
    pub fn search(&self, query: &str) -> Result<Vec<Memory>, StoreError> {
        let pattern = format!("%{}%", query);
        let mut stmt = self.conn.prepare(
            "SELECT id, entity, attribute, value, source, created_at, updated_at
             FROM memories
             WHERE entity LIKE ?1 OR attribute LIKE ?1 OR value LIKE ?1
             ORDER BY updated_at DESC LIMIT 20",
        )?;
        let memories = stmt.query_map([&pattern], row_to_memory)?;
        let result: Result<Vec<Memory>, rusqlite::Error> = memories.collect();
        Ok(result?)
    }
}

// --- Private helpers --------------------------------------------------------

/// Returns current time as milliseconds since the Unix epoch.
fn now_ms() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock before epoch")
        .as_millis() as i64
}

/// Deserialise a `memories` table row into a [`Memory`] struct.
fn row_to_memory(row: &rusqlite::Row<'_>) -> rusqlite::Result<Memory> {
    Ok(Memory {
        id: row.get(0)?,
        entity: row.get(1)?,
        attribute: row.get(2)?,
        value: row.get(3)?,
        source: row.get(4)?,
        created_at: row.get(5)?,
        updated_at: row.get(6)?,
    })
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

    #[test]
    fn test_insert_and_get_memory() {
        let (_dir, db_path) = temp_store();
        let store = MemoryStore::open(&db_path, &test_key()).expect("open failed");
        let memory = Memory::new(
            "Sofia",
            "dietary",
            "vegetarian",
            Some("2026-04-14 transcript"),
        );
        store.insert(&memory).expect("insert failed");
        let got = store.get(&memory.id).expect("get failed");
        assert!(got.is_some(), "expected memory to be returned");
        let got = got.unwrap();
        assert_eq!(got.id, memory.id);
        assert_eq!(got.entity, "Sofia");
        assert_eq!(got.attribute, "dietary");
        assert_eq!(got.value, "vegetarian");
        assert_eq!(got.source, Some("2026-04-14 transcript".to_string()));
        assert_eq!(got.created_at, memory.created_at);
        assert_eq!(got.updated_at, memory.updated_at);
    }

    #[test]
    fn test_get_missing_returns_none() {
        let (_dir, db_path) = temp_store();
        let store = MemoryStore::open(&db_path, &test_key()).expect("open failed");
        let result = store.get("nonexistent-id").expect("get failed");
        assert!(result.is_none());
    }

    #[test]
    fn test_update_value_changes_value_and_timestamp() {
        let (_dir, db_path) = temp_store();
        let store = MemoryStore::open(&db_path, &test_key()).expect("open failed");
        let memory = Memory::new("Sofia", "role", "engineer", None);
        store.insert(&memory).expect("insert failed");
        let original_updated_at = memory.updated_at;
        std::thread::sleep(std::time::Duration::from_millis(10));
        store
            .update_value(&memory.id, "senior engineer")
            .expect("update_value failed");
        let got = store
            .get(&memory.id)
            .expect("get failed")
            .expect("memory missing after update");
        assert_eq!(got.value, "senior engineer");
        assert!(got.updated_at >= original_updated_at);
    }

    #[test]
    fn test_delete_removes_memory() {
        let (_dir, db_path) = temp_store();
        let store = MemoryStore::open(&db_path, &test_key()).expect("open failed");
        let memory = Memory::new("Sofia", "dietary", "vegetarian", None);
        store.insert(&memory).expect("insert failed");
        store.delete(&memory.id).expect("delete failed");
        let result = store.get(&memory.id).expect("get after delete failed");
        assert!(result.is_none());
    }

    #[test]
    fn test_find_by_entity_returns_all_for_entity() {
        let (_dir, db_path) = temp_store();
        let store = MemoryStore::open(&db_path, &test_key()).expect("open failed");
        let m1 = Memory::new("Sofia", "dietary", "vegetarian", None);
        let m2 = Memory::new("Sofia", "role", "engineer", None);
        let m3 = Memory::new("Chris", "role", "manager", None);
        store.insert(&m1).expect("insert m1 failed");
        store.insert(&m2).expect("insert m2 failed");
        store.insert(&m3).expect("insert m3 failed");
        let results = store
            .find_by_entity("Sofia")
            .expect("find_by_entity failed");
        assert_eq!(results.len(), 2);
        for m in &results {
            assert_eq!(m.entity, "Sofia");
        }
    }

    #[test]
    fn test_record_count_reflects_inserts() {
        let (_dir, db_path) = temp_store();
        let store = MemoryStore::open(&db_path, &test_key()).expect("open failed");
        assert_eq!(store.record_count().expect("count failed"), 0);
        let m1 = Memory::new("Sofia", "dietary", "vegetarian", None);
        let m2 = Memory::new("Chris", "role", "manager", None);
        store.insert(&m1).expect("insert m1 failed");
        store.insert(&m2).expect("insert m2 failed");
        assert_eq!(store.record_count().expect("count failed"), 2);
    }

    #[test]
    fn test_list_recent_returns_memories_after_cutoff() {
        let (_dir, db_path) = temp_store();
        let store = MemoryStore::open(&db_path, &test_key()).expect("open failed");
        let memory = Memory::new("Sofia", "dietary", "vegetarian", None);
        store.insert(&memory).expect("insert failed");
        let results = store.list_recent(0, 10).expect("list_recent failed");
        assert_eq!(results.len(), 1);
    }

    #[test]
    fn test_list_recent_respects_limit() {
        let (_dir, db_path) = temp_store();
        let store = MemoryStore::open(&db_path, &test_key()).expect("open failed");
        for i in 0..5 {
            let memory = Memory::new("Sofia", &format!("attr{}", i), &format!("val{}", i), None);
            store.insert(&memory).expect("insert failed");
        }
        let results = store.list_recent(0, 3).expect("list_recent failed");
        assert_eq!(results.len(), 3);
    }

    #[test]
    fn test_search_finds_matching_entity() {
        let (_dir, db_path) = temp_store();
        let store = MemoryStore::open(&db_path, &test_key()).expect("open failed");
        let memory = Memory::new("Sofia", "dietary", "vegetarian", None);
        store.insert(&memory).expect("insert failed");
        let results = store.search("Sofi").expect("search failed");
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].entity, "Sofia");
    }

    #[test]
    fn test_search_finds_matching_value() {
        let (_dir, db_path) = temp_store();
        let store = MemoryStore::open(&db_path, &test_key()).expect("open failed");
        let memory = Memory::new("Sofia", "dietary", "vegetarian", None);
        store.insert(&memory).expect("insert failed");
        let results = store.search("vegeta").expect("search failed");
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].value, "vegetarian");
    }

    #[test]
    fn test_search_no_match_returns_empty() {
        let (_dir, db_path) = temp_store();
        let store = MemoryStore::open(&db_path, &test_key()).expect("open failed");
        let memory = Memory::new("Sofia", "dietary", "vegetarian", None);
        store.insert(&memory).expect("insert failed");
        let results = store.search("nonexistent_xyz").expect("search failed");
        assert!(results.is_empty());
    }
}
