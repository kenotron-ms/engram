# Engram — Phase 2: Search Infrastructure

> **For agentic workers:** REQUIRED SUB-SKILL: Use `superpowers:subagent-driven-development` (recommended) or `superpowers:executing-plans` to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build `engram-search` — full-text search via tantivy (BM25), vector semantic search via sqlite-vec + fastembed AllMiniLML6V2, and hybrid RRF ranking — with `engram index` and `engram search` CLI commands.

**Architecture:** New `engram-search` crate with four modules: `indexer` (tantivy, content-hash incremental), `embedder` (fastembed, 384-dim vectors), `vector` (sqlite-vec KNN), `hybrid` (RRF merge). CLI commands index the vault on demand and search via any mode.

**Tech Stack:** tantivy 0.22, fastembed 4, sqlite-vec 0.1, rusqlite 0.31 (bundled), sha2 0.10, hex 0.4, serde_json 1

---

## Codebase Orientation

Before starting, understand these existing patterns:

- **`crates/engram-core/src/vault.rs`** — `Vault::new(path)`, `vault.read("relative/path.md") -> Result<String, VaultError>`, `vault.list_markdown() -> Result<Vec<String>, VaultError>`. The `walk_dir` helper skips hidden directories — do NOT place the `.engram/` index dir inside the vault root for this reason (use `~/.engram/search/` instead as described in Tasks 7–9).
- **`crates/engram-core/src/store.rs`** — `MemoryStore::open(path, key)`, uses `rusqlite` with `bundled-sqlcipher` feature. The `engram-search` crate uses plain `bundled` (no SQLCipher — the vector index is a local-only compute artifact, not encrypted).
- **`crates/engram-cli/src/main.rs`** — clap derive pattern. `Commands` enum adds one variant per subcommand. Package name is `"engram"` so CLI test commands use `-p engram`. Functions named `run_*`. Separator line: `println!("{}", "─".repeat(41));`. Path helpers (`default_vault_path()`, `default_store_path()`) use `directories::UserDirs`.
- **Test conventions** — Unit tests live in `#[cfg(test)] mod tests { ... }` at the bottom of the source file. `TempDir::new().unwrap()` for temp directories. Integration tests go in `crates/<crate>/tests/`.
- **Commit style** — `feat(search): ...`, `feat(cli): ...`, `chore(search): ...`

---

## File Structure Being Created

```
crates/engram-search/
├── Cargo.toml
└── src/
    ├── lib.rs          ← SearchError enum, SearchResult, SearchSource; pub mod declarations
    ├── indexer.rs      ← TantivyIndexer: index vault markdown, incremental via content hash
    ├── embedder.rs     ← Embedder: fastembed AllMiniLML6V2, embed text → Vec<f32>
    ├── vector.rs       ← VectorIndex: sqlite-vec KNN over memory facts
    └── hybrid.rs       ← HybridSearch: RRF merge of tantivy + vector results
tests/
    └── integration_test.rs
```

Also modified:
- `Cargo.toml` (workspace root) — add `"crates/engram-search"` to members
- `crates/engram-cli/Cargo.toml` — add `engram-search` dependency
- `crates/engram-cli/src/main.rs` — add `Index` + `Search` subcommands, update `run_status()`

---

## Task 1: Initialize engram-search crate

**Files:**
- Modify: `Cargo.toml` (workspace root)
- Create: `crates/engram-search/Cargo.toml`
- Create: `crates/engram-search/src/lib.rs`
- Create: `crates/engram-search/src/indexer.rs` (stub)
- Create: `crates/engram-search/src/embedder.rs` (stub)
- Create: `crates/engram-search/src/vector.rs` (stub)
- Create: `crates/engram-search/src/hybrid.rs` (stub)

**Step 1: Add engram-search to workspace**

Open `Cargo.toml` at the workspace root. Current content:

```toml
[workspace]
    members = [
        "crates/engram-core",
        "crates/engram-cli",
        "crates/engram-sync",
    ]
    resolver = "2"
```

Add `"crates/engram-search"`:

```toml
[workspace]
    members = [
        "crates/engram-core",
        "crates/engram-cli",
        "crates/engram-sync",
        "crates/engram-search",
    ]
    resolver = "2"
```

**Step 2: Create the crate directory**

```bash
mkdir -p ~/workspace/ms/engram/crates/engram-search/src
mkdir -p ~/workspace/ms/engram/crates/engram-search/tests
```

**Step 3: Create `crates/engram-search/Cargo.toml`**

```toml
[package]
name = "engram-search"
version = "0.1.0"
edition = "2021"

[dependencies]
engram-core = { path = "../engram-core" }
tantivy = "0.22"
fastembed = "4"
sqlite-vec = "0.1"
rusqlite = { version = "0.31", features = ["bundled"] }
serde = { version = "1", features = ["derive"] }
serde_json = "1"
thiserror = "2"
sha2 = "0.10"
hex = "0.4"

[dev-dependencies]
tempfile = "3"
```

**Step 4: Create `crates/engram-search/src/lib.rs`**

```rust
// engram-search: full-text and semantic search for the vault

pub mod embedder;
pub mod hybrid;
pub mod indexer;
pub mod vector;

use thiserror::Error;

/// Unified error type for all engram-search operations.
#[derive(Debug, Error)]
pub enum SearchError {
    #[error("indexing error: {0}")]
    Index(String),

    #[error("embedding error: {0}")]
    Embed(String),

    #[error("database error: {0}")]
    Db(String),

    #[error("I/O error: {0}")]
    Io(String),
}

/// Which search subsystem produced this result.
#[derive(Debug, Clone, PartialEq)]
pub enum SearchSource {
    FullText,
    Vector,
    Hybrid,
}

/// A single ranked search result returned to callers.
#[derive(Debug, Clone)]
pub struct SearchResult {
    /// Vault-relative file path (e.g., `People/Sofia.md`) or memory fact ID.
    pub path: String,
    /// Short excerpt of matched content.
    pub snippet: String,
    /// Combined relevance score (higher = more relevant).
    pub score: f32,
    /// Which search subsystem produced this result.
    pub source: SearchSource,
}
```

**Step 5: Create stub module files**

`crates/engram-search/src/indexer.rs`:
```rust
// stub — implemented in Task 2
```

`crates/engram-search/src/embedder.rs`:
```rust
// stub — implemented in Task 4
```

`crates/engram-search/src/vector.rs`:
```rust
// stub — implemented in Task 5
```

`crates/engram-search/src/hybrid.rs`:
```rust
// stub — implemented in Task 6
```

**Step 6: Verify the crate compiles**

Run from `~/workspace/ms/engram/`:
```bash
cargo build -p engram-search
```

Expected: compiles with zero errors (warnings about unused imports from stubs are fine).

**Step 7: Commit**

```bash
cd ~/workspace/ms/engram
git add Cargo.toml crates/engram-search/ && \
git commit -m "chore(search): initialize engram-search crate with stub modules"
```

---

## Task 2: TantivyIndexer — index a single file

**Files:**
- Modify: `crates/engram-search/src/indexer.rs`

**Step 1: Write the failing tests first**

Replace the stub in `crates/engram-search/src/indexer.rs` with tests only (no implementation yet):

```rust
// crates/engram-search/src/indexer.rs

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_content_hash_is_16_hex_chars() {
        let hash = TantivyIndexer::content_hash("hello world");
        assert_eq!(hash.len(), 16, "expected 16 hex chars (first 8 bytes of SHA-256)");
        assert!(hash.chars().all(|c| c.is_ascii_hexdigit()));
    }

    #[test]
    fn test_same_content_produces_same_hash() {
        let h1 = TantivyIndexer::content_hash("Sofia is vegetarian");
        let h2 = TantivyIndexer::content_hash("Sofia is vegetarian");
        assert_eq!(h1, h2);
    }

    #[test]
    fn test_different_content_produces_different_hash() {
        let h1 = TantivyIndexer::content_hash("Sofia is vegetarian");
        let h2 = TantivyIndexer::content_hash("Chris is omnivore");
        assert_ne!(h1, h2);
    }

    #[test]
    fn test_index_file_then_search_finds_result() {
        let dir = TempDir::new().unwrap();
        let indexer = TantivyIndexer::open(dir.path()).unwrap();
        indexer
            .index_file("People/Sofia.md", "Sofia is vegetarian and lives in Seattle.")
            .unwrap();
        let results = indexer.search("vegetarian", 5).unwrap();
        assert!(!results.is_empty(), "expected at least one result for 'vegetarian'");
        assert_eq!(results[0].path, "People/Sofia.md");
    }

    #[test]
    fn test_search_returns_empty_for_no_matches() {
        let dir = TempDir::new().unwrap();
        let indexer = TantivyIndexer::open(dir.path()).unwrap();
        indexer
            .index_file("notes.md", "This is a note about apples.")
            .unwrap();
        let results = indexer.search("xenomorph", 5).unwrap();
        assert!(results.is_empty());
    }
}
```

