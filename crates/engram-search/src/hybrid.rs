// hybrid: combines full-text and vector search results via Reciprocal Rank Fusion (RRF)

use std::collections::HashMap;

use crate::embedder::Embedder;
use crate::indexer::TantivyIndexer;
use crate::vector::VectorIndex;
use crate::{SearchError, SearchResult, SearchSource};

/// The k constant for Reciprocal Rank Fusion (Cormack, Clarke & Buettcher 2009).
const RRF_K: f32 = 60.0;

/// Compute the RRF score contribution of a single result at `rank` (0-indexed) with constant `k`.
///
/// Formula: 1 / (k + rank + 1)
/// For rank=0 and k=60: 1/61 ≈ 0.01639
pub fn rrf_score(rank: usize, k: f32) -> f32 {
    1.0 / (k + rank as f32 + 1.0)
}

/// Hybrid search combining full-text (TantivyIndexer) and vector (VectorIndex) search via RRF.
pub struct HybridSearch {
    indexer: TantivyIndexer,
    vector_index: VectorIndex,
    embedder: Embedder,
}

impl HybridSearch {
    /// Create a new HybridSearch from its three component sub-systems.
    pub fn new(indexer: TantivyIndexer, vector_index: VectorIndex, embedder: Embedder) -> Self {
        Self {
            indexer,
            vector_index,
            embedder,
        }
    }

