// embedder: fastembed vector embedding generation

use fastembed::{EmbeddingModel, InitOptions, TextEmbedding};

use crate::SearchError;

/// Wraps a fastembed TextEmbedding model for producing dense vector embeddings.
///
/// Uses AllMiniLML6V2, which produces 384-dimensional float vectors.
/// Model weights (~90 MB) are downloaded on first use and cached locally thereafter.
pub struct Embedder {
    model: TextEmbedding,
}

impl Embedder {
    /// Load the AllMiniLML6V2 embedding model.
    ///
    /// Downloads model weights on first call; uses the local cache thereafter.
    pub fn new() -> Result<Self, SearchError> {
        let model = TextEmbedding::try_new(InitOptions::new(EmbeddingModel::AllMiniLML6V2))
            .map_err(|e| SearchError::Embed(e.to_string()))?;
        Ok(Self { model })
    }

    /// Embed a single text string, returning a 384-dimensional vector.
    pub fn embed(&self, text: &str) -> Result<Vec<f32>, SearchError> {
        let mut embeddings = self
            .model
            .embed(vec![text], None)
            .map_err(|e| SearchError::Embed(e.to_string()))?;
        embeddings
            .pop()
            .ok_or_else(|| SearchError::Embed("no embedding returned".to_string()))
    }

    /// Embed a batch of text strings, returning one 384-dimensional vector per input.
    pub fn embed_batch(&self, texts: &[&str]) -> Result<Vec<Vec<f32>>, SearchError> {
        self.model
            .embed(texts.to_vec(), None)
            .map_err(|e| SearchError::Embed(e.to_string()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_embedder() -> Embedder {
        Embedder::new().expect("Embedder should load successfully")
    }

    #[test]
    fn test_embed_produces_384_dimensions() {
        let embedder = make_embedder();
        let embedding = embedder.embed("hello world").expect("embed should succeed");
        assert_eq!(embedding.len(), 384);
    }

    #[test]
    fn test_same_text_produces_same_vector() {
        let embedder = make_embedder();
        let a = embedder
            .embed("deterministic test text")
            .expect("embed should succeed");
        let b = embedder
            .embed("deterministic test text")
            .expect("embed should succeed");
        assert_eq!(a, b);
    }

    #[test]
    fn test_different_texts_produce_different_vectors() {
        let embedder = make_embedder();
        let a = embedder.embed("hello world").expect("embed should succeed");
        let b = embedder
            .embed("goodbye world")
            .expect("embed should succeed");
        assert_ne!(a, b);
    }

    #[test]
    fn test_embed_batch_returns_one_vector_per_input() {
        let embedder = make_embedder();
        let texts = ["first text", "second text", "third text"];
        let embeddings = embedder
            .embed_batch(&texts)
            .expect("embed_batch should succeed");
        assert_eq!(embeddings.len(), 3);
        for emb in &embeddings {
            assert_eq!(emb.len(), 384);
        }
    }
}