**Step 2: Run tests to verify they fail**

```bash
cd ~/workspace/ms/engram
cargo test -p engram-search indexer
```

Expected: compile error — `TantivyIndexer` not defined.

**Step 3: Implement `indexer.rs`**

Replace the file with the full implementation plus the tests from Step 1:

```rust
// crates/engram-search/src/indexer.rs

use std::path::Path;

use sha2::{Digest, Sha256};
use tantivy::{
    collector::TopDocs,
    query::{QueryParser, TermQuery},
    schema::{Field, IndexRecordOption, Schema, Term, STORED, STRING, TEXT},
    Index, IndexReader, IndexWriter, ReloadPolicy,
};

use crate::{SearchError, SearchResult, SearchSource};

/// Full-text search index over vault markdown files.
pub struct TantivyIndexer {
    index: Index,
    schema: Schema,
    path_field: Field,
    body_field: Field,
    hash_field: Field,
    reader: IndexReader,
}

/// Statistics from a bulk vault index operation.
#[derive(Debug, Clone)]
pub struct IndexStats {
    /// Number of files newly indexed or reindexed.
    pub indexed: usize,
    /// Number of files skipped because content hash was unchanged.
    pub skipped: usize,
    /// Total number of markdown files in the vault.
    pub total: usize,
}

impl TantivyIndexer {
    /// Open (or create) a tantivy index at `index_dir`.
    ///
    /// If `index_dir` already contains a tantivy index (`meta.json` is
    /// present), the existing index is opened. Otherwise a new index is
    /// created with the standard schema.
    pub fn open(index_dir: &Path) -> Result<Self, SearchError> {
        std::fs::create_dir_all(index_dir)
            .map_err(|e| SearchError::Io(e.to_string()))?;

        let mut schema_builder = Schema::builder();
        // PATH: stored + indexed as a single token (no tokenization) for exact-match lookup.
        let path_field = schema_builder.add_text_field("path", STRING | STORED);
        // BODY: indexed with standard tokenizer for full-text BM25 search.
        let body_field = schema_builder.add_text_field("body", TEXT);
        // HASH: stored + indexed for incremental reindex detection.
        let hash_field = schema_builder.add_text_field("content_hash", STRING | STORED);
        let schema = schema_builder.build();

        let index = if index_dir.join("meta.json").exists() {
            Index::open_in_dir(index_dir)
                .map_err(|e| SearchError::Index(e.to_string()))?
        } else {
            Index::create_in_dir(index_dir, schema.clone())
                .map_err(|e| SearchError::Index(e.to_string()))?
        };

        let reader = index
            .reader_builder()
            .reload_policy(ReloadPolicy::OnCommit)
            .try_into()
            .map_err(|e: tantivy::TantivyError| SearchError::Index(e.to_string()))?;

        Ok(Self {
            index,
            schema,
            path_field,
            body_field,
            hash_field,
            reader,
        })
    }

    /// SHA-256 of `content`, hex-encoded, first 16 characters (8 bytes).
    pub fn content_hash(content: &str) -> String {
        let mut hasher = Sha256::new();
        hasher.update(content.as_bytes());
        let result = hasher.finalize();
        hex::encode(&result[..8])
    }

    /// Index a single file at `path` with `content`.
    ///
    /// If a document for `path` already exists in the index it is deleted
    /// before the new document is added (update semantics).
    pub fn index_file(&self, path: &str, content: &str) -> Result<(), SearchError> {
        let mut writer: IndexWriter = self
            .index
            .writer(50_000_000)
            .map_err(|e| SearchError::Index(e.to_string()))?;

        // Delete any existing document for this path.
        let term = Term::from_field_text(self.path_field, path);
        writer.delete_term(term);

        let mut doc = tantivy::Document::default();
        doc.add_text(self.path_field, path);
        doc.add_text(self.body_field, content);
        doc.add_text(self.hash_field, &Self::content_hash(content));
        writer
            .add_document(doc)
            .map_err(|e| SearchError::Index(e.to_string()))?;
        writer
            .commit()
            .map_err(|e| SearchError::Index(e.to_string()))?;
        self.reader
            .reload()
            .map_err(|e| SearchError::Index(e.to_string()))?;
        Ok(())
    }

    /// Full-text search over indexed documents.
    ///
    /// Returns up to `limit` results ranked by BM25 score descending.
    pub fn search(&self, query: &str, limit: usize) -> Result<Vec<SearchResult>, SearchError> {
        let searcher = self.reader.searcher();
        let query_parser =
            QueryParser::for_index(&self.index, vec![self.body_field]);
        let parsed = query_parser
            .parse_query(query)
            .map_err(|e| SearchError::Index(e.to_string()))?;
        let top_docs = searcher
            .search(&parsed, &TopDocs::with_limit(limit))
            .map_err(|e| SearchError::Index(e.to_string()))?;

        let mut results = Vec::new();
        for (score, doc_address) in top_docs {
            let doc = searcher
                .doc(doc_address)
                .map_err(|e| SearchError::Index(e.to_string()))?;
            let path = doc
                .get_first(self.path_field)
                .and_then(|v| v.as_text())
                .unwrap_or("")
                .to_string();
            let body = doc
                .get_first(self.body_field)
                .and_then(|v| v.as_text())
                .unwrap_or("");
            // Simple excerpt: first 200 characters of the body field.
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

    /// Returns `true` if the document at `path` is absent from the index or
    /// if its stored hash differs from `current_hash`.
    pub fn needs_reindex(&self, path: &str, current_hash: &str) -> bool {
        let searcher = self.reader.searcher();
        let term = Term::from_field_text(self.path_field, path);
        let query = TermQuery::new(term, IndexRecordOption::Basic);
        let top_docs = match searcher.search(&query, &TopDocs::with_limit(1)) {
            Ok(docs) => docs,
            Err(_) => return true,
        };
        if let Some((_score, doc_address)) = top_docs.first() {
            if let Ok(doc) = searcher.doc(*doc_address) {
                let stored_hash = doc
                    .get_first(self.hash_field)
                    .and_then(|v| v.as_text())
                    .unwrap_or("");
                return stored_hash != current_hash;
            }
        }
        true // not in index — needs indexing
    }

    /// Bulk-index all `.md` files in `vault`.
    ///
    /// Files whose content hash is unchanged since the last `index_vault` run
    /// are skipped. All new/changed files are written in a single writer
    /// commit for efficiency.
    pub fn index_vault(
        &self,
        vault: &engram_core::vault::Vault,
    ) -> Result<IndexStats, SearchError> {
        let files = vault
            .list_markdown()
            .map_err(|e| SearchError::Io(e.to_string()))?;
        let total = files.len();
        let mut indexed = 0usize;
        let mut skipped = 0usize;

        let mut writer: IndexWriter = self
            .index
            .writer(50_000_000)
            .map_err(|e| SearchError::Index(e.to_string()))?;

        for path in &files {
            let content = vault
                .read(path)
                .map_err(|e| SearchError::Io(e.to_string()))?;
            let hash = Self::content_hash(&content);

            if !self.needs_reindex(path, &hash) {
                skipped += 1;
                continue;
            }

            // Delete any existing document for this path.
            let term = Term::from_field_text(self.path_field, path);
            writer.delete_term(term);

            let mut doc = tantivy::Document::default();
            doc.add_text(self.path_field, path);
            doc.add_text(self.body_field, &content);
            doc.add_text(self.hash_field, &hash);
            writer
                .add_document(doc)
                .map_err(|e| SearchError::Index(e.to_string()))?;
            indexed += 1;
        }

        writer
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

    /// Total number of documents currently in the index.
    pub fn indexed_doc_count(&self) -> usize {
        self.reader.searcher().num_docs() as usize
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_content_hash_is_16_hex_chars() {
        let hash = TantivyIndexer::content_hash("hello world");
        assert_eq!(hash.len(), 16, "expected 16 hex chars (first 8 bytes of SHA-256)");
        assert!(hash.chars().all(|c| c.is_ascii_hexdigit()));
    }

    #[test]
    fn test_same_content_produces_same_hash() {
        let h1 = TantivyIndexer::content_hash("Sofia is vegetarian");
        let h2 = TantivyIndexer::content_hash("Sofia is vegetarian");
        assert_eq!(h1, h2);
    }

    #[test]
    fn test_different_content_produces_different_hash() {
        let h1 = TantivyIndexer::content_hash("Sofia is vegetarian");
        let h2 = TantivyIndexer::content_hash("Chris is omnivore");
        assert_ne!(h1, h2);
    }

    #[test]
    fn test_index_file_then_search_finds_result() {
        let dir = TempDir::new().unwrap();
        let indexer = TantivyIndexer::open(dir.path()).unwrap();
        indexer
            .index_file("People/Sofia.md", "Sofia is vegetarian and lives in Seattle.")
            .unwrap();
        let results = indexer.search("vegetarian", 5).unwrap();
        assert!(!results.is_empty(), "expected at least one result for 'vegetarian'");
        assert_eq!(results[0].path, "People/Sofia.md");
    }

    #[test]
    fn test_search_returns_empty_for_no_matches() {
        let dir = TempDir::new().unwrap();
        let indexer = TantivyIndexer::open(dir.path()).unwrap();
        indexer
            .index_file("notes.md", "This is a note about apples.")
            .unwrap();
        let results = indexer.search("xenomorph", 5).unwrap();
        assert!(results.is_empty());
    }
}
```