    /// Run hybrid search for `query`, returning up to `limit` results.
    ///
    /// Uses Reciprocal Rank Fusion (k=60) to merge full-text and vector results.
    /// Returns results with `source=Hybrid` and snippet from full-text results if available.
    pub fn search(&self, query: &str, limit: usize) -> Result<Vec<SearchResult>, SearchError> {
        let candidate_pool = limit * 3;

        // Full-text search
        let ft_results = self.indexer.search(query, candidate_pool)?;

        // Vector search: embed the query then run KNN
        let embedding = self.embedder.embed(query)?;
        let vec_results = self.vector_index.knn_search(&embedding, candidate_pool)?;

        // RRF merge: accumulate 1/(k + rank + 1) for each result in each list
        let mut scores: HashMap<String, f32> = HashMap::new();

        for (rank, result) in ft_results.iter().enumerate() {
            *scores.entry(result.path.clone()).or_insert(0.0) += rrf_score(rank, RRF_K);
        }

        for (rank, (memory_id, _distance)) in vec_results.iter().enumerate() {
            *scores.entry(memory_id.clone()).or_insert(0.0) += rrf_score(rank, RRF_K);
        }

        // Build snippet map from full-text results (path → snippet)
        let snippet_map: HashMap<String, String> = ft_results
            .into_iter()
            .map(|r| (r.path, r.snippet))
            .collect();

        // Sort by combined RRF score descending
        let mut scored: Vec<(String, f32)> = scores.into_iter().collect();
        scored.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

        // Return top limit results with source=Hybrid
        let results = scored
            .into_iter()
            .take(limit)
            .map(|(path, score)| SearchResult {
                snippet: snippet_map.get(&path).cloned().unwrap_or_default(),
                path,
                score,
                source: SearchSource::Hybrid,
            })
            .collect();

        Ok(results)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::embedder::Embedder;
    use crate::indexer::TantivyIndexer;
    use crate::vector::VectorIndex;
    use engram_core::vault::Vault;
    use tempfile::TempDir;

    /// Helper: build a HybridSearch with "Sofia.md" in both the full-text and vector indices.
    fn make_hybrid_with_sofia() -> (HybridSearch, TempDir, TempDir, TempDir) {
        let index_dir = TempDir::new().unwrap();
        let vault_dir = TempDir::new().unwrap();
        let vec_db_dir = TempDir::new().unwrap();

        // Write Sofia.md to the vault and index it in full-text
        let vault = Vault::new(vault_dir.path());
        vault
            .write("Sofia.md", "Sofia is a city in Bulgaria and the capital.")
            .unwrap();

        let mut indexer = TantivyIndexer::open(index_dir.path()).unwrap();
        indexer.index_vault(&vault).unwrap();

        // Open the vector index and insert Sofia.md's embedding
        let vec_db_path = vec_db_dir.path().join("vectors.db");
        let vector_index = VectorIndex::open(&vec_db_path).unwrap();

        let embedder = Embedder::new().unwrap();
        let sofia_embedding = embedder
            .embed("Sofia is a city in Bulgaria and the capital.")
            .unwrap();
        vector_index.insert("Sofia.md", &sofia_embedding).unwrap();

        let hybrid = HybridSearch::new(indexer, vector_index, embedder);
        (hybrid, index_dir, vault_dir, vec_db_dir)
    }

    /// Hybrid search must return at least one result when matching documents exist in both indices.
    #[test]
    fn test_hybrid_search_returns_results() {
        let (hybrid, _i, _v, _d) = make_hybrid_with_sofia();

        let results = hybrid.search("Sofia Bulgaria", 5).unwrap();
        assert!(
            !results.is_empty(),
            "hybrid search should return at least one result"
        );
    }

    /// A document that appears in both full-text and vector results should be boosted by RRF
    /// and rank above a document that only appears in one list.
    #[test]
    fn test_rrf_boosts_item_appearing_in_both_lists() {
        let index_dir = TempDir::new().unwrap();
        let vault_dir = TempDir::new().unwrap();
        let vec_db_dir = TempDir::new().unwrap();

        let vault = Vault::new(vault_dir.path());
        // Sofia.md: will be in both FT and vector results
        vault
            .write("Sofia.md", "Sofia is a city in Bulgaria and the capital.")
            .unwrap();
        // other.md: will be in FT results but NOT in the vector index
        vault
            .write(
                "other.md",
                "Other content about Sofia and cities in Europe.",
            )
            .unwrap();

        let mut indexer = TantivyIndexer::open(index_dir.path()).unwrap();
        indexer.index_vault(&vault).unwrap();

        let vec_db_path = vec_db_dir.path().join("vectors.db");
        let vector_index = VectorIndex::open(&vec_db_path).unwrap();

        let embedder = Embedder::new().unwrap();
        // Only insert Sofia.md into the vector index
        let sofia_embedding = embedder
            .embed("Sofia is a city in Bulgaria and the capital.")
            .unwrap();
        vector_index.insert("Sofia.md", &sofia_embedding).unwrap();

        let hybrid = HybridSearch::new(indexer, vector_index, embedder);
        let results = hybrid.search("Sofia Bulgaria capital", 5).unwrap();

        assert!(!results.is_empty(), "should return at least one result");
        assert_eq!(
            results[0].path, "Sofia.md",
            "Sofia.md should rank first because it appears in both FT and vector results (RRF boost)"
        );
    }

    /// Verify the RRF formula with k=60:
    ///   rank 0 in one list  → 1/(60+0+1) = 1/61 ≈ 0.01639
    ///   rank 0 in both lists → 2/61 ≈ 0.03279
    #[test]
    fn test_rrf_score_formula_k60() {
        // rank 0 in one list: 1 / (60 + 0 + 1) = 1/61
        let score_one_list = rrf_score(0, 60.0);
        let expected_one = 1.0_f32 / 61.0;
        assert!(
            (score_one_list - expected_one).abs() < 1e-6,
            "rank 0 in one list should be 1/61 ≈ 0.01639, got {score_one_list}"
        );

        // rank 0 in both lists: 2 * (1/61) = 2/61
        let score_both_lists = rrf_score(0, 60.0) + rrf_score(0, 60.0);
        let expected_both = 2.0_f32 / 61.0;
        assert!(
            (score_both_lists - expected_both).abs() < 1e-6,
            "rank 0 in both lists should be 2/61 ≈ 0.03279, got {score_both_lists}"
        );
    }
}
