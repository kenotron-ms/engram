// observe.rs — types, error, transcript parser, LLM fact extraction

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

/// System prompt instructing the LLM to extract atomic facts as a JSON array.
pub const DEFAULT_SYSTEM_PROMPT: &str = r#"You are a fact-extraction assistant. Given a conversation, extract all atomic facts mentioned. Return ONLY a JSON array (no markdown, no explanation) where each element has exactly these fields:
- "entity": the subject of the fact (a person, place, thing, or concept)
- "attribute": what property or characteristic is described
- "value": the value of that attribute
- "source": always "user" for facts stated by the user, "assistant" for facts stated by the assistant

Example output:
[{"entity":"Sofia","attribute":"diet","value":"vegetarian","source":"user"},{"entity":"Paris","attribute":"country","value":"France","source":"user"}]

If there are no facts, return an empty array: []"#;

/// Parse an Anthropic Messages API response and extract the facts.
///
/// Expects the response to contain `content[0].text` with a JSON array of facts.
/// Markdown code fences (```json ... ```) are stripped defensively before parsing.
pub fn parse_facts_response(json: &serde_json::Value) -> Result<Vec<ExtractedFact>, ObserveError> {
    // Extract content[0].text from the Anthropic response
    let text = json
        .get("content")
        .and_then(|c| c.as_array())
        .and_then(|arr| arr.first())
        .and_then(|first| first.get("text"))
        .and_then(|t| t.as_str())
        .ok_or_else(|| ObserveError::Api("Missing content[0].text in API response".to_string()))?;

    // Strip markdown fences defensively
    let cleaned = strip_markdown_fences(text);

    // Deserialize into Vec<ExtractedFact>
    let facts: Vec<ExtractedFact> = serde_json::from_str(cleaned.trim())?;
    Ok(facts)
}

/// Strip leading/trailing markdown code fences from a string.
fn strip_markdown_fences(text: &str) -> &str {
    let trimmed = text.trim();
    // Remove leading ```json or ``` fence
    let after_open = if let Some(rest) = trimmed.strip_prefix("```json") {
        rest
    } else if let Some(rest) = trimmed.strip_prefix("```") {
        rest
    } else {
        return trimmed;
    };
    // Remove trailing ``` fence
    let after_newline = after_open.trim_start_matches('\n');
    if let Some(content) = after_newline.strip_suffix("```") {
        content.trim()
    } else {
        after_newline.trim()
    }
}

/// Format transcript turns as a human-readable conversation string.
fn format_turns_as_text(turns: &[TranscriptTurn]) -> String {
    turns
        .iter()
        .map(|t| format!("{}: {}", t.role, t.content))
        .collect::<Vec<_>>()
        .join("\n")
}

