// vector: sqlite-vec vector storage and search

use std::path::Path;
use std::sync::Once;

use rusqlite::{params, Connection};

use crate::SearchError;

/// One-time registration guard so `sqlite3_auto_extension` is called at most once per process.
static INIT_SQLITE_VEC: Once = Once::new();

/// SQLite-backed KNN vector store using sqlite-vec (vec0 virtual tables).
pub struct VectorIndex {
    conn: Connection,
}

impl VectorIndex {
    /// Open (or create) a VectorIndex at `path`.
    ///
    /// Registers the sqlite-vec extension via `sqlite3_auto_extension`, opens the SQLite
    /// connection at `path`, and ensures the `memory_vectors` virtual table exists.
    pub fn open(path: &Path) -> Result<Self, SearchError> {
        // Register the sqlite-vec extension exactly once for this process.
        INIT_SQLITE_VEC.call_once(|| unsafe {
            rusqlite::ffi::sqlite3_auto_extension(Some(std::mem::transmute::<
                *const (),
                unsafe extern "C" fn(
                    *mut rusqlite::ffi::sqlite3,
                    *mut *const i8,
                    *const rusqlite::ffi::sqlite3_api_routines,
                ) -> i32,
            >(sqlite_vec::sqlite3_vec_init as *const ())));
        });

        let conn = Connection::open(path).map_err(|e| SearchError::Db(e.to_string()))?;

        conn.execute_batch(
            "CREATE VIRTUAL TABLE IF NOT EXISTS memory_vectors \
             USING vec0(embedding float[384], +memory_id text);",
        )
        .map_err(|e| SearchError::Db(e.to_string()))?;

        Ok(Self { conn })
    }

    /// Insert an embedding together with its associated `memory_id`.
    ///
    /// The embedding is serialized as a JSON array before being stored.
    pub fn insert(&self, memory_id: &str, embedding: &[f32]) -> Result<(), SearchError> {
        let embedding_json =
            serde_json::to_string(embedding).map_err(|e| SearchError::Db(e.to_string()))?;

        self.conn
            .execute(
                "INSERT INTO memory_vectors (embedding, memory_id) VALUES (?, ?)",
                params![embedding_json, memory_id],
            )
            .map_err(|e| SearchError::Db(e.to_string()))?;

        Ok(())
    }

    /// Return the `limit` nearest neighbours to `query_embedding`, ordered by ascending L2 distance.
    ///
    /// Returns `(memory_id, distance)` pairs.
    ///
    /// sqlite-vec requires the `k` limit to be visible at query-planning time, so we embed it
    /// directly in the SQL string via `k = <literal>` in the WHERE clause.  The spec's
    /// `LIMIT ?2` form does not work when the limit is a bound parameter because the virtual
    /// table planner never sees it.
    pub fn knn_search(
        &self,
        query_embedding: &[f32],
        limit: usize,
    ) -> Result<Vec<(String, f32)>, SearchError> {
        let query_json =
            serde_json::to_string(query_embedding).map_err(|e| SearchError::Db(e.to_string()))?;

        // sqlite-vec requires the `k` limit to be a compile-time-visible constraint.
        // Using `k = ?` in the WHERE clause is the canonical approach documented by sqlite-vec.
        let mut stmt = self
            .conn
            .prepare(
                "SELECT memory_id, distance \
                 FROM memory_vectors \
                 WHERE embedding MATCH ?1 \
                 AND k = ?2 \
                 ORDER BY distance",
            )
            .map_err(|e| SearchError::Db(e.to_string()))?;

        let rows = stmt
            .query_map(params![query_json, limit as i64], |row| {
                let memory_id: String = row.get(0)?;
                let distance: f32 = row.get(1)?;
                Ok((memory_id, distance))
            })
            .map_err(|e| SearchError::Db(e.to_string()))?;

        let mut results = Vec::new();
        for row in rows {
            results.push(row.map_err(|e| SearchError::Db(e.to_string()))?);
        }

        Ok(results)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    /// Helper: create a 384-dimensional zero vector.
    fn zero_vec() -> Vec<f32> {
        vec![0.0_f32; 384]
    }

    /// Helper: create a 384-dimensional all-ones vector.
    fn ones_vec() -> Vec<f32> {
        vec![1.0_f32; 384]
    }

    /// Helper: open a fresh VectorIndex backed by a temp file.
    fn make_index() -> (VectorIndex, TempDir) {
        let dir = TempDir::new().unwrap();
        let db_path = dir.path().join("test_vectors.db");
        let index = VectorIndex::open(&db_path).unwrap();
        (index, dir)
    }

    #[test]
    fn test_insert_and_knn_finds_inserted_vector() {
        let (index, _dir) = make_index();

        index.insert("vec-zero", &zero_vec()).unwrap();

        let results = index.knn_search(&zero_vec(), 1).unwrap();
        assert_eq!(results.len(), 1, "should find exactly one result");
        assert_eq!(
            results[0].0, "vec-zero",
            "should return the inserted memory_id"
        );
    }

    #[test]
    fn test_knn_returns_nearest_neighbor_first() {
        let (index, _dir) = make_index();

        // Insert two vectors: zero and ones.
        index.insert("vec-zero", &zero_vec()).unwrap();
        index.insert("vec-ones", &ones_vec()).unwrap();

        // Query near the zero vector — zero-vec should rank first (smaller distance).
        let results = index.knn_search(&zero_vec(), 2).unwrap();
        assert_eq!(results.len(), 2, "should return 2 results");
        assert_eq!(
            results[0].0, "vec-zero",
            "vec-zero should be the nearest neighbour"
        );
        assert!(
            results[0].1 < results[1].1,
            "nearest neighbour should have a strictly smaller distance: {} vs {}",
            results[0].1,
            results[1].1
        );
    }

    #[test]
    fn test_knn_with_limit_returns_at_most_limit_results() {
        let (index, _dir) = make_index();

        // Insert 5 distinct vectors.
        for i in 0..5u8 {
            let mut v = vec![0.0_f32; 384];
            v[0] = f32::from(i);
            index.insert(&format!("vec-{i}"), &v).unwrap();
        }

        // Request at most 3 results.
        let results = index.knn_search(&zero_vec(), 3).unwrap();
        assert!(
            results.len() <= 3,
            "knn_search with limit=3 should return at most 3 results, got {}",
            results.len()
        );
    }
}
