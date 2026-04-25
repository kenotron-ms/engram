// indexer: tantivy full-text index management

use std::path::Path;

use hex;
use sha2::{Digest, Sha256};
use tantivy::collector::TopDocs;
use tantivy::query::{QueryParser, TermQuery};
use tantivy::schema::{IndexRecordOption, Schema, Value, STORED, STRING, TEXT};
use tantivy::{doc, Index, IndexReader, IndexWriter, ReloadPolicy, TantivyDocument, Term};

use engram_core::vault::Vault;

use crate::{SearchError, SearchResult, SearchSource};

/// Full-text indexer backed by tantivy.
pub struct TantivyIndexer {
    index: Index,
    reader: IndexReader,
    writer: IndexWriter,
    path_field: tantivy::schema::Field,
    body_field: tantivy::schema::Field,
    hash_field: tantivy::schema::Field,
}

/// Statistics from a vault indexing run.
pub struct IndexStats {
    pub indexed: usize,
    pub skipped: usize,
    pub total: usize,
}

impl TantivyIndexer {
    /// Open or create a tantivy index at `index_dir`.
    ///
    /// If `meta.json` exists in `index_dir`, opens the existing index;
    /// otherwise creates a new one with the expected schema.
    pub fn open(index_dir: &Path) -> Result<Self, SearchError> {
        let index = if index_dir.join("meta.json").exists() {
            Index::open_in_dir(index_dir)
                .map_err(|e| SearchError::Index(e.to_string()))?
        } else {
            let mut schema_builder = Schema::builder();
            // path: exact-match indexed, stored for retrieval
            schema_builder.add_text_field("path", STRING | STORED);
            // body: full-text indexed AND stored so we can return snippets
            schema_builder.add_text_field("body", TEXT | STORED);
            // content_hash: exact-match indexed, stored for comparison
            schema_builder.add_text_field("content_hash", STRING | STORED);
            let schema = schema_builder.build();

            std::fs::create_dir_all(index_dir)
                .map_err(|e| SearchError::Io(e.to_string()))?;

            Index::create_in_dir(index_dir, schema)
                .map_err(|e| SearchError::Index(e.to_string()))?
        };

        // Resolve field handles from the (potentially pre-existing) schema.
        let schema = index.schema();
        let path_field = schema
            .get_field("path")
            .map_err(|_| SearchError::Index("missing 'path' field in schema".to_string()))?;
        let body_field = schema
            .get_field("body")
            .map_err(|_| SearchError::Index("missing 'body' field in schema".to_string()))?;
        let hash_field = schema
            .get_field("content_hash")
            .map_err(|_| SearchError::Index("missing 'content_hash' field in schema".to_string()))?;

        let reader = index
            .reader_builder()
            .reload_policy(ReloadPolicy::OnCommitWithDelay)
            .try_into()
            .map_err(|e: tantivy::TantivyError| SearchError::Index(e.to_string()))?;

        let writer = index
            .writer(50_000_000)
            .map_err(|e| SearchError::Index(e.to_string()))?;

        Ok(Self {
            index,
            reader,
            writer,
            path_field,
            body_field,
            hash_field,
        })
    }

    /// Compute the SHA-256 of `content`, hex-encode it, and return the first 16 characters.
    pub fn content_hash(content: &str) -> String {
        let mut hasher = Sha256::new();
        hasher.update(content.as_bytes());
        let result = hasher.finalize();
        let hex_str = hex::encode(result);
        hex_str[..16].to_string()
    }

    /// Index a single file: delete any existing doc for `path`, add the new doc, then commit.
    pub fn index_file(&mut self, path: &str, content: &str) -> Result<(), SearchError> {
        // Delete any previously indexed version of this path.
        let path_term = Term::from_field_text(self.path_field, path);
        self.writer.delete_term(path_term);

        let hash = Self::content_hash(content);

        // Cache fields as copies to use in doc! without conflicting borrows.
        let path_field = self.path_field;
        let body_field = self.body_field;
        let hash_field = self.hash_field;

        let document = doc!(
            path_field => path,
            body_field => content,
            hash_field => hash,
        );
        self.writer
            .add_document(document)
            .map_err(|e| SearchError::Index(e.to_string()))?;

        self.writer
            .commit()
            .map_err(|e| SearchError::Index(e.to_string()))?;

        self.reader
            .reload()
            .map_err(|e| SearchError::Index(e.to_string()))?;

        Ok(())
    }