/// Call the Anthropic Messages API to extract facts from transcript turns.
///
/// POSTs to `https://api.anthropic.com/v1/messages` using model `claude-haiku-4-5`,
/// `max_tokens` 2048, and the required headers. Returns the parsed facts.
pub fn extract_facts(
    turns: &[TranscriptTurn],
    api_key: &str,
) -> Result<Vec<ExtractedFact>, ObserveError> {
    let conversation_text = format_turns_as_text(turns);

    let body = serde_json::json!({
        "model": "claude-haiku-4-5",
        "max_tokens": 2048,
        "system": DEFAULT_SYSTEM_PROMPT,
        "messages": [
            {
                "role": "user",
                "content": conversation_text
            }
        ]
    });

    let client = reqwest::blocking::Client::new();
    let response = client
        .post("https://api.anthropic.com/v1/messages")
        .header("x-api-key", api_key)
        .header("anthropic-version", "2023-06-01")
        .header("content-type", "application/json")
        .json(&body)
        .send()
        .map_err(|e| ObserveError::Api(format!("HTTP request failed: {e}")))?;

    let status = response.status();
    let response_json: serde_json::Value = response
        .json()
        .map_err(|e| ObserveError::Api(format!("Failed to parse API response: {e}")))?;

    if !status.is_success() {
        let msg = response_json
            .get("error")
            .and_then(|e| e.get("message"))
            .and_then(|m| m.as_str())
            .unwrap_or("Unknown API error");
        return Err(ObserveError::Api(format!("API returned {status}: {msg}")));
    }

    parse_facts_response(&response_json)
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

    // ── parse_facts_response tests ─────────────────────────────────────────

    #[test]
    fn test_parse_facts_response_from_fixture() {
        // Anthropic-style response with 2 facts in content[0].text
        let fixture: serde_json::Value = serde_json::json!({
            "content": [
                {
                    "type": "text",
                    "text": "[{\"entity\":\"Sofia\",\"attribute\":\"diet\",\"value\":\"vegetarian\",\"source\":\"user\"},{\"entity\":\"Alice\",\"attribute\":\"age\",\"value\":\"30\",\"source\":\"user\"}]"
                }
            ]
        });
        let facts = parse_facts_response(&fixture).unwrap();
        assert_eq!(facts.len(), 2);
        assert_eq!(facts[0].entity, "Sofia");
        assert_eq!(facts[0].attribute, "diet");
        assert_eq!(facts[0].value, "vegetarian");
        assert_eq!(facts[1].entity, "Alice");
        assert_eq!(facts[1].attribute, "age");
        assert_eq!(facts[1].value, "30");
    }

    #[test]
    fn test_parse_facts_response_empty_array() {
        let fixture: serde_json::Value = serde_json::json!({
            "content": [
                {
                    "type": "text",
                    "text": "[]"
                }
            ]
        });
        let facts = parse_facts_response(&fixture).unwrap();
        assert!(facts.is_empty());
    }

    #[test]
    fn test_parse_facts_response_missing_content_is_error() {
        let fixture: serde_json::Value = serde_json::json!({
            "model": "claude-haiku-4-5"
        });
        let result = parse_facts_response(&fixture);
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_facts_response_strips_json_markdown_fence() {
        // Covers the ```json … ``` code path in strip_markdown_fences
        let fixture: serde_json::Value = serde_json::json!({
            "content": [{
                "type": "text",
                "text": "```json\n[{\"entity\":\"Sofia\",\"attribute\":\"diet\",\"value\":\"vegetarian\",\"source\":\"user\"}]\n```"
            }]
        });
        let facts = parse_facts_response(&fixture).unwrap();
        assert_eq!(facts.len(), 1);
        assert_eq!(facts[0].entity, "Sofia");
        assert_eq!(facts[0].attribute, "diet");
        assert_eq!(facts[0].value, "vegetarian");
    }

    #[test]
    fn test_parse_facts_response_strips_bare_markdown_fence() {
        // Covers the bare ``` … ``` code path in strip_markdown_fences (no "json" qualifier)
        let fixture: serde_json::Value = serde_json::json!({
            "content": [{
                "type": "text",
                "text": "```\n[{\"entity\":\"Paris\",\"attribute\":\"country\",\"value\":\"France\",\"source\":\"user\"}]\n```"
            }]
        });
        let facts = parse_facts_response(&fixture).unwrap();
        assert_eq!(facts.len(), 1);
        assert_eq!(facts[0].entity, "Paris");
        assert_eq!(facts[0].attribute, "country");
        assert_eq!(facts[0].value, "France");
    }

    #[test]
    #[ignore]
    fn test_extract_facts_real_api_call() {
        let api_key = std::env::var("ANTHROPIC_API_KEY")
            .expect("ANTHROPIC_API_KEY must be set for this test");
        let turns = vec![TranscriptTurn {
            role: "user".to_string(),
            content: "Sofia is a vegetarian who loves hiking.".to_string(),
            timestamp: None,
        }];
        let facts = extract_facts(&turns, &api_key).unwrap();
        assert!(!facts.is_empty(), "Expected at least one fact");
        assert!(
            facts.iter().any(|f| f.entity.to_lowercase().contains("sofia")),
            "Expected a fact about Sofia"
        );
    }

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
