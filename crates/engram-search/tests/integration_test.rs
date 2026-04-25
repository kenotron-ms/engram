// Integration tests: vault index + search round-trip including hybrid search

use engram_core::vault::Vault;
use engram_search::embedder::Embedder;
use engram_search::hybrid::HybridSearch;
use engram_search::indexer::TantivyIndexer;
use engram_search::vector::VectorIndex;
use tempfile::TempDir;

/// Maximum number of results to request from any search call in these tests.
const MAX_RESULTS: usize = 10;

/// Create a temporary vault populated with the given (relative_path, content) pairs.
///
/// Returns `(TempDir, Vault)` — the caller MUST bind the `TempDir` to keep the directory alive.
fn make_vault(files: &[(&str, &str)]) -> (TempDir, Vault) {
    let dir = TempDir::new().expect("failed to create temp dir");
    let vault = Vault::new(dir.path());
    for (path, content) in files {
        vault
            .write(path, content)
            .expect("failed to write vault file");
    }
    (dir, vault)
}

/// Full round-trip: write 5 files, index the vault, check stats, then run BM25 searches.
#[test]
fn test_full_index_and_search_round_trip() {
    let (_vault_dir, vault) = make_vault(&[
        (
            "People/Sofia.md",
            "Sofia is a vegetarian chef who loves Mediterranean cuisine and fresh produce.",
        ),
        (
            "People/Chris.md",
            "Chris follows a strict kosher dietary lifestyle. Dietary restrictions are central \
             to his daily routine — he never mixes dairy and meat.",
        ),
        (
            "Work/Project.md",
            "The project involves building a next-generation full-text search engine for \
             structured documents.",
        ),
        (
            "Work/Notes.md",
            "Meeting notes: we need to discuss dietary requirements for the team catering \
             order placed next month.",
        ),
        (
            "Personal/Goals.md",
            "Personal goals for the year: run a half-marathon, learn to cook Thai food, \
             and read twenty books.",
        ),
    ]);

    let index_dir = TempDir::new().expect("failed to create index dir");
    let mut indexer = TantivyIndexer::open(index_dir.path()).expect("failed to open indexer");

    // --- Index ---
    let stats = indexer.index_vault(&vault).expect("index_vault failed");

    assert_eq!(stats.total, 5, "total should be 5");
    assert_eq!(stats.indexed, 5, "indexed should be 5 on first run");
    assert_eq!(stats.skipped, 0, "skipped should be 0 on first run");
    assert_eq!(
        indexer.indexed_doc_count(),
        5,
        "indexed_doc_count should be 5"
    );

    // --- Search 'kosher': only Chris.md contains that word ---
    let results = indexer
        .search("kosher", MAX_RESULTS)
        .expect("search failed");
    assert_eq!(
        results.len(),
        1,
        "search 'kosher' should return exactly 1 result"
    );
    assert!(
        results[0].path.contains("Chris.md"),
        "the kosher result should be People/Chris.md, got: {}",
        results[0].path
    );

    // --- Search 'dietary': Chris.md and Work/Notes.md both contain the word ---
    let results = indexer
        .search("dietary", MAX_RESULTS)
        .expect("search failed");
    assert!(
        results.len() >= 2,
        "search 'dietary' should return at least 2 results, got {}",
        results.len()
    );
}

/// Searching an indexed vault with a term that appears in no document returns an empty list.
#[test]
fn test_search_returns_empty_for_unknown_query() {
    let (_vault_dir, vault) = make_vault(&[(
        "note.md",
        "This is a simple note about everyday life and routine tasks.",
    )]);

    let index_dir = TempDir::new().expect("failed to create index dir");
    let mut indexer = TantivyIndexer::open(index_dir.path()).expect("failed to open indexer");
    indexer.index_vault(&vault).expect("index_vault failed");

    // A term that does not appear anywhere in the vault
    let results = indexer
        .search("zxqwkjhflasdkjmnonexistent", MAX_RESULTS)
        .expect("search failed");
    assert!(
        results.is_empty(),
        "a nonsense query should return no results, got {}",
        results.len()
    );
}