**Step 4: Run tests to verify they pass**

```bash
cd ~/workspace/ms/engram
cargo test -p engram-search indexer
```

Expected: `5 tests pass`.

**Step 5: Commit**

```bash
git add crates/engram-search/src/indexer.rs && \
git commit -m "feat(search): TantivyIndexer with index_file, content_hash, and search"
```

---

## Task 3: TantivyIndexer — incremental vault indexing

**Files:**
- Modify: `crates/engram-search/src/indexer.rs` (tests only — implementation already complete from Task 2)

`index_vault()`, `needs_reindex()`, and `IndexStats` were implemented in Task 2. This task adds tests to verify their behavior.

**Step 1: Add the vault tests to `indexer.rs`**

Append these tests to the `#[cfg(test)] mod tests` block at the bottom of `crates/engram-search/src/indexer.rs`:

```rust
    #[test]
    fn test_needs_reindex_returns_true_for_missing_path() {
        let dir = TempDir::new().unwrap();
        let indexer = TantivyIndexer::open(dir.path()).unwrap();
        assert!(indexer.needs_reindex("never/indexed.md", "anyhash"));
    }

    #[test]
    fn test_needs_reindex_returns_false_when_hash_unchanged() {
        let dir = TempDir::new().unwrap();
        let indexer = TantivyIndexer::open(dir.path()).unwrap();
        let content = "Sofia is vegetarian.";
        let hash = TantivyIndexer::content_hash(content);
        indexer.index_file("Sofia.md", content).unwrap();
        assert!(!indexer.needs_reindex("Sofia.md", &hash));
    }

    #[test]
    fn test_needs_reindex_returns_true_when_hash_changed() {
        let dir = TempDir::new().unwrap();
        let indexer = TantivyIndexer::open(dir.path()).unwrap();
        indexer.index_file("Sofia.md", "original content").unwrap();
        let new_hash = TantivyIndexer::content_hash("completely different content");
        assert!(indexer.needs_reindex("Sofia.md", &new_hash));
    }

    #[test]
    fn test_index_vault_first_run_indexes_all_files() {
        let index_dir = TempDir::new().unwrap();
        let vault_dir = TempDir::new().unwrap();
        std::fs::write(vault_dir.path().join("a.md"), "content a").unwrap();
        std::fs::write(vault_dir.path().join("b.md"), "content b").unwrap();
        let vault = engram_core::vault::Vault::new(vault_dir.path());

        let indexer = TantivyIndexer::open(index_dir.path()).unwrap();
        let stats = indexer.index_vault(&vault).unwrap();

        assert_eq!(stats.total, 2);
        assert_eq!(stats.indexed, 2);
        assert_eq!(stats.skipped, 0);
    }

    #[test]
    fn test_index_vault_second_run_skips_unchanged_files() {
        let index_dir = TempDir::new().unwrap();
        let vault_dir = TempDir::new().unwrap();
        std::fs::write(vault_dir.path().join("note.md"), "hello world").unwrap();
        let vault = engram_core::vault::Vault::new(vault_dir.path());
        let indexer = TantivyIndexer::open(index_dir.path()).unwrap();

        // First pass — should index
        let stats1 = indexer.index_vault(&vault).unwrap();
        assert_eq!(stats1.indexed, 1);
        assert_eq!(stats1.skipped, 0);

        // Second pass — same content, should skip
        let stats2 = indexer.index_vault(&vault).unwrap();
        assert_eq!(stats2.indexed, 0);
        assert_eq!(stats2.skipped, 1);
    }

    #[test]
    fn test_index_vault_reindexes_changed_file() {
        let index_dir = TempDir::new().unwrap();
        let vault_dir = TempDir::new().unwrap();
        let file_path = vault_dir.path().join("note.md");
        std::fs::write(&file_path, "original content").unwrap();
        let vault = engram_core::vault::Vault::new(vault_dir.path());
        let indexer = TantivyIndexer::open(index_dir.path()).unwrap();

        indexer.index_vault(&vault).unwrap();

        // Update the file
        std::fs::write(&file_path, "completely different content about Seattle").unwrap();

        let stats = indexer.index_vault(&vault).unwrap();
        assert_eq!(stats.indexed, 1, "changed file must be reindexed");
        assert_eq!(stats.skipped, 0);

        // Verify the new content is searchable
        let results = indexer.search("Seattle", 5).unwrap();
        assert!(!results.is_empty());
    }
```

**Step 2: Run the new tests**

```bash
cd ~/workspace/ms/engram
cargo test -p engram-search indexer
```

Expected: all tests pass (now 11 total in this module).

**Step 3: Commit**

```bash
git add crates/engram-search/src/indexer.rs && \
git commit -m "test(search): incremental vault indexing — needs_reindex and index_vault coverage"
```

---

## Task 4: Embedder — fastembed text embeddings

**Files:**
- Modify: `crates/engram-search/src/embedder.rs`

> **Note:** `Embedder::new()` downloads the AllMiniLML6V2 model (~90 MB) to `~/.cache/huggingface` on first call. Subsequent runs use the local cache and are fast.

**Step 1: Write the failing tests first**

Replace the stub in `crates/engram-search/src/embedder.rs` with tests only:

```rust
// crates/engram-search/src/embedder.rs

#[cfg(test)]
mod tests {
    use super::*;

    // Note: first run of any test in this module will download AllMiniLML6V2
    // (~90 MB) to ~/.cache/huggingface. Subsequent runs use the local cache.

    #[test]
    fn test_embed_produces_384_dimensions() {
        let embedder = Embedder::new().unwrap();
        let vec = embedder.embed("hello world").unwrap();
        assert_eq!(vec.len(), 384, "AllMiniLML6V2 produces 384-dimensional embeddings");
    }

    #[test]
    fn test_same_text_produces_same_vector() {
        let embedder = Embedder::new().unwrap();
        let v1 = embedder.embed("Sofia lives in Seattle").unwrap();
        let v2 = embedder.embed("Sofia lives in Seattle").unwrap();
        assert_eq!(v1, v2, "deterministic model: same text must produce same vector");
    }

    #[test]
    fn test_different_texts_produce_different_vectors() {
        let embedder = Embedder::new().unwrap();
        let v1 = embedder.embed("vegetarian diet preferences").unwrap();
        let v2 = embedder.embed("software engineering career").unwrap();
        assert_ne!(v1, v2);
    }

    #[test]
    fn test_embed_batch_returns_one_vector_per_input() {
        let embedder = Embedder::new().unwrap();
        let texts = ["first sentence", "second sentence", "third sentence"];
        let vecs = embedder.embed_batch(&texts).unwrap();
        assert_eq!(vecs.len(), 3);
        for v in &vecs {
            assert_eq!(v.len(), 384);
        }
    }
}
```

**Step 2: Run tests to verify compile failure**

```bash
cd ~/workspace/ms/engram
cargo test -p engram-search embedder
```

Expected: compile error — `Embedder` not defined.

**Step 3: Implement `embedder.rs`**

Replace the file with:

```rust
// crates/engram-search/src/embedder.rs

use fastembed::{EmbeddingModel, InitOptions, TextEmbedding};

use crate::SearchError;

/// Wraps fastembed's AllMiniLML6V2 model for text→vector conversion.
///
/// The model is downloaded to `~/.cache/huggingface` on first use and
/// cached locally for subsequent runs. Thread-safe via fastembed internals.
pub struct Embedder {
    model: TextEmbedding,
}

impl Embedder {
    /// Load the AllMiniLML6V2 embedding model.
    ///
    /// Downloads the model (~90 MB) to `~/.cache/huggingface` on first call.
    /// Subsequent calls load from cache and return quickly.
    pub fn new() -> Result<Self, SearchError> {
        let model = TextEmbedding::try_new(
            InitOptions::new(EmbeddingModel::AllMiniLML6V2),
        )
        .map_err(|e| SearchError::Embed(e.to_string()))?;
        Ok(Self { model })
    }

    /// Embed a single text string into a 384-dimensional f32 vector.
    pub fn embed(&self, text: &str) -> Result<Vec<f32>, SearchError> {
        let mut results = self
            .model
            .embed(vec![text], None)
            .map_err(|e| SearchError::Embed(e.to_string()))?;
        results
            .pop()
            .ok_or_else(|| SearchError::Embed("fastembed returned no embeddings".to_string()))
    }

    /// Embed a batch of texts. Returns one 384-dim vector per input, in order.
    pub fn embed_batch(&self, texts: &[&str]) -> Result<Vec<Vec<f32>>, SearchError> {
        self.model
            .embed(texts.to_vec(), None)
            .map_err(|e| SearchError::Embed(e.to_string()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_embed_produces_384_dimensions() {
        let embedder = Embedder::new().unwrap();
        let vec = embedder.embed("hello world").unwrap();
        assert_eq!(vec.len(), 384);
    }

    #[test]
    fn test_same_text_produces_same_vector() {
        let embedder = Embedder::new().unwrap();
        let v1 = embedder.embed("Sofia lives in Seattle").unwrap();
        let v2 = embedder.embed("Sofia lives in Seattle").unwrap();
        assert_eq!(v1, v2);
    }

    #[test]
    fn test_different_texts_produce_different_vectors() {
        let embedder = Embedder::new().unwrap();
        let v1 = embedder.embed("vegetarian diet preferences").unwrap();
        let v2 = embedder.embed("software engineering career").unwrap();
        assert_ne!(v1, v2);
    }

    #[test]
    fn test_embed_batch_returns_one_vector_per_input() {
        let embedder = Embedder::new().unwrap();
        let texts = ["first sentence", "second sentence", "third sentence"];
        let vecs = embedder.embed_batch(&texts).unwrap();
        assert_eq!(vecs.len(), 3);
        for v in &vecs {
            assert_eq!(v.len(), 384);
        }
    }
}
```

**Step 4: Run tests to verify they pass**

```bash
cd ~/workspace/ms/engram
cargo test -p engram-search embedder
```

Expected: `4 tests pass` (first run is slow — model download; subsequent runs are fast).

**Step 5: Commit**

```bash
git add crates/engram-search/src/embedder.rs && \
git commit -m "feat(search): Embedder with fastembed AllMiniLML6V2, embed + embed_batch"
```

---

## Task 5: VectorIndex — sqlite-vec KNN store

**Files:**
- Modify: `crates/engram-search/src/vector.rs`

**Step 1: Write the failing tests first**

Replace the stub with tests only:

```rust
// crates/engram-search/src/vector.rs

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn zero_vec() -> Vec<f32> {
        vec![0.0_f32; 384]
    }

    fn ones_vec() -> Vec<f32> {
        vec![1.0_f32; 384]
    }

    #[test]
    fn test_insert_and_knn_finds_inserted_vector() {
        let dir = TempDir::new().unwrap();
        let db_path = dir.path().join("vectors.db");
        let index = VectorIndex::open(&db_path).unwrap();

        index.insert("memory-001", &zero_vec()).unwrap();

        let results = index.knn_search(&zero_vec(), 1).unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].0, "memory-001");
    }

    #[test]
    fn test_knn_returns_nearest_neighbor_first() {
        let dir = TempDir::new().unwrap();
        let db_path = dir.path().join("vectors.db");
        let index = VectorIndex::open(&db_path).unwrap();

        index.insert("zero-vec", &zero_vec()).unwrap();
        index.insert("ones-vec", &ones_vec()).unwrap();

        // Query slightly above zero — zero-vec should be closer
        let query: Vec<f32> = vec![0.1_f32; 384];
        let results = index.knn_search(&query, 2).unwrap();

        assert_eq!(results.len(), 2);
        assert_eq!(results[0].0, "zero-vec", "nearest neighbor should be zero-vec");
        assert!(results[0].1 < results[1].1, "first result must have smaller distance");
    }

    #[test]
    fn test_knn_with_limit_returns_at_most_limit_results() {
        let dir = TempDir::new().unwrap();
        let db_path = dir.path().join("vectors.db");
        let index = VectorIndex::open(&db_path).unwrap();

        for i in 0..5 {
            let v: Vec<f32> = (0..384).map(|j| (i * j) as f32 / 1000.0).collect();
            index.insert(&format!("fact-{i}"), &v).unwrap();
        }

        let results = index.knn_search(&zero_vec(), 3).unwrap();
        assert!(results.len() <= 3);
    }
}
```

**Step 2: Run tests to verify compile failure**

```bash
cd ~/workspace/ms/engram
cargo test -p engram-search vector
```

Expected: compile error — `VectorIndex` not defined.

**Step 3: Implement `vector.rs`**

Replace the file with:

```rust
// crates/engram-search/src/vector.rs
//
// KNN vector search over memory facts using sqlite-vec.
//
// Extension loading: sqlite-vec is statically linked via the `sqlite-vec`
// crate. We register it with SQLite via sqlite3_auto_extension so it is
// available in every connection opened after this call. Call VectorIndex::open()
// before creating any other connections in the same process.
//
// sqlite-vec docs: https://alexgarcia.xyz/sqlite-vec/

use std::path::Path;

use rusqlite::{params, Connection};

use crate::SearchError;

/// Vector index backed by sqlite-vec for KNN semantic search.
pub struct VectorIndex {
    conn: Connection,
}

impl VectorIndex {
    /// Open (or create) a sqlite-vec database at `path`.
    ///
    /// Registers the sqlite-vec extension globally on first call, then opens
    /// the database and ensures the `memory_vectors` virtual table exists.
    pub fn open(path: &Path) -> Result<Self, SearchError> {
        // Register the sqlite-vec SQLite extension for all subsequent connections.
        // The `sqlite-vec` crate provides sqlite3_vec_init as the entry point.
        // The transmute is required because sqlite3_auto_extension expects a
        // no-arg function pointer while the actual init signature takes three args.
        unsafe {
            rusqlite::ffi::sqlite3_auto_extension(Some(std::mem::transmute::<
                unsafe extern "C" fn(
                    *mut rusqlite::ffi::sqlite3,
                    *mut *mut std::os::raw::c_char,
                    *mut rusqlite::ffi::sqlite3_api_routines,
                ) -> std::os::raw::c_int,
                unsafe extern "C" fn() -> std::os::raw::c_int,
            >(sqlite_vec::sqlite3_vec_init)));
        }

        let conn = Connection::open(path).map_err(|e| SearchError::Db(e.to_string()))?;

        // Create the virtual table if it does not exist.
        // +memory_id marks an auxiliary (metadata) column that is stored
        // alongside the vector but not used in distance calculations.
        conn.execute_batch(
            "CREATE VIRTUAL TABLE IF NOT EXISTS memory_vectors
             USING vec0(embedding float[384], +memory_id text);",
        )
        .map_err(|e| SearchError::Db(e.to_string()))?;

        Ok(Self { conn })
    }

    /// Insert a memory fact with its 384-dimensional embedding.
    ///
    /// `memory_id` is an opaque identifier (e.g., UUID or vault-relative path).
    /// `embedding` must be exactly 384 f32 values.
    pub fn insert(&self, memory_id: &str, embedding: &[f32]) -> Result<(), SearchError> {
        let json = serde_json::to_string(embedding)
            .map_err(|e| SearchError::Db(e.to_string()))?;
        self.conn
            .execute(
                "INSERT INTO memory_vectors(embedding, memory_id) VALUES (?1, ?2)",
                params![json, memory_id],
            )
            .map_err(|e| SearchError::Db(e.to_string()))?;
        Ok(())
    }

    /// K-nearest-neighbour search.
    ///
    /// Returns up to `limit` `(memory_id, distance)` pairs ordered by
    /// ascending distance (closest first). Distance is L2.
    pub fn knn_search(
        &self,
        query_embedding: &[f32],
        limit: usize,
    ) -> Result<Vec<(String, f32)>, SearchError> {
        let json = serde_json::to_string(query_embedding)
            .map_err(|e| SearchError::Db(e.to_string()))?;
        let mut stmt = self
            .conn
            .prepare(
                "SELECT memory_id, distance
                 FROM memory_vectors
                 WHERE embedding MATCH ?1
                 ORDER BY distance
                 LIMIT ?2",
            )
            .map_err(|e| SearchError::Db(e.to_string()))?;

        let results = stmt
            .query_map(params![json, limit as i64], |row| {
                Ok((row.get::<_, String>(0)?, row.get::<_, f32>(1)?))
            })
            .map_err(|e| SearchError::Db(e.to_string()))?;

        results
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| SearchError::Db(e.to_string()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn zero_vec() -> Vec<f32> {
        vec![0.0_f32; 384]
    }

    fn ones_vec() -> Vec<f32> {
        vec![1.0_f32; 384]
    }

    #[test]
    fn test_insert_and_knn_finds_inserted_vector() {
        let dir = TempDir::new().unwrap();
        let db_path = dir.path().join("vectors.db");
        let index = VectorIndex::open(&db_path).unwrap();

        index.insert("memory-001", &zero_vec()).unwrap();

        let results = index.knn_search(&zero_vec(), 1).unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].0, "memory-001");
    }

    #[test]
    fn test_knn_returns_nearest_neighbor_first() {
        let dir = TempDir::new().unwrap();
        let db_path = dir.path().join("vectors.db");
        let index = VectorIndex::open(&db_path).unwrap();

        index.insert("zero-vec", &zero_vec()).unwrap();
        index.insert("ones-vec", &ones_vec()).unwrap();

        let query: Vec<f32> = vec![0.1_f32; 384];
        let results = index.knn_search(&query, 2).unwrap();

        assert_eq!(results.len(), 2);
        assert_eq!(results[0].0, "zero-vec");
        assert!(results[0].1 < results[1].1);
    }

    #[test]
    fn test_knn_with_limit_returns_at_most_limit_results() {
        let dir = TempDir::new().unwrap();
        let db_path = dir.path().join("vectors.db");
        let index = VectorIndex::open(&db_path).unwrap();

        for i in 0..5 {
            let v: Vec<f32> = (0..384).map(|j| (i * j) as f32 / 1000.0).collect();
            index.insert(&format!("fact-{i}"), &v).unwrap();
        }

        let results = index.knn_search(&zero_vec(), 3).unwrap();
        assert!(results.len() <= 3);
    }
}
```