    /// Full-text BM25 search over the `body` field.  Returns up to `limit` results.
    pub fn search(&self, query: &str, limit: usize) -> Result<Vec<SearchResult>, SearchError> {
        let searcher = self.reader.searcher();
        let query_parser = QueryParser::for_index(&self.index, vec![self.body_field]);
        let parsed_query = query_parser
            .parse_query(query)
            .map_err(|e| SearchError::Index(e.to_string()))?;

        let top_docs = searcher
            .search(&parsed_query, &TopDocs::with_limit(limit))
            .map_err(|e| SearchError::Index(e.to_string()))?;

        let mut results = Vec::new();
        for (score, doc_address) in top_docs {
            let retrieved: TantivyDocument = searcher
                .doc(doc_address)
                .map_err(|e| SearchError::Index(e.to_string()))?;

            let path = retrieved
                .get_first(self.path_field)
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();

            let body = retrieved
                .get_first(self.body_field)
                .and_then(|v| v.as_str())
                .unwrap_or("");

            // Snippet = first 200 characters of the body.
            let snippet: String = body.chars().take(200).collect();

            results.push(SearchResult {
                path,
                snippet,
                score,
                source: SearchSource::FullText,
            });
        }

        Ok(results)
    }

    /// Return `true` when `path` is not in the index or its stored hash differs from `current_hash`.
    pub fn needs_reindex(&self, path: &str, current_hash: &str) -> bool {
        let searcher = self.reader.searcher();
        let term = Term::from_field_text(self.path_field, path);
        let query = TermQuery::new(term, IndexRecordOption::Basic);

        let top_docs = match searcher.search(&query, &TopDocs::with_limit(1)) {
            Ok(docs) => docs,
            Err(_) => return true,
        };

        if let Some((_, doc_address)) = top_docs.into_iter().next() {
            let doc: TantivyDocument = match searcher.doc(doc_address) {
                Ok(d) => d,
                Err(_) => return true,
            };
            if let Some(stored_hash) = doc.get_first(self.hash_field).and_then(|v| v.as_str()) {
                return stored_hash != current_hash;
            }
        }

        // Document not found → must index.
        true
    }

    /// Index all markdown files in `vault`, skipping unchanged files.
    ///
    /// Uses a single writer and a single commit at the end.
    pub fn index_vault(&mut self, vault: &Vault) -> Result<IndexStats, SearchError> {
        let files = vault
            .list_markdown()
            .map_err(|e| SearchError::Io(e.to_string()))?;

        let total = files.len();
        let mut indexed = 0usize;
        let mut skipped = 0usize;

        // Cache field handles as copies to avoid borrow conflicts.
        let path_field = self.path_field;
        let body_field = self.body_field;
        let hash_field = self.hash_field;

        for rel_path in &files {
            let content = vault
                .read(rel_path)
                .map_err(|e| SearchError::Io(e.to_string()))?;

            let current_hash = Self::content_hash(&content);

            if !self.needs_reindex(rel_path, &current_hash) {
                skipped += 1;
                continue;
            }

            // Delete any stale doc for this path, then add the updated version.
            let path_term = Term::from_field_text(path_field, rel_path.as_str());
            self.writer.delete_term(path_term);

            let document = doc!(
                path_field => rel_path.as_str(),
                body_field => content.as_str(),
                hash_field => current_hash,
            );
            self.writer
                .add_document(document)
                .map_err(|e| SearchError::Index(e.to_string()))?;

            indexed += 1;
        }

        // Single commit for the whole vault run.
        self.writer
            .commit()
            .map_err(|e| SearchError::Index(e.to_string()))?;

        self.reader
            .reload()
            .map_err(|e| SearchError::Index(e.to_string()))?;

        Ok(IndexStats {
            indexed,
            skipped,
            total,
        })
    }

