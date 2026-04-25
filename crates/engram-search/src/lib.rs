// engram-search: full-text and vector search for engram

    pub mod embedder;
    pub mod hybrid;
    pub mod indexer;
    pub mod vector;

    use thiserror::Error;

    #[derive(Debug, Error)]
    pub enum SearchError {
        #[error("index error: {0}")]
        Index(String),
        #[error("embed error: {0}")]
        Embed(String),
        #[error("database error: {0}")]
        Db(String),
        #[error("io error: {0}")]
        Io(String),
    }

    #[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
    pub enum SearchSource {
        FullText,
        Vector,
        Hybrid,
    }

    #[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
    pub struct SearchResult {
        pub path: String,
        pub snippet: String,
        pub score: f32,
        pub source: SearchSource,
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        #[test]
        fn search_error_variants_exist() {
            let _index_err = SearchError::Index("test".to_string());
            let _embed_err = SearchError::Embed("test".to_string());
            let _db_err = SearchError::Db("test".to_string());
            let _io_err = SearchError::Io("test".to_string());
        }

        #[test]
        fn search_source_variants_exist() {
            let _full_text = SearchSource::FullText;
            let _vector = SearchSource::Vector;
            let _hybrid = SearchSource::Hybrid;
        }

        #[test]
        fn search_result_has_expected_fields() {
            let result = SearchResult {
                path: "/path/to/note.md".to_string(),
                snippet: "some text".to_string(),
                score: 0.95,
                source: SearchSource::Hybrid,
            };
            assert_eq!(result.path, "/path/to/note.md");
            assert_eq!(result.snippet, "some text");
            assert!((result.score - 0.95).abs() < f32::EPSILON);
        }
    }
    