> **If the `transmute` does not compile:** The `sqlite3_vec_init` function signature may differ in your version of the `sqlite-vec` crate. Run `cargo doc -p sqlite-vec --open` and check the exported init function signature. Adjust the `transmute` type parameters to match exactly.

**Step 4: Run tests to verify they pass**

```bash
cd ~/workspace/ms/engram
cargo test -p engram-search vector
```

Expected: `3 tests pass`.

**Step 5: Commit**

```bash
git add crates/engram-search/src/vector.rs && \
git commit -m "feat(search): VectorIndex with sqlite-vec KNN insert and knn_search"
```

---

## Task 6: HybridSearch — RRF merge of full-text and vector results

**Files:**
- Modify: `crates/engram-search/src/hybrid.rs`

**Step 1: Write the failing tests first**

Replace the stub with tests only:

```rust
// crates/engram-search/src/hybrid.rs

#[cfg(test)]
mod tests {
    use super::*;
    use engram_core::vault::Vault;
    use tempfile::TempDir;
    use crate::{embedder::Embedder, indexer::TantivyIndexer, vector::VectorIndex};

    fn setup() -> (TantivyIndexer, VectorIndex, Embedder, TempDir, TempDir) {
        let index_dir = TempDir::new().unwrap();
        let vault_dir = TempDir::new().unwrap();
        let db_path = index_dir.path().join("vectors.db");
        let indexer = TantivyIndexer::open(index_dir.path()).unwrap();
        let vector_index = VectorIndex::open(&db_path).unwrap();
        let embedder = Embedder::new().unwrap();
        (indexer, vector_index, embedder, index_dir, vault_dir)
    }

    #[test]
    fn test_hybrid_search_returns_results() {
        let (indexer, vector_index, embedder, _idx, vault_dir) = setup();

        let vault = Vault::new(vault_dir.path());
        vault.write("Sofia.md", "Sofia is vegetarian and lives in Seattle.").unwrap();
        indexer.index_vault(&vault).unwrap();

        let embedding = embedder.embed("Sofia is vegetarian").unwrap();
        vector_index.insert("Sofia.md", &embedding).unwrap();

        let hybrid = HybridSearch::new(indexer, vector_index, embedder);
        let results = hybrid.search("Sofia vegetarian", 5).unwrap();
        assert!(!results.is_empty());
    }

    #[test]
    fn test_rrf_boosts_item_appearing_in_both_lists() {
        // RRF guarantee: a doc in BOTH lists scores higher than one in only one list.
        let (indexer, vector_index, embedder, _idx, vault_dir) = setup();

        let vault = Vault::new(vault_dir.path());
        vault.write("Sofia.md", "Sofia is vegetarian").unwrap();
        vault.write("Tasks.md", "Review project tasks for next sprint").unwrap();
        indexer.index_vault(&vault).unwrap();

        // Only Sofia.md gets a vector embedding — so it appears in both lists.
        let sofia_emb = embedder.embed("Sofia is vegetarian").unwrap();
        vector_index.insert("Sofia.md", &sofia_emb).unwrap();

        let hybrid = HybridSearch::new(indexer, vector_index, embedder);
        let results = hybrid.search("Sofia vegetarian", 5).unwrap();

        assert!(!results.is_empty());
        // Sofia.md must rank first because it appears in both FT and vector results.
        assert_eq!(results[0].path, "Sofia.md");
    }

    #[test]
    fn test_rrf_score_formula_k60() {
        // Verify the RRF scoring formula: score = 1/(k + rank), k = 60.
        // Rank 0 in one list: 1/(60+1) ≈ 0.01639
        // Rank 0 in two lists: 2 * 1/(60+1) ≈ 0.03279
        let k = 60.0_f32;
        let single_list_score = 1.0 / (k + 1.0); // rank 0
        let dual_list_score = 2.0 * (1.0 / (k + 1.0)); // rank 0 in both
        assert!(dual_list_score > single_list_score);
        assert!((dual_list_score - 0.0328).abs() < 0.001);
    }
}
```

**Step 2: Run tests to verify compile failure**

```bash
cd ~/workspace/ms/engram
cargo test -p engram-search hybrid
```

Expected: compile error — `HybridSearch` not defined.

**Step 3: Implement `hybrid.rs`**

Replace the file with:

```rust
// crates/engram-search/src/hybrid.rs
//
// Reciprocal Rank Fusion (RRF) over tantivy full-text + sqlite-vec vector results.
//
// RRF formula: score(doc) = Σ_lists 1 / (k + rank(doc, list))
// where k = 60 (standard value from Cormack, Clarke & Buettcher 2009).
// Rank is 1-based (rank 1 = position 0 in the sorted results list).

use std::collections::HashMap;

use crate::{
    embedder::Embedder, indexer::TantivyIndexer, vector::VectorIndex, SearchError,
    SearchResult, SearchSource,
};

const RRF_K: f32 = 60.0;

/// Combines full-text BM25 and KNN vector search with Reciprocal Rank Fusion.
pub struct HybridSearch {
    indexer: TantivyIndexer,
    vector_index: VectorIndex,
    embedder: Embedder,
}

impl HybridSearch {
    /// Create a new `HybridSearch` from the three search subsystems.
    pub fn new(
        indexer: TantivyIndexer,
        vector_index: VectorIndex,
        embedder: Embedder,
    ) -> Self {
        Self {
            indexer,
            vector_index,
            embedder,
        }
    }

    /// Hybrid search: run full-text and vector queries, merge with RRF.
    ///
    /// 1. Full-text (tantivy BM25) over indexed vault markdown.
    /// 2. Embed query → KNN over sqlite-vec memory facts.
    /// 3. RRF: sum `1/(k + rank)` contributions from each list per document.
    /// 4. Sort by combined score descending, return top `limit`.
    pub fn search(&self, query: &str, limit: usize) -> Result<Vec<SearchResult>, SearchError> {
        let candidate_pool = limit * 3;

        // --- Full-text results ---
        let ft_results = self.indexer.search(query, candidate_pool)?;

        // --- Vector results ---
        let query_embedding = self.embedder.embed(query)?;
        let vec_results = self.vector_index.knn_search(&query_embedding, candidate_pool)?;

        // --- RRF merge ---
        // We use the document id (path or memory_id) as the merge key.
        let mut scores: HashMap<String, f32> = HashMap::new();
        let mut snippets: HashMap<String, String> = HashMap::new();

        // Accumulate scores from full-text results (rank is 0-based internally,
        // but RRF uses 1/(k + 1-based rank)).
        for (rank, result) in ft_results.iter().enumerate() {
            let contribution = 1.0 / (RRF_K + (rank as f32 + 1.0));
            *scores.entry(result.path.clone()).or_insert(0.0) += contribution;
            snippets
                .entry(result.path.clone())
                .or_insert_with(|| result.snippet.clone());
        }

        // Accumulate scores from vector results.
        for (rank, (memory_id, _distance)) in vec_results.iter().enumerate() {
            let contribution = 1.0 / (RRF_K + (rank as f32 + 1.0));
            *scores.entry(memory_id.clone()).or_insert(0.0) += contribution;
        }

        // Sort by combined score descending.
        let mut ranked: Vec<(String, f32)> = scores.into_iter().collect();
        ranked.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

        // Build final SearchResult list.
        let results = ranked
            .into_iter()
            .take(limit)
            .map(|(id, combined_score)| {
                let snippet = snippets.get(&id).cloned().unwrap_or_default();
                SearchResult {
                    path: id,
                    snippet,
                    score: combined_score,
                    source: SearchSource::Hybrid,
                }
            })
            .collect();

        Ok(results)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{embedder::Embedder, indexer::TantivyIndexer, vector::VectorIndex};
    use engram_core::vault::Vault;
    use tempfile::TempDir;

    fn setup() -> (TantivyIndexer, VectorIndex, Embedder, TempDir, TempDir) {
        let index_dir = TempDir::new().unwrap();
        let vault_dir = TempDir::new().unwrap();
        let db_path = index_dir.path().join("vectors.db");
        let indexer = TantivyIndexer::open(index_dir.path()).unwrap();
        let vector_index = VectorIndex::open(&db_path).unwrap();
        let embedder = Embedder::new().unwrap();
        (indexer, vector_index, embedder, index_dir, vault_dir)
    }

    #[test]
    fn test_hybrid_search_returns_results() {
        let (indexer, vector_index, embedder, _idx, vault_dir) = setup();

        let vault = Vault::new(vault_dir.path());
        vault
            .write("Sofia.md", "Sofia is vegetarian and lives in Seattle.")
            .unwrap();
        indexer.index_vault(&vault).unwrap();

        let embedding = embedder.embed("Sofia is vegetarian").unwrap();
        vector_index.insert("Sofia.md", &embedding).unwrap();

        let hybrid = HybridSearch::new(indexer, vector_index, embedder);
        let results = hybrid.search("Sofia vegetarian", 5).unwrap();
        assert!(!results.is_empty());
    }

    #[test]
    fn test_rrf_boosts_item_appearing_in_both_lists() {
        let (indexer, vector_index, embedder, _idx, vault_dir) = setup();

        let vault = Vault::new(vault_dir.path());
        vault.write("Sofia.md", "Sofia is vegetarian").unwrap();
        vault
            .write("Tasks.md", "Review project tasks for next sprint")
            .unwrap();
        indexer.index_vault(&vault).unwrap();

        let sofia_emb = embedder.embed("Sofia is vegetarian").unwrap();
        vector_index.insert("Sofia.md", &sofia_emb).unwrap();

        let hybrid = HybridSearch::new(indexer, vector_index, embedder);
        let results = hybrid.search("Sofia vegetarian", 5).unwrap();

        assert!(!results.is_empty());
        assert_eq!(results[0].path, "Sofia.md");
    }

    #[test]
    fn test_rrf_score_formula_k60() {
        let k = 60.0_f32;
        let single_list_score = 1.0 / (k + 1.0);
        let dual_list_score = 2.0 * (1.0 / (k + 1.0));
        assert!(dual_list_score > single_list_score);
        assert!((dual_list_score - 0.0328).abs() < 0.001);
    }
}
```

**Step 4: Run tests to verify they pass**

```bash
cd ~/workspace/ms/engram
cargo test -p engram-search hybrid
```

Expected: `3 tests pass` (the embedding tests will be fast because fastembed is already cached from Task 4).

**Step 5: Commit**

```bash
git add crates/engram-search/src/hybrid.rs && \
git commit -m "feat(search): HybridSearch with RRF merge of full-text and vector results"
```

---

## Task 7: CLI — `engram index`

**Files:**
- Modify: `crates/engram-cli/Cargo.toml`
- Modify: `crates/engram-cli/src/main.rs`

**Step 1: Add `engram-search` dependency to CLI**

Open `crates/engram-cli/Cargo.toml`. Add `engram-search` to `[dependencies]`:

```toml
engram-search = { path = "../engram-search" }
```

The `[dependencies]` block should now include:

```toml
[dependencies]
engram-core = { path = "../engram-core" }
engram-sync = { path = "../engram-sync" }
engram-search = { path = "../engram-search" }
clap = { version = "4", features = ["derive"] }
directories = "5"
thiserror = "2"
anyhow = "1"
rpassword = "7"
tokio = { version = "1", features = ["full"] }
open = "5"
reqwest = { version = "0.12", features = ["json", "blocking"] }
serde_json = "1"
```

**Step 2: Add `Index` variant to the `Commands` enum in `main.rs`**

Open `crates/engram-cli/src/main.rs`. Find the `Commands` enum:

```rust
#[derive(Subcommand)]
enum Commands {
    /// Print vault state, memory store stats, and keyring status
    Status,
    /// Manage sync backend authentication
    Auth {
```

Add the `Index` variant before `Status`:

```rust
#[derive(Subcommand)]
enum Commands {
    /// Index vault content for full-text and vector search
    Index {
        /// Vault path (defaults to ~/.lifeos/memory)
        #[arg(long)]
        vault: Option<std::path::PathBuf>,
        /// Force reindex even if content hash is unchanged
        #[arg(long)]
        force: bool,
    },
    /// Print vault state, memory store stats, and keyring status
    Status,
    /// Manage sync backend authentication
    Auth {
```

**Step 3: Add `default_search_dir()` helper to `main.rs`**

Add this function alongside the existing `default_vault_path()` and `default_store_path()` helpers at the bottom of `main.rs` (before `#[cfg(test)]`):

```rust
/// Returns the default search index directory: `~/.engram/search`.
fn default_search_dir() -> std::path::PathBuf {
    UserDirs::new()
        .map(|u| u.home_dir().join(".engram/search"))
        .unwrap_or_else(|| std::path::PathBuf::from(".engram/search"))
}
```

**Step 4: Add `run_index()` function to `main.rs`**

Add this function alongside the other `run_*` functions (e.g., after `run_status()`):

```rust
/// Index vault markdown files for full-text search.
///
/// Uses content-hash comparison to skip files that haven't changed.
/// Pass `--force` to reindex everything regardless of hash.
fn run_index(vault_path: Option<std::path::PathBuf>, force: bool) {
    use engram_core::vault::Vault;
    use engram_search::indexer::TantivyIndexer;

    let vault_path = vault_path.unwrap_or_else(default_vault_path);
    let search_dir = default_search_dir();

    if !vault_path.exists() {
        eprintln!("Vault not found: {}", vault_path.display());
        eprintln!("Run: engram init");
        std::process::exit(1);
    }

    println!("Indexing {} ...", vault_path.display());

    // If --force, wipe the existing index directory so everything is reindexed.
    if force && search_dir.exists() {
        if let Err(e) = std::fs::remove_dir_all(&search_dir) {
            eprintln!("Failed to clear index: {e}");
            std::process::exit(1);
        }
    }

    let vault = Vault::new(&vault_path);
    let indexer = match TantivyIndexer::open(&search_dir) {
        Ok(i) => i,
        Err(e) => {
            eprintln!("Failed to open search index: {e}");
            std::process::exit(1);
        }
    };

    let stats = match indexer.index_vault(&vault) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("Indexing failed: {e}");
            std::process::exit(1);
        }
    };

    let size_mb = dir_size_bytes(&search_dir) as f64 / 1_048_576.0;

    println!("{}", "─".repeat(41));
    println!("Indexed:  {} files", stats.indexed);
    println!("Skipped:  {} files (unchanged)", stats.skipped);
    println!("Total:    {} files", stats.total);
    println!("Index:    {} ({:.1} MB)", search_dir.display(), size_mb);
}

/// Recursively sum the size in bytes of all files under `path`.
fn dir_size_bytes(path: &std::path::Path) -> u64 {
    let Ok(entries) = std::fs::read_dir(path) else {
        return 0;
    };
    entries
        .flatten()
        .map(|e| {
            let p = e.path();
            if p.is_file() {
                p.metadata().map(|m| m.len()).unwrap_or(0)
            } else if p.is_dir() {
                dir_size_bytes(&p)
            } else {
                0
            }
        })
        .sum()
}
```

**Step 5: Wire `Index` into `main()`**

Find the `match cli.command` block in `main()`:

```rust
    match cli.command {
        Commands::Status => run_status(),
        Commands::Auth { command } => match command {
```