    /// Return the total number of documents currently visible in the index.
    pub fn indexed_doc_count(&self) -> usize {
        self.reader.searcher().num_docs() as usize
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use engram_core::vault::Vault;
    use tempfile::TempDir;

    fn make_indexer() -> (TantivyIndexer, TempDir) {
        let dir = TempDir::new().unwrap();
        let indexer = TantivyIndexer::open(dir.path()).unwrap();
        (indexer, dir)
    }

    #[test]
    fn test_content_hash_is_16_hex_chars() {
        let hash = TantivyIndexer::content_hash("some content");
        assert_eq!(hash.len(), 16, "hash must be exactly 16 characters");
        assert!(
            hash.chars().all(|c| c.is_ascii_hexdigit()),
            "hash must contain only hex chars, got: {hash}"
        );
    }

    #[test]
    fn test_same_content_produces_same_hash() {
        let h1 = TantivyIndexer::content_hash("hello world");
        let h2 = TantivyIndexer::content_hash("hello world");
        assert_eq!(h1, h2, "same input must produce same hash");
    }

    #[test]
    fn test_different_content_produces_different_hash() {
        let h1 = TantivyIndexer::content_hash("hello world");
        let h2 = TantivyIndexer::content_hash("goodbye world");
        assert_ne!(h1, h2, "different inputs must produce different hashes");
    }

    #[test]
    fn test_index_file_then_search_finds_result() {
        let (mut indexer, _dir) = make_indexer();
        indexer
            .index_file(
                "notes/test.md",
                "The quick brown fox jumps over the lazy dog",
            )
            .unwrap();

        let results = indexer.search("quick brown fox", 10).unwrap();
        assert!(!results.is_empty(), "search should find at least one result");
        assert_eq!(results[0].path, "notes/test.md");
    }

    #[test]
    fn test_search_returns_empty_for_no_matches() {
        let (mut indexer, _dir) = make_indexer();
        indexer
            .index_file("notes/test.md", "The quick brown fox")
            .unwrap();

        let results = indexer
            .search("completely unrelated elephants", 10)
            .unwrap();
        assert!(
            results.is_empty(),
            "search should return empty for no matches"
        );
    }

    // --- Incremental indexing tests ---

    #[test]
    fn test_needs_reindex_returns_true_for_missing_path() {
        let (indexer, _dir) = make_indexer();
        // A path that was never indexed must require indexing.
        let hash = TantivyIndexer::content_hash("some content");
        assert!(
            indexer.needs_reindex("never/indexed.md", &hash),
            "needs_reindex should return true for a path that has never been indexed"
        );
    }

    #[test]
    fn test_needs_reindex_returns_false_when_hash_unchanged() {
        let (mut indexer, _dir) = make_indexer();
        let content = "this is my note content";
        indexer.index_file("notes/unchanged.md", content).unwrap();

        let hash = TantivyIndexer::content_hash(content);
        assert!(
            !indexer.needs_reindex("notes/unchanged.md", &hash),
            "needs_reindex should return false when the stored hash matches the current hash"
        );
    }

    #[test]
    fn test_needs_reindex_returns_true_when_hash_changed() {
        let (mut indexer, _dir) = make_indexer();
        let original = "original content";
        indexer.index_file("notes/note.md", original).unwrap();

        // Compute a hash for *different* content.
        let different_hash = TantivyIndexer::content_hash("modified content");
        assert!(
            indexer.needs_reindex("notes/note.md", &different_hash),
            "needs_reindex should return true when the stored hash differs from the current hash"
        );
    }

    #[test]
    fn test_index_vault_first_run_indexes_all_files() {
        let index_dir = TempDir::new().unwrap();
        let vault_dir = TempDir::new().unwrap();
        let vault = Vault::new(vault_dir.path());

        vault.write("note1.md", "First note content").unwrap();
        vault.write("note2.md", "Second note content").unwrap();

        let mut indexer = TantivyIndexer::open(index_dir.path()).unwrap();
        let stats = indexer.index_vault(&vault).unwrap();

        assert_eq!(stats.total, 2, "total should be 2");
        assert_eq!(stats.indexed, 2, "indexed should be 2 on first run");
        assert_eq!(stats.skipped, 0, "skipped should be 0 on first run");
    }

    #[test]
    fn test_index_vault_second_run_skips_unchanged_files() {
        let index_dir = TempDir::new().unwrap();
        let vault_dir = TempDir::new().unwrap();
        let vault = Vault::new(vault_dir.path());

        vault.write("note.md", "Unchanged content").unwrap();

        let mut indexer = TantivyIndexer::open(index_dir.path()).unwrap();

        // First run — file is new, must be indexed.
        let first_stats = indexer.index_vault(&vault).unwrap();
        assert_eq!(first_stats.indexed, 1, "first run: indexed should be 1");
        assert_eq!(first_stats.skipped, 0, "first run: skipped should be 0");

        // Second run — content unchanged, must be skipped.
        let second_stats = indexer.index_vault(&vault).unwrap();
        assert_eq!(second_stats.indexed, 0, "second run: indexed should be 0");
        assert_eq!(second_stats.skipped, 1, "second run: skipped should be 1");
    }

    #[test]
    fn test_index_vault_reindexes_changed_file() {
        let index_dir = TempDir::new().unwrap();
        let vault_dir = TempDir::new().unwrap();
        let vault = Vault::new(vault_dir.path());

        vault
            .write("note.md", "Original content about cats")
            .unwrap();

        let mut indexer = TantivyIndexer::open(index_dir.path()).unwrap();

        // First run — index the original content.
        indexer.index_vault(&vault).unwrap();

        // Modify the file.
        vault
            .write("note.md", "Updated content about dogs")
            .unwrap();

        // Second run — changed file must be reindexed.
        let stats = indexer.index_vault(&vault).unwrap();
        assert_eq!(stats.indexed, 1, "should have reindexed the 1 changed file");
        assert_eq!(stats.skipped, 0, "should have skipped 0 files");

        // The new content must be searchable.
        let results = indexer.search("dogs", 10).unwrap();
        assert!(
            !results.is_empty(),
            "updated content should be searchable after reindexing"
        );
        assert_eq!(results[0].path, "note.md");
    }
}