/// Incremental re-index: only a file whose content changed is re-indexed; the rest are skipped.
#[test]
fn test_incremental_reindex_only_reindexes_changed_file() {
    let (_vault_dir, vault) = make_vault(&[
        ("note1.md", "Note one is all about apples and orchards."),
        (
            "note2.md",
            "Note two discusses bananas and tropical fruit farming.",
        ),
        (
            "note3.md",
            "Note three is about cherries and cherry picking season.",
        ),
        (
            "note4.md",
            "Note four covers dates and arid-climate agriculture.",
        ),
        (
            "note5.md",
            "Note five explores elderberries and their medicinal uses.",
        ),
    ]);

    let index_dir = TempDir::new().expect("failed to create index dir");
    let mut indexer = TantivyIndexer::open(index_dir.path()).expect("failed to open indexer");

    // --- First pass: all 5 files are new, all must be indexed ---
    let stats = indexer
        .index_vault(&vault)
        .expect("first index_vault failed");
    assert_eq!(stats.total, 5, "first pass: total should be 5");
    assert_eq!(stats.indexed, 5, "first pass: all 5 should be indexed");
    assert_eq!(stats.skipped, 0, "first pass: none should be skipped");

    // --- Second pass: nothing changed, all 5 should be skipped ---
    let stats = indexer
        .index_vault(&vault)
        .expect("second index_vault failed");
    assert_eq!(stats.total, 5, "second pass: total should be 5");
    assert_eq!(stats.indexed, 0, "second pass: none should be indexed");
    assert_eq!(stats.skipped, 5, "second pass: all 5 should be skipped");

    // --- Modify note3.md ---
    vault
        .write(
            "note3.md",
            "Note three has been updated: it now discusses mangoes and other tropical fruits.",
        )
        .expect("failed to update note3.md");

    // --- Third pass: only note3.md has changed, index 1 skip 4 ---
    let stats = indexer
        .index_vault(&vault)
        .expect("third index_vault failed");
    assert_eq!(stats.total, 5, "third pass: total should be 5");
    assert_eq!(
        stats.indexed, 1,
        "third pass: only 1 file should be re-indexed"
    );
    assert_eq!(stats.skipped, 4, "third pass: 4 files should be skipped");

    // --- New content must be searchable ---
    let results = indexer
        .search("mangoes", MAX_RESULTS)
        .expect("search failed");
    assert!(
        !results.is_empty(),
        "new content in note3.md should be searchable after re-index"
    );
    assert!(
        results[0].path.contains("note3.md"),
        "mangoes should be found in note3.md, got: {}",
        results[0].path
    );
}

/// Full hybrid-search round-trip: vault → full-text index → vector index → HybridSearch.
///
/// Embeddings are inserted only for two of the three files.  A hybrid query for
/// "Sofia vegetarian" must return results that include People/Sofia.md.
#[test]
fn test_hybrid_search_full_round_trip() {
    // Define content once so vault files and embeddings stay in sync.
    let sofia_text = "Sofia is a dedicated vegetarian who enjoys Mediterranean food and follows a \
                      plant-based diet for both ethical and health reasons.";
    let transcript_text = "Transcript of the weekly engineering meeting discussing project \
                           milestones, sprint velocity, and upcoming deadlines.";

    let (_vault_dir, vault) = make_vault(&[
        ("People/Sofia.md", sofia_text),
        ("Meeting/Transcript.md", transcript_text),
        (
            "Work/Notes.md",
            "General work notes covering product roadmap, stakeholder feedback, and \
             quarterly planning objectives.",
        ),
    ]);

    // --- Full-text index ---
    let index_dir = TempDir::new().expect("failed to create index dir");
    let mut indexer = TantivyIndexer::open(index_dir.path()).expect("failed to open indexer");
    indexer.index_vault(&vault).expect("index_vault failed");

    // --- Vector index ---
    let vec_db_dir = TempDir::new().expect("failed to create vector db dir");
    let vector_index = VectorIndex::open(&vec_db_dir.path().join("vectors.db"))
        .expect("failed to open vector index");

    let embedder = Embedder::new().expect("failed to initialise embedder");

    // Insert embeddings for Sofia.md and Transcript.md (using their vault-relative paths as IDs)
    let sofia_embedding = embedder.embed(sofia_text).expect("embed Sofia failed");
    vector_index
        .insert("People/Sofia.md", &sofia_embedding)
        .expect("insert Sofia embedding failed");

    let transcript_embedding = embedder
        .embed(transcript_text)
        .expect("embed Transcript failed");
    vector_index
        .insert("Meeting/Transcript.md", &transcript_embedding)
        .expect("insert Transcript embedding failed");

    // --- Hybrid search ---
    let hybrid = HybridSearch::new(indexer, vector_index, embedder);
    let results = hybrid
        .search("Sofia vegetarian", MAX_RESULTS)
        .expect("hybrid search failed");

    assert!(
        !results.is_empty(),
        "hybrid search for 'Sofia vegetarian' should return at least one result"
    );

    let paths: Vec<&str> = results.iter().map(|r| r.path.as_str()).collect();
    assert!(
        paths.iter().any(|p| p.contains("Sofia.md")),
        "results should contain People/Sofia.md; got: {:?}",
        paths
    );
}