Add the `Index` arm at the top:

```rust
    match cli.command {
        Commands::Index { vault, force } => run_index(vault, force),
        Commands::Status => run_status(),
        Commands::Auth { command } => match command {
```

**Step 6: Verify the binary compiles**

```bash
cd ~/workspace/ms/engram
cargo build -p engram
```

Expected: compiles with zero errors.

**Step 7: Smoke-test the command**

```bash
cargo run -p engram -- index
```

Expected output (numbers will vary based on your actual vault):
```
Indexing /Users/ken/.lifeos/memory ...
─────────────────────────────────────────
Indexed:  23 files
Skipped:  0 files (unchanged)
Total:    23 files
Index:    /Users/ken/.engram/search (1.1 MB)
```

Run it a second time to confirm the skipping behaviour:
```bash
cargo run -p engram -- index
```

Expected: `Indexed: 0 files`, `Skipped: 23 files (unchanged)`.

**Step 8: Commit**

```bash
git add crates/engram-cli/Cargo.toml crates/engram-cli/src/main.rs && \
git commit -m "feat(cli): engram index with incremental content-hash reindexing"
```

---

## Task 8: CLI — `engram search "<query>"`

**Files:**
- Modify: `crates/engram-cli/src/main.rs`

**Step 1: Add `Search` variant and `SearchMode` enum to `Commands`**

Find the `Commands` enum. Add `Search` after `Index`:

```rust
/// Search vault content
Search {
    /// Query string
    query: String,
    /// Maximum number of results to return
    #[arg(long, default_value = "10")]
    limit: usize,
    /// Search mode
    #[arg(long, value_enum, default_value = "hybrid")]
    mode: SearchMode,
},
```

Add the `SearchMode` enum **before** the `Commands` enum:

```rust
#[derive(clap::ValueEnum, Clone, Debug)]
enum SearchMode {
    /// BM25 full-text search only (tantivy)
    Fulltext,
    /// Semantic vector search only (sqlite-vec + fastembed)
    Vector,
    /// Hybrid full-text + vector with RRF ranking (default)
    Hybrid,
}
```

**Step 2: Add `default_vectors_path()` helper**

Add alongside the other default path helpers:

```rust
/// Returns the default vector index path: `~/.engram/vectors.db`.
fn default_vectors_path() -> std::path::PathBuf {
    UserDirs::new()
        .map(|u| u.home_dir().join(".engram/vectors.db"))
        .unwrap_or_else(|| std::path::PathBuf::from(".engram/vectors.db"))
}
```

**Step 3: Implement `run_search()` function**

Add after `run_index()`:

```rust
/// Execute a vault search query and print ranked results.
fn run_search(query: &str, limit: usize, mode: &SearchMode) {
    use engram_search::{
        embedder::Embedder,
        hybrid::HybridSearch,
        indexer::TantivyIndexer,
        vector::VectorIndex,
        SearchSource,
    };

    let search_dir = default_search_dir();
    if !search_dir.join("meta.json").exists() {
        eprintln!("Search index not built. Run: engram index");
        std::process::exit(1);
    }

    let indexer = match TantivyIndexer::open(&search_dir) {
        Ok(i) => i,
        Err(e) => {
            eprintln!("Failed to open search index: {e}");
            std::process::exit(1);
        }
    };

    let results = match mode {
        SearchMode::Fulltext => indexer.search(query, limit).unwrap_or_else(|e| {
            eprintln!("Search failed: {e}");
            std::process::exit(1);
        }),
        SearchMode::Vector | SearchMode::Hybrid => {
            let vectors_path = default_vectors_path();
            let vector_index = match VectorIndex::open(&vectors_path) {
                Ok(v) => v,
                Err(e) => {
                    eprintln!("Failed to open vector index: {e}");
                    std::process::exit(1);
                }
            };
            let embedder = match Embedder::new() {
                Ok(e) => e,
                Err(e) => {
                    eprintln!("Failed to load embedding model: {e}");
                    std::process::exit(1);
                }
            };
            match mode {
                SearchMode::Vector => {
                    let embedding = embedder.embed(query).unwrap_or_else(|e| {
                        eprintln!("Embedding failed: {e}");
                        std::process::exit(1);
                    });
                    vector_index
                        .knn_search(&embedding, limit)
                        .unwrap_or_else(|e| {
                            eprintln!("Vector search failed: {e}");
                            std::process::exit(1);
                        })
                        .into_iter()
                        .map(|(id, dist)| engram_search::SearchResult {
                            path: id,
                            snippet: String::new(),
                            score: 1.0 - dist, // convert distance to a score
                            source: SearchSource::Vector,
                        })
                        .collect()
                }
                _ => {
                    // Hybrid
                    let hybrid = HybridSearch::new(indexer, vector_index, embedder);
                    hybrid.search(query, limit).unwrap_or_else(|e| {
                        eprintln!("Search failed: {e}");
                        std::process::exit(1);
                    })
                }
            }
        }
    };

    let mode_label = match mode {
        SearchMode::Fulltext => "fulltext",
        SearchMode::Vector => "vector",
        SearchMode::Hybrid => "hybrid",
    };

    println!(
        "Results for \"{}\" ({}, {} results)",
        query,
        mode_label,
        results.len()
    );
    println!("{}", "─".repeat(49));

    if results.is_empty() {
        println!("No results found.");
        return;
    }

    for result in &results {
        println!("{} (score: {:.2})", result.path, result.score);
        if !result.snippet.is_empty() {
            println!("  ...{}...", result.snippet.trim());
        }
        println!();
    }
}
```

**Step 4: Wire `Search` into `main()`**

Add the `Search` arm to the `match cli.command` block:

```rust
        Commands::Search { query, limit, mode } => run_search(&query, limit, &mode),
```

The full `match cli.command` block should now begin:

```rust
    match cli.command {
        Commands::Index { vault, force } => run_index(vault, force),
        Commands::Search { query, limit, mode } => run_search(&query, limit, &mode),
        Commands::Status => run_status(),
        Commands::Auth { command } => match command {
```

**Step 5: Verify the binary compiles**

```bash
cd ~/workspace/ms/engram
cargo build -p engram
```

Expected: compiles with zero errors.

**Step 6: Smoke-test the search command**

First ensure the vault is indexed:
```bash
cargo run -p engram -- index
```

Then search:
```bash
cargo run -p engram -- search "vegetarian"
```

Expected output format:
```
Results for "vegetarian" (hybrid, 3 results)
─────────────────────────────────────────────────
People/Sofia.md (score: 0.03)
  ...Sofia is vegetarian and lives in Seattle...

```

**Step 7: Commit**

```bash
git add crates/engram-cli/src/main.rs && \
git commit -m "feat(cli): engram search with fulltext, vector, and hybrid modes"
```

---

## Task 9: Update `engram status` with search index stats

**Files:**
- Modify: `crates/engram-cli/src/main.rs`

**Step 1: Locate the end of `run_status()` in `main.rs`**

The function ends with the keyring status block:

```rust
    // ── Keyring status ──────────────────────────────────────────────────────
    match key_result {
        Ok(_) => println!("Key:          present ✓"),
        Err(_) => println!("Key:          not set"),
    }
}
```

**Step 2: Insert the search index block before the keyring block**

Find this code in `run_status()`:

```rust
    // ── Keyring status ──────────────────────────────────────────────────────
    match key_result {
        Ok(_) => println!("Key:          present ✓"),
        Err(_) => println!("Key:          not set"),
    }
```

Replace it with:

```rust
    // ── Search index status ─────────────────────────────────────────────────
    let search_dir = default_search_dir();
    if search_dir.join("meta.json").exists() {
        use engram_search::indexer::TantivyIndexer;
        match TantivyIndexer::open(&search_dir) {
            Ok(indexer) => {
                let count = indexer.indexed_doc_count();
                let size_mb = dir_size_bytes(&search_dir) as f64 / 1_048_576.0;
                println!(
                    "Search index: {} ({} files indexed, {:.1} MB)",
                    search_dir.display(),
                    count,
                    size_mb
                );
            }
            Err(_) => {
                println!("Search index: {} (error opening index)", search_dir.display());
            }
        }
    } else {
        println!("Search index: not built (run: engram index)");
    }

    // ── Keyring status ──────────────────────────────────────────────────────
    match key_result {
        Ok(_) => println!("Key:          present \u{2713}"),
        Err(_) => println!("Key:          not set"),
    }
```

**Step 3: Verify the binary compiles**

```bash
cd ~/workspace/ms/engram
cargo build -p engram
```

Expected: compiles with zero errors.

**Step 4: Smoke-test `engram status`**

```bash
cargo run -p engram -- status
```

