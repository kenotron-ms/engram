// observe.rs — types, error, transcript parser

use serde::Deserialize;
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::Path;
use thiserror::Error;

/// Errors that can occur during observation / fact-extraction operations.
#[derive(Debug, Error)]
pub enum ObserveError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("JSON parse error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("API error: {0}")]
    Api(String),

    #[error("Missing API key")]
    MissingApiKey,
}

/// A single turn in a conversation transcript.
#[derive(Debug, Deserialize)]
pub struct TranscriptTurn {
    pub role: String,
    pub content: String,
    pub timestamp: Option<i64>,
}

/// A fact extracted from a transcript turn.
#[derive(Debug, Deserialize)]
pub struct ExtractedFact {
    pub entity: String,
    pub attribute: String,
    pub value: String,
    pub source: String,
}

/// Summary statistics for an observe run.
#[derive(Debug)]
pub struct ObserveStats {
    pub facts_extracted: usize,
    pub facts_written: usize,
    pub session_path: String,
}

/// Read a JSONL transcript file and return the turns in order.
///
/// Blank lines and lines that are not valid JSON objects with at least
/// a `role` and `content` string field are silently skipped.
pub fn parse_transcript(path: &Path) -> Result<Vec<TranscriptTurn>, ObserveError> {
    let file = File::open(path)?;
    let reader = BufReader::new(file);
    let mut turns = Vec::new();

    for line in reader.lines() {
        let line = line?;
        let trimmed = line.trim();

        // Skip blank lines.
        if trimmed.is_empty() {
            continue;
        }

        // Skip malformed (non-JSON) lines.
        let value: serde_json::Value = match serde_json::from_str(trimmed) {
            Ok(v) => v,
            Err(_) => continue,
        };

        // Each line must be a JSON object with role and content string fields.
        let role = match value.get("role").and_then(|v| v.as_str()) {
            Some(r) => r.to_string(),
            None => continue,
        };
        let content = match value.get("content").and_then(|v| v.as_str()) {
            Some(c) => c.to_string(),
            None => continue,
        };
        let timestamp = value.get("timestamp").and_then(|v| v.as_i64());

        turns.push(TranscriptTurn {
            role,
            content,
            timestamp,
        });
    }

    Ok(turns)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    #[test]
    fn test_parse_transcript_three_turns() {
        let mut file = NamedTempFile::new().unwrap();
        writeln!(
            file,
            r#"{{"role":"user","content":"Hello","timestamp":1000}}"#
        )
        .unwrap();
        writeln!(
            file,
            r#"{{"role":"assistant","content":"Hi there","timestamp":2000}}"#
        )
        .unwrap();
        writeln!(
            file,
            r#"{{"role":"user","content":"How are you?","timestamp":3000}}"#
        )
        .unwrap();

        let turns = parse_transcript(file.path()).unwrap();
        assert_eq!(turns.len(), 3);

        assert_eq!(turns[0].role, "user");
        assert_eq!(turns[0].content, "Hello");
        assert_eq!(turns[0].timestamp, Some(1000));

        assert_eq!(turns[1].role, "assistant");
        assert_eq!(turns[1].content, "Hi there");
        assert_eq!(turns[1].timestamp, Some(2000));

        assert_eq!(turns[2].role, "user");
        assert_eq!(turns[2].content, "How are you?");
        assert_eq!(turns[2].timestamp, Some(3000));
    }

    #[test]
    fn test_parse_transcript_skips_blank_lines() {
        let mut file = NamedTempFile::new().unwrap();
        writeln!(
            file,
            r#"{{"role":"user","content":"First","timestamp":1}}"#
        )
        .unwrap();
        writeln!(file, "").unwrap(); // blank line
        writeln!(
            file,
            r#"{{"role":"assistant","content":"Second","timestamp":2}}"#
        )
        .unwrap();

        let turns = parse_transcript(file.path()).unwrap();
        assert_eq!(turns.len(), 2);
        assert_eq!(turns[0].content, "First");
        assert_eq!(turns[1].content, "Second");
    }

    #[test]
    fn test_parse_transcript_skips_malformed_lines() {
        let mut file = NamedTempFile::new().unwrap();
        writeln!(
            file,
            r#"{{"role":"user","content":"Valid","timestamp":1}}"#
        )
        .unwrap();
        writeln!(file, "this is not json at all").unwrap(); // malformed
        writeln!(
            file,
            r#"{{"role":"assistant","content":"Also valid","timestamp":2}}"#
        )
        .unwrap();

        let turns = parse_transcript(file.path()).unwrap();
        assert_eq!(turns.len(), 2);
        assert_eq!(turns[0].content, "Valid");
        assert_eq!(turns[1].content, "Also valid");
    }
}