Expected output (with index present):
```
─────────────────────────────────────
Vault:        /Users/ken/.lifeos/memory (247 files)
Memory store: /Users/ken/.engram/memory.db (not initialized)
Search index: /Users/ken/.engram/search (23 files indexed, 1.1 MB)
Key:          not set
```

Expected output (index not yet built):
```
─────────────────────────────────────
Vault:        /Users/ken/.lifeos/memory (247 files)
Memory store: /Users/ken/.engram/memory.db (not initialized)
Search index: not built (run: engram index)
Key:          not set
```

**Step 5: Commit**

```bash
git add crates/engram-cli/src/main.rs && \
git commit -m "feat(cli): engram status shows search index file count and size"
```

---

## Task 10: Integration test — vault index + search round-trip

**Files:**
- Create: `crates/engram-search/tests/integration_test.rs`

**Step 1: Create the integration test file**

Create `crates/engram-search/tests/integration_test.rs`:

```rust
// crates/engram-search/tests/integration_test.rs
//
// End-to-end tests: create a temp vault, index it, search it.
//
// Note: tests that use Embedder::new() will download AllMiniLML6V2 (~90 MB)
// to ~/.cache/huggingface on first run. Subsequent runs use the local cache.

use engram_core::vault::Vault;
use engram_search::{
    embedder::Embedder,
    hybrid::HybridSearch,
    indexer::{IndexStats, TantivyIndexer},
    vector::VectorIndex,
};
use tempfile::TempDir;

/// Create a TempDir vault pre-populated with the given (filename, content) pairs.
fn make_vault(files: &[(&str, &str)]) -> (TempDir, Vault) {
    let vault_dir = TempDir::new().expect("create temp vault dir");
    let vault = Vault::new(vault_dir.path());
    for (name, content) in files {
        vault.write(name, content).expect("write vault file");
    }
    (vault_dir, vault)
}

#[test]
fn test_full_index_and_search_round_trip() {
    let index_dir = TempDir::new().unwrap();
    let (_vault_dir, vault) = make_vault(&[
        ("People/Sofia.md", "Sofia is vegetarian and lives in Seattle."),
        ("People/Chris.md", "Chris follows a kosher diet."),
        ("Work/Project.md", "Q2 planning for the infrastructure project."),
        ("Work/Notes.md", "Meeting notes: discussed dietary requirements."),
        ("Personal/Goals.md", "Run a marathon and eat healthier this year."),
    ]);

    let indexer = TantivyIndexer::open(index_dir.path()).unwrap();
    let stats = indexer.index_vault(&vault).unwrap();

    assert_eq!(stats.total, 5);
    assert_eq!(stats.indexed, 5);
    assert_eq!(stats.skipped, 0);

    // Verify the indexed doc count matches
    assert_eq!(indexer.indexed_doc_count(), 5);

    // Search for a keyword present in exactly one file
    let results = indexer.search("kosher", 10).unwrap();
    assert_eq!(results.len(), 1, "only Chris.md contains 'kosher'");
    assert_eq!(results[0].path, "People/Chris.md");

    // Search for a keyword present in multiple files
    let results = indexer.search("dietary", 10).unwrap();
    assert!(results.len() >= 2, "multiple files discuss dietary topics");
}

#[test]
fn test_search_returns_empty_for_unknown_query() {
    let index_dir = TempDir::new().unwrap();
    let (_vault_dir, vault) = make_vault(&[
        ("note.md", "This is a simple note about apples and oranges."),
    ]);

    let indexer = TantivyIndexer::open(index_dir.path()).unwrap();
    indexer.index_vault(&vault).unwrap();

    let results = indexer.search("xenomorphic-plasma-flux", 10).unwrap();
    assert!(results.is_empty(), "no results expected for nonsense query");
}

#[test]
fn test_incremental_reindex_only_reindexes_changed_file() {
    let index_dir = TempDir::new().unwrap();
    let vault_dir = TempDir::new().unwrap();

    // Write 5 files
    for i in 1..=5 {
        std::fs::write(
            vault_dir.path().join(format!("note{i}.md")),
            format!("Content of note number {i}."),
        )
        .unwrap();
    }

    let vault = Vault::new(vault_dir.path());
    let indexer = TantivyIndexer::open(index_dir.path()).unwrap();

    // First pass: index all 5
    let stats1: IndexStats = indexer.index_vault(&vault).unwrap();
    assert_eq!(stats1.indexed, 5);
    assert_eq!(stats1.skipped, 0);

    // Second pass: nothing changed — all 5 skipped
    let stats2 = indexer.index_vault(&vault).unwrap();
    assert_eq!(stats2.indexed, 0);
    assert_eq!(stats2.skipped, 5);

    // Modify one file
    std::fs::write(
        vault_dir.path().join("note3.md"),
        "Completely rewritten content about Seattle.",
    )
    .unwrap();

    // Third pass: only note3.md reindexed; the other 4 skipped
    let stats3 = indexer.index_vault(&vault).unwrap();
    assert_eq!(stats3.indexed, 1, "only the changed file must be reindexed");
    assert_eq!(stats3.skipped, 4);

    // Verify the new content is searchable
    let results = indexer.search("Seattle", 5).unwrap();
    assert!(!results.is_empty());
    assert_eq!(results[0].path, "note3.md");
}

#[test]
fn test_hybrid_search_full_round_trip() {
    let index_dir = TempDir::new().unwrap();
    let db_path = index_dir.path().join("vectors.db");

    let (_vault_dir, vault) = make_vault(&[
        (
            "People/Sofia.md",
            "Sofia is vegetarian and lives in Seattle.",
        ),
        (
            "Work/Transcript.md",
            "Dietary preferences: Sofia vegetarian, Chris kosher.",
        ),
        ("Personal/Goals.md", "Run a marathon this year."),
    ]);

    let indexer = TantivyIndexer::open(index_dir.path()).unwrap();
    indexer.index_vault(&vault).unwrap();

    let vector_index = VectorIndex::open(&db_path).unwrap();
    let embedder = Embedder::new().unwrap();

    // Insert embeddings for two of the files
    let sofia_emb = embedder
        .embed("Sofia is vegetarian and lives in Seattle.")
        .unwrap();
    let transcript_emb = embedder
        .embed("Dietary preferences: Sofia vegetarian, Chris kosher.")
        .unwrap();
    vector_index.insert("People/Sofia.md", &sofia_emb).unwrap();
    vector_index
        .insert("Work/Transcript.md", &transcript_emb)
        .unwrap();

    let hybrid = HybridSearch::new(indexer, vector_index, embedder);
    let results = hybrid.search("Sofia vegetarian", 5).unwrap();

    assert!(!results.is_empty());
    // Both Sofia.md and Transcript.md discuss "Sofia vegetarian"
    let result_paths: Vec<&str> = results.iter().map(|r| r.path.as_str()).collect();
    assert!(
        result_paths.contains(&"People/Sofia.md"),
        "Sofia.md must appear in results"
    );
}
```

**Step 2: Run the integration tests**

```bash
cd ~/workspace/ms/engram
cargo test -p engram-search --test integration_test
```

Expected: `4 tests pass` (slow on first run due to model download).

**Step 3: Run the complete engram-search test suite**

```bash
cd ~/workspace/ms/engram
cargo test -p engram-search
```

Expected: all unit + integration tests pass.

**Step 4: Run the full workspace test suite**

```bash
cd ~/workspace/ms/engram
cargo test --workspace
```

Expected: all crates compile and pass tests. (Keychain tests are marked `#[ignore]` and will not run.)

**Step 5: Commit**

```bash
git add crates/engram-search/tests/integration_test.rs && \
git commit -m "test(search): integration tests — vault index + search round-trip"
```

---

## Completion Checklist

When all 10 tasks are done, verify the following:

```bash
# All tests pass
cargo test --workspace

# CLI help includes the new subcommands
cargo run -p engram -- --help

# Index the vault
cargo run -p engram -- index

# Run a hybrid search
cargo run -p engram -- search "your query here"

# Status shows search index stats
cargo run -p engram -- status
```

Expected final `engram status` output:
```
─────────────────────────────────────
Vault:        /Users/ken/.lifeos/memory (247 files)
Memory store: /Users/ken/.engram/memory.db (not initialized)
Search index: /Users/ken/.engram/search (247 files indexed, 1.2 MB)
Key:          not set
```

Expected final `engram search` output:
```
Results for "Sofia vegetarian" (hybrid, 2 results)
─────────────────────────────────────────────────
People/Sofia.md (score: 0.03)
  ...Sofia is vegetarian and lives in Seattle...

Work/Transcripts/2026-03-09.md (score: 0.02)
  ...dietary preferences: Sofia vegetarian, Chris kosher...

```
