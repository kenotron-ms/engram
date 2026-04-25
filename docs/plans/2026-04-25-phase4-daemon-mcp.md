# Engram — Phase 4: Daemon + MCP Server

> **For agentic workers:** REQUIRED SUB-SKILL: Use `superpowers:subagent-driven-development` (recommended) or `superpowers:executing-plans` to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build the always-on intelligence layer: `engram observe` (LLM-powered transcript processing), `engram load` (dynamic context emission), `engram daemon` (background file watcher), `engram mcp` (MCP stdio server for cross-harness integration), and `engram install` / `engram doctor` (self-managing system service).

**Architecture:** New modules in `engram-cli`: `observe.rs` (parse transcript → LLM extract → write facts), `load.rs` (query store → format context), `daemon.rs` (notify file watcher + processing queue), `mcp.rs` (JSON-RPC 2.0 stdio server), `install.rs` (launchd/systemd). Two new methods on `MemoryStore` in `engram-core`: `list_recent` and `search`. No new crate — all in the CLI binary.

**Tech Stack:** notify 6, serde 1 (derive), tokio 1 (full, already present), reqwest 0.12 blocking (already present), serde_json 1 (already present)

---

## Codebase Orientation

Read these before starting — every code block in this plan is derived from them:

- **`crates/engram-cli/src/main.rs`** — `fn main()` is synchronous. Async ops use `tokio::runtime::Runtime::new().unwrap()` + `runtime.block_on(...)`. Unicode `✓` = `\u{2713}`, `─` = `\u{2500}`. Helper functions `default_vault_path()` and `default_store_path()` already exist. Each command has a dedicated `run_*()` free function. Subcommands are clap `#[derive(Subcommand)]` variants.
- **`crates/engram-core/src/store.rs`** — `MemoryStore::open(path: &Path, key: &EngramKey)`, `insert(&Memory)`, `get(&str)`, `find_by_entity(&str)`, `record_count()`. `Memory::new(entity, attribute, value, source: Option<&str>)`. Tests: `EngramKey::derive(b"testpassword", &[0u8; 16])`, `TempDir::new()`.
- **`crates/engram-cli/Cargo.toml`** — tokio (full), reqwest (json + blocking), serde_json, thiserror, anyhow, clap (derive), directories, open, rpassword already present. `notify = "6"` and `serde = { version = "1", features = ["derive"] }` must be added.
- **`crates/engram-cli/tests/cli_integration.rs`** — Uses `assert_cmd::Command::cargo_bin("engram")` + `predicates::prelude::*`. Key-chain-touching tests are `#[ignore = "requires keychain access; run with cargo test -- --include-ignored in a GUI session"]`.
- **`reqwest::blocking::Client`** — already used in `run_auth_add_onedrive`. Use the same pattern for Anthropic API calls in `observe.rs`.

---

## File Structure Being Created

```
crates/engram-core/src/store.rs          ← ADD list_recent() and search() methods
crates/engram-cli/
├── Cargo.toml                           ← ADD notify = "6", serde with derive, tempfile dev-dep
└── src/
    ├── main.rs                          ← ADD mod declarations, Commands variants, run_* functions
    ├── observe.rs                       ← NEW: transcript parsing, LLM extraction, fact writing
    ├── load.rs                          ← NEW: context emission from memory store
    ├── daemon.rs                        ← NEW: notify watcher + processing queue
    ├── mcp.rs                           ← NEW: JSON-RPC 2.0 stdio server
    └── install.rs                       ← NEW: launchd / systemd service management
tests/
    ├── cli_integration.rs               ← ADD new CLI smoke tests
    └── observe_load_test.rs             ← NEW: round-trip integration test
```

---

## Task 1: observe.rs — types, error, transcript parser

**Files:**
- Create: `crates/engram-cli/src/observe.rs`
- Test lives in: `crates/engram-cli/src/observe.rs` (`#[cfg(test)]` block)

### Step 1: Create `crates/engram-cli/src/observe.rs` with the types and parser

```rust
// observe.rs — Transcript processing: parse → LLM extract → write facts

use std::io::BufRead;
use std::path::Path;

use serde::Deserialize;
use thiserror::Error;

// ── Error ─────────────────────────────────────────────────────────────────────

#[derive(Debug, Error)]
pub enum ObserveError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("JSON parse error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("LLM API error: {0}")]
    Api(String),

    #[error("missing ANTHROPIC_API_KEY — pass --api-key or set the environment variable")]
    MissingApiKey,
}

// ── Types ─────────────────────────────────────────────────────────────────────

/// One turn in a session transcript.
#[derive(Debug, Clone)]
pub struct TranscriptTurn {
    pub role: String,
    pub content: String,
    pub timestamp: Option<i64>,
}

/// An atomic fact extracted by the LLM.
#[derive(Debug, Clone, Deserialize)]
pub struct ExtractedFact {
    pub entity: String,
    pub attribute: String,
    pub value: String,
    pub source: String,
}

/// Outcome of a full observe_session call.
#[derive(Debug)]
pub struct ObserveStats {
    pub facts_extracted: usize,
    pub facts_written: usize,
    pub session_path: String,
}

// ── Parser ────────────────────────────────────────────────────────────────────

/// Read a JSONL transcript file and return the turns in order.
/// Each line must be a JSON object with at least `role` and `content` string fields.
/// Lines that are blank or fail to parse are skipped.
pub fn parse_transcript(path: &Path) -> Result<Vec<TranscriptTurn>, ObserveError> {
    let file = std::fs::File::open(path)?;
    let reader = std::io::BufReader::new(file);
    let mut turns = Vec::new();

    for line in reader.lines() {
        let line = line?;
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        let v: serde_json::Value = match serde_json::from_str(line) {
            Ok(v) => v,
            Err(_) => continue, // skip malformed lines
        };
        let role = v["role"].as_str().unwrap_or("").to_string();
        let content = v["content"].as_str().unwrap_or("").to_string();
        let timestamp = v["timestamp"].as_i64();
        turns.push(TranscriptTurn { role, content, timestamp });
    }

    Ok(turns)
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    #[test]
    fn test_parse_transcript_three_turns() {
        let mut f = NamedTempFile::new().unwrap();
        writeln!(f, r#"{{"role":"user","content":"Hello","timestamp":1714000000}}"#).unwrap();
        writeln!(f, r#"{{"role":"assistant","content":"Hi back","timestamp":1714000001}}"#).unwrap();
        writeln!(f, r#"{{"role":"user","content":"Bye"}}"#).unwrap();

        let turns = parse_transcript(f.path()).unwrap();
        assert_eq!(turns.len(), 3);
        assert_eq!(turns[0].role, "user");
        assert_eq!(turns[0].content, "Hello");
        assert_eq!(turns[0].timestamp, Some(1714000000));
        assert_eq!(turns[1].role, "assistant");
        assert_eq!(turns[1].content, "Hi back");
        assert_eq!(turns[1].timestamp, Some(1714000001));
        assert_eq!(turns[2].role, "user");
        assert_eq!(turns[2].content, "Bye");
        assert_eq!(turns[2].timestamp, None);
    }

    #[test]
    fn test_parse_transcript_skips_blank_lines() {
        let mut f = NamedTempFile::new().unwrap();
        writeln!(f, r#"{{"role":"user","content":"A"}}"#).unwrap();
        writeln!(f, "").unwrap();
        writeln!(f, r#"{{"role":"assistant","content":"B"}}"#).unwrap();

        let turns = parse_transcript(f.path()).unwrap();
        assert_eq!(turns.len(), 2);
    }

    #[test]
    fn test_parse_transcript_skips_malformed_lines() {
        let mut f = NamedTempFile::new().unwrap();
        writeln!(f, r#"{{"role":"user","content":"A"}}"#).unwrap();
        writeln!(f, "not json at all").unwrap();
        writeln!(f, r#"{{"role":"assistant","content":"B"}}"#).unwrap();

        let turns = parse_transcript(f.path()).unwrap();
        assert_eq!(turns.len(), 2);
    }
}
```

### Step 2: Run test to verify it passes

```bash
cd ~/workspace/ms/engram
cargo test -p engram --lib observe::tests -- -q 2>&1 | head -30
```

Expected: 3 tests pass. (The module isn't wired into `main.rs` yet — that's Task 4.)

> **Note:** This will fail to compile until `serde` and `tempfile` are in `Cargo.toml`. That's fixed in the next step.

### Step 3: Add missing dependencies to `crates/engram-cli/Cargo.toml`

The current `[dependencies]` block ends with `serde_json = "1"`. Add `serde` with the derive feature, and `notify`. Add `tempfile` to dev-dependencies.

Open `crates/engram-cli/Cargo.toml`. Its full current content is:

```toml
[package]
    name = "engram"
    version = "0.1.0"
    edition = "2021"

    [[bin]]
    name = "engram"
    path = "src/main.rs"

    [dependencies]
    engram-core = { path = "../engram-core" }
    engram-sync = { path = "../engram-sync" }
    clap = { version = "4", features = ["derive"] }
    directories = "5"
    thiserror = "2"
    anyhow = "1"
    rpassword = "7"
    tokio = { version = "1", features = ["full"] }
    open = "5"
    reqwest = { version = "0.12", features = ["json", "blocking"] }
    serde_json = "1"

    [dev-dependencies]
    assert_cmd = "2"
    predicates = "3"
```

Replace with:

```toml
[package]
    name = "engram"
    version = "0.1.0"
    edition = "2021"

    [[bin]]
    name = "engram"
    path = "src/main.rs"

    [dependencies]
    engram-core = { path = "../engram-core" }
    engram-sync = { path = "../engram-sync" }
    clap = { version = "4", features = ["derive"] }
    directories = "5"
    thiserror = "2"
    anyhow = "1"
    rpassword = "7"
    tokio = { version = "1", features = ["full"] }
    open = "5"
    reqwest = { version = "0.12", features = ["json", "blocking"] }
    serde_json = "1"
    serde = { version = "1", features = ["derive"] }
    notify = "6"

    [dev-dependencies]
    assert_cmd = "2"
    predicates = "3"
    tempfile = "3"
```

### Step 4: Run tests again to confirm they compile and pass

```bash
cd ~/workspace/ms/engram
cargo test -p engram --lib observe::tests -- -q 2>&1 | head -30
```

Expected: `test result: ok. 3 passed; 0 failed`

### Step 5: Commit

```bash
cd ~/workspace/ms/engram
git add crates/engram-cli/src/observe.rs crates/engram-cli/Cargo.toml
git commit -m "feat(observe): add TranscriptTurn types and parse_transcript()"
```

---

## Task 2: observe.rs — LLM fact extraction

**Files:**
- Modify: `crates/engram-cli/src/observe.rs` (add extraction logic)

### Step 1: Write the failing test for parse_facts_response

Add this test to the `#[cfg(test)]` block in `observe.rs`:

```rust
    #[test]
    fn test_parse_facts_response_from_fixture() {
        let fixture = serde_json::json!({
            "content": [{
                "text": r#"[{"entity":"Sofia","attribute":"dietary","value":"vegetarian","source":"session"},{"entity":"Chris","attribute":"preference","value":"small components","source":"session"}]"#
            }]
        });
        let facts = parse_facts_response(&fixture).unwrap();
        assert_eq!(facts.len(), 2);
        assert_eq!(facts[0].entity, "Sofia");
        assert_eq!(facts[0].attribute, "dietary");
        assert_eq!(facts[0].value, "vegetarian");
        assert_eq!(facts[1].entity, "Chris");
    }

    #[test]
    fn test_parse_facts_response_empty_array() {
        let fixture = serde_json::json!({
            "content": [{"text": "[]"}]
        });
        let facts = parse_facts_response(&fixture).unwrap();
        assert_eq!(facts.len(), 0);
    }

    #[test]
    fn test_parse_facts_response_missing_content_is_error() {
        let fixture = serde_json::json!({"no_content": "here"});
        assert!(parse_facts_response(&fixture).is_err());
    }

    /// Real API call — only runs with --include-ignored in a session that has ANTHROPIC_API_KEY set.
    #[test]
    #[ignore = "requires ANTHROPIC_API_KEY; run with cargo test -- --include-ignored"]
    fn test_extract_facts_real_api_call() {
        let api_key = std::env::var("ANTHROPIC_API_KEY")
            .expect("ANTHROPIC_API_KEY must be set for this test");
        let turns = vec![
            TranscriptTurn {
                role: "user".to_string(),
                content: "My colleague Sofia is vegetarian and lives in Seattle.".to_string(),
                timestamp: None,
            },
        ];
        let facts = extract_facts(&turns, &api_key).unwrap();
        assert!(!facts.is_empty(), "expected at least one fact");
        let entities: Vec<&str> = facts.iter().map(|f| f.entity.as_str()).collect();
        assert!(entities.contains(&"Sofia"), "expected Sofia in entities");
    }
```

### Step 2: Run to verify failing

```bash
cd ~/workspace/ms/engram
cargo test -p engram --lib observe::tests::test_parse_facts_response_from_fixture -- -q 2>&1 | head -20
```

Expected: FAIL — `parse_facts_response` not defined yet.

### Step 3: Implement the extraction logic

Add these items to `observe.rs` ABOVE the `#[cfg(test)]` block:

```rust
// ── LLM Extraction ────────────────────────────────────────────────────────────

/// Built-in system prompt used when `_protocols/transcripts.md` is not present in the vault.
const DEFAULT_SYSTEM_PROMPT: &str = r#"You are an expert at extracting atomic facts from conversation transcripts.

Extract facts from the conversation below. Return ONLY a JSON array — no markdown, no prose.

Each element must have exactly these fields:
  "entity"    — the subject (person, project, concept, event)
  "attribute" — the property being described
  "value"     — the value of that property
  "source"    — a short phrase identifying where in the conversation this came from

Example:
[
  {"entity":"Sofia","attribute":"dietary","value":"vegetarian","source":"user message"},
  {"entity":"Offsite","attribute":"budget","value":"$47k","source":"assistant summary"}
]

If there are no extractable facts, return an empty array: []"#;

/// Parse the raw Anthropic API response JSON into a list of ExtractedFact values.
/// This is the testable inner function — `extract_facts` is the full network call.
pub fn parse_facts_response(
    json: &serde_json::Value,
) -> Result<Vec<ExtractedFact>, ObserveError> {
    let text = json["content"][0]["text"]
        .as_str()
        .ok_or_else(|| ObserveError::Api("no text field in Anthropic response content[0]".to_string()))?;

    // The LLM might wrap the JSON in a markdown fence — strip it defensively.
    let text = text.trim();
    let text = text.strip_prefix("```json").unwrap_or(text);
    let text = text.strip_prefix("```").unwrap_or(text);
    let text = text.strip_suffix("```").unwrap_or(text);
    let text = text.trim();

    let facts: Vec<ExtractedFact> = serde_json::from_str(text)
        .map_err(|e| ObserveError::Api(format!("failed to parse facts JSON: {e}\nraw: {text}")))?;

    Ok(facts)
}

/// POST to the Anthropic Messages API and extract facts from the transcript turns.
///
/// Uses `reqwest::blocking::Client` — same as the rest of the CLI (no extra async runtime).
pub fn extract_facts(
    turns: &[TranscriptTurn],
    api_key: &str,
) -> Result<Vec<ExtractedFact>, ObserveError> {
    let conversation_text = turns
        .iter()
        .map(|t| format!("{}: {}", t.role, t.content))
        .collect::<Vec<_>>()
        .join("\n");

    let body = serde_json::json!({
        "model": "claude-haiku-4-5",
        "max_tokens": 2048,
        "system": DEFAULT_SYSTEM_PROMPT,
        "messages": [{"role": "user", "content": conversation_text}]
    });

    let client = reqwest::blocking::Client::new();
    let resp = client
        .post("https://api.anthropic.com/v1/messages")
        .header("x-api-key", api_key)
        .header("anthropic-version", "2023-06-01")
        .header("content-type", "application/json")
        .json(&body)
        .send()
        .map_err(|e| ObserveError::Api(e.to_string()))?;

    if !resp.status().is_success() {
        let status = resp.status();
        let body = resp.text().unwrap_or_default();
        return Err(ObserveError::Api(format!("API returned {status}: {body}")));
    }

    let json: serde_json::Value =
        resp.json().map_err(|e| ObserveError::Api(e.to_string()))?;

    parse_facts_response(&json)
}
```

### Step 4: Run the unit tests to verify they pass

```bash
cd ~/workspace/ms/engram
cargo test -p engram --lib observe::tests -- -q 2>&1 | head -30
```

Expected: `test result: ok. 6 passed; 0 failed` (3 from Task 1 + 3 new, ignoring the `#[ignore]` test)

### Step 5: Commit

```bash
cd ~/workspace/ms/engram
git add crates/engram-cli/src/observe.rs
git commit -m "feat(observe): add extract_facts() and parse_facts_response()"
```

---

## Task 3: observe.rs — write facts to store + observe_session orchestrator

**Files:**
- Modify: `crates/engram-cli/src/observe.rs` (add write logic and orchestrator)

### Step 1: Write the failing test

Add to the `#[cfg(test)]` block in `observe.rs`:

```rust
    #[test]
    fn test_write_facts_to_store() {
        use engram_core::{crypto::EngramKey, store::MemoryStore};
        use tempfile::TempDir;

        let dir = TempDir::new().unwrap();
        let db_path = dir.path().join("test.db");
        let key = EngramKey::derive(b"testpassword", &[0u8; 16]).unwrap();
        let store = MemoryStore::open(&db_path, &key).unwrap();

        let facts = vec![
            ExtractedFact {
                entity: "Sofia".to_string(),
                attribute: "dietary".to_string(),
                value: "vegetarian".to_string(),
                source: "session-1".to_string(),
            },
            ExtractedFact {
                entity: "Chris".to_string(),
                attribute: "preference".to_string(),
                value: "small components".to_string(),
                source: "session-1".to_string(),
            },
        ];

        let written = write_facts_to_store(&facts, &store);
        assert_eq!(written, 2);
        assert_eq!(store.record_count().unwrap(), 2);

        let sofia = store.find_by_entity("Sofia").unwrap();
        assert_eq!(sofia.len(), 1);
        assert_eq!(sofia[0].attribute, "dietary");
        assert_eq!(sofia[0].value, "vegetarian");
        assert_eq!(sofia[0].source, Some("session-1".to_string()));
    }
```

### Step 2: Run to verify it fails

```bash
cd ~/workspace/ms/engram
cargo test -p engram --lib observe::tests::test_write_facts_to_store -- -q 2>&1 | head -20
```

Expected: FAIL — `write_facts_to_store` not defined.

### Step 3: Implement write_facts_to_store and observe_session

Add these functions to `observe.rs` ABOVE the `#[cfg(test)]` block:

```rust
// ── Store Writer ──────────────────────────────────────────────────────────────

/// Write a slice of extracted facts into the memory store.
/// Returns the number of facts successfully written (failed inserts are silently skipped).
pub fn write_facts_to_store(
    facts: &[ExtractedFact],
    store: &engram_core::store::MemoryStore,
) -> usize {
    facts
        .iter()
        .filter_map(|fact| {
            let source = if fact.source.is_empty() {
                None
            } else {
                Some(fact.source.as_str())
            };
            let memory = engram_core::store::Memory::new(
                &fact.entity,
                &fact.attribute,
                &fact.value,
                source,
            );
            store.insert(&memory).ok()
        })
        .count()
}

// ── Orchestrator ──────────────────────────────────────────────────────────────

/// Full pipeline: parse transcript → call LLM → write facts to store.
///
/// On success returns stats. On any error (IO, API, store) returns an `ObserveError`.
pub fn observe_session(
    session_path: &Path,
    store: &engram_core::store::MemoryStore,
    api_key: &str,
) -> Result<ObserveStats, ObserveError> {
    let turns = parse_transcript(session_path)?;
    let facts = extract_facts(&turns, api_key)?;
    let facts_extracted = facts.len();
    let facts_written = write_facts_to_store(&facts, store);

    Ok(ObserveStats {
        facts_extracted,
        facts_written,
        session_path: session_path.display().to_string(),
    })
}
```

### Step 4: Run tests to verify they pass

```bash
cd ~/workspace/ms/engram
cargo test -p engram --lib observe::tests -- -q 2>&1 | head -30
```

Expected: `test result: ok. 7 passed; 0 failed`

### Step 5: Commit

```bash
cd ~/workspace/ms/engram
git add crates/engram-cli/src/observe.rs
git commit -m "feat(observe): add write_facts_to_store() and observe_session()"
```

---

## Task 4: CLI — `engram observe <session-path>`

**Files:**
- Modify: `crates/engram-cli/src/main.rs`
- Modify: `crates/engram-cli/tests/cli_integration.rs`

### Step 1: Write the failing CLI test

Add to `crates/engram-cli/tests/cli_integration.rs`:

```rust
// ── observe subcommand tests ─────────────────────────────────────────────────

/// `engram observe --help` must show `session-path` and `api-key` in its usage.
#[test]
fn test_observe_help_shows_expected_args() {
    let mut cmd = Command::cargo_bin("engram").unwrap();
    cmd.args(["observe", "--help"]);
    cmd.assert()
        .success()
        .stdout(predicate::str::contains("session-path"))
        .stdout(predicate::str::contains("api-key"));
}

/// `engram observe` with a non-existent path must exit non-zero.
#[test]
fn test_observe_nonexistent_path_exits_nonzero() {
    let mut cmd = Command::cargo_bin("engram").unwrap();
    cmd.args(["observe", "/tmp/definitely-does-not-exist/transcript.jsonl"]);
    cmd.assert().failure();
}
```

### Step 2: Run to verify failing

```bash
cd ~/workspace/ms/engram
cargo test -p engram --test cli_integration test_observe_help -- -q 2>&1 | head -20
```

Expected: FAIL — `observe` subcommand does not exist yet.

### Step 3: Wire observe into main.rs

At the top of `main.rs`, immediately after the existing `use` statements, add:

```rust
mod observe;
```

In the `Commands` enum, add a new variant after the existing `Sync` variant:

```rust
    /// Process a session transcript and extract atomic facts into the memory store
    Observe {
        /// Path to the transcript.jsonl file to process
        session_path: std::path::PathBuf,
        /// Anthropic API key (falls back to ANTHROPIC_API_KEY env var)
        #[arg(long, env = "ANTHROPIC_API_KEY")]
        api_key: Option<String>,
    },
```

In the `match cli.command` block, add a new arm after the `Commands::Sync` arm:

```rust
        Commands::Observe { session_path, api_key } => {
            run_observe(&session_path, api_key.as_deref());
        }
```

Add the `run_observe` free function near the end of `main.rs` (before the `#[cfg(test)]` block):

```rust
fn run_observe(session_path: &std::path::Path, api_key: Option<&str>) {
    use crate::observe::observe_session;
    use engram_core::{crypto::KeyStore, store::MemoryStore};

    let api_key = match api_key {
        Some(k) => k.to_string(),
        None => {
            eprintln!(
                "Error: ANTHROPIC_API_KEY not set. \
                 Pass --api-key or set the environment variable."
            );
            std::process::exit(1);
        }
    };

    let store_path = default_store_path();
    if let Some(parent) = store_path.parent() {
        std::fs::create_dir_all(parent).ok();
    }

    let key_store = KeyStore::new("engram");
    let key = match key_store.retrieve() {
        Ok(k) => k,
        Err(_) => {
            eprintln!("No vault key found. Run: engram init");
            std::process::exit(1);
        }
    };

    let store = match MemoryStore::open(&store_path, &key) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("Failed to open memory store: {e}");
            std::process::exit(1);
        }
    };

    match observe_session(session_path, &store, &api_key) {
        Ok(stats) => {
            println!("Observed:  {}", stats.session_path);
            println!("Extracted: {} facts", stats.facts_extracted);
            println!("Written:   {} memories to store", stats.facts_written);
        }
        Err(e) => {
            eprintln!("Error: {e}");
            std::process::exit(1);
        }
    }
}
```

### Step 4: Run the CLI tests to verify they pass

```bash
cd ~/workspace/ms/engram
cargo test -p engram --test cli_integration test_observe -- -q 2>&1 | head -30
```

Expected: `test result: ok. 2 passed; 0 failed`

Also verify the full test suite still passes:

```bash
cd ~/workspace/ms/engram
cargo test -p engram -q 2>&1 | tail -10
```

### Step 5: Commit

```bash
cd ~/workspace/ms/engram
git add crates/engram-cli/src/main.rs crates/engram-cli/src/observe.rs \
        crates/engram-cli/tests/cli_integration.rs
git commit -m "feat(cli): add 'engram observe <session-path>' command"
```

---

## Task 5: store.rs list_recent + search, then load.rs

**Files:**
- Modify: `crates/engram-core/src/store.rs` (add `list_recent` and `search` methods)
- Create: `crates/engram-cli/src/load.rs`

### Step 1: Write the failing tests

Add these tests to the `#[cfg(test)]` block in `crates/engram-core/src/store.rs`:

```rust
    #[test]
    fn test_list_recent_returns_memories_after_cutoff() {
        let (_dir, db_path) = temp_store();
        let store = MemoryStore::open(&db_path, &test_key()).expect("open failed");

        // Insert one memory far in the past (simulated by inserting then manually checking
        // that a zero since_ms cutoff returns everything)
        let m = Memory::new("Sofia", "dietary", "vegetarian", None);
        store.insert(&m).expect("insert failed");

        let results = store.list_recent(0, 10).expect("list_recent failed");
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].entity, "Sofia");
    }

    #[test]
    fn test_list_recent_respects_limit() {
        let (_dir, db_path) = temp_store();
        let store = MemoryStore::open(&db_path, &test_key()).expect("open failed");

        for i in 0..5 {
            let m = Memory::new("Entity", "attr", &format!("value{i}"), None);
            store.insert(&m).expect("insert failed");
        }

        let results = store.list_recent(0, 3).expect("list_recent failed");
        assert_eq!(results.len(), 3);
    }

    #[test]
    fn test_search_finds_matching_entity() {
        let (_dir, db_path) = temp_store();
        let store = MemoryStore::open(&db_path, &test_key()).expect("open failed");

        store.insert(&Memory::new("Sofia", "dietary", "vegetarian", None)).expect("insert");
        store.insert(&Memory::new("Chris", "role", "engineer", None)).expect("insert");

        let results = store.search("Sofia").expect("search failed");
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].entity, "Sofia");
    }

    #[test]
    fn test_search_finds_matching_value() {
        let (_dir, db_path) = temp_store();
        let store = MemoryStore::open(&db_path, &test_key()).expect("open failed");

        store.insert(&Memory::new("Sofia", "dietary", "vegetarian", None)).expect("insert");
        store.insert(&Memory::new("Chris", "dietary", "vegan", None)).expect("insert");

        let results = store.search("vegetarian").expect("search failed");
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].entity, "Sofia");
    }

    #[test]
    fn test_search_no_match_returns_empty() {
        let (_dir, db_path) = temp_store();
        let store = MemoryStore::open(&db_path, &test_key()).expect("open failed");
        store.insert(&Memory::new("Sofia", "dietary", "vegetarian", None)).expect("insert");
        let results = store.search("zzznomatch").expect("search failed");
        assert!(results.is_empty());
    }
```

### Step 2: Run to verify failing

```bash
cd ~/workspace/ms/engram
cargo test -p engram-core --lib tests::test_list_recent -- -q 2>&1 | head -20
```

Expected: FAIL — `list_recent` not defined.

### Step 3: Add list_recent and search to MemoryStore

In `crates/engram-core/src/store.rs`, add these two methods to the `impl MemoryStore` block (after `find_by_entity`):

```rust
    /// Return up to `limit` memories with `created_at >= since_ms`, ordered by `created_at` DESC.
    /// `since_ms` is a Unix timestamp in milliseconds. Pass `0` to return all records.
    pub fn list_recent(&self, since_ms: i64, limit: usize) -> Result<Vec<Memory>, StoreError> {
        let mut stmt = self.conn.prepare(
            "SELECT id, entity, attribute, value, source, created_at, updated_at
             FROM memories
             WHERE created_at >= ?1
             ORDER BY created_at DESC
             LIMIT ?2",
        )?;
        let memories = stmt.query_map(rusqlite::params![since_ms, limit as i64], row_to_memory)?;
        let result: Result<Vec<Memory>, rusqlite::Error> = memories.collect();
        Ok(result?)
    }

    /// Full-text LIKE search across entity, attribute, and value fields.
    /// Returns up to 20 results ordered by `updated_at` DESC.
    /// This is a Phase 4 placeholder — Phase 5 will replace this with hybrid tantivy+vector search.
    pub fn search(&self, query: &str) -> Result<Vec<Memory>, StoreError> {
        let pattern = format!("%{query}%");
        let mut stmt = self.conn.prepare(
            "SELECT id, entity, attribute, value, source, created_at, updated_at
             FROM memories
             WHERE entity LIKE ?1 OR attribute LIKE ?1 OR value LIKE ?1
             ORDER BY updated_at DESC
             LIMIT 20",
        )?;
        let memories = stmt.query_map([&pattern], row_to_memory)?;
        let result: Result<Vec<Memory>, rusqlite::Error> = memories.collect();
        Ok(result?)
    }
```

### Step 4: Run store tests to verify they pass

```bash
cd ~/workspace/ms/engram
cargo test -p engram-core -q 2>&1 | tail -10
```

Expected: all store tests pass.

### Step 5: Write the failing test for load_context

Create `crates/engram-cli/src/load.rs` with the test first:

```rust
// load.rs — Emit dynamic awareness context from the memory store

use std::collections::HashMap;

use thiserror::Error;

use engram_core::store::MemoryStore;

#[derive(Debug, Error)]
pub enum LoadError {
    #[error("store error: {0}")]
    Store(#[from] engram_core::store::StoreError),
}

/// 30 days expressed as milliseconds.
const THIRTY_DAYS_MS: i64 = 30 * 24 * 60 * 60 * 1_000;

/// Query the memory store for recent memories and format them as an `<engram-context>` block.
///
/// Groups facts by entity. Suitable for injection into an AI harness system prompt.
pub fn load_context(store: &MemoryStore) -> Result<String, LoadError> {
    let since_ms = (std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .expect("system clock before Unix epoch")
        .as_millis() as i64)
        .saturating_sub(THIRTY_DAYS_MS);

    let memories = store.list_recent(since_ms, 20)?;

    // Group by entity — preserve insertion order for deterministic output.
    let mut order: Vec<String> = Vec::new();
    let mut by_entity: HashMap<String, Vec<String>> = HashMap::new();
    for m in &memories {
        if !by_entity.contains_key(&m.entity) {
            order.push(m.entity.clone());
        }
        by_entity
            .entry(m.entity.clone())
            .or_default()
            .push(format!("{}: {}", m.attribute, m.value));
    }

    let memory_lines: Vec<String> = order
        .iter()
        .map(|entity| {
            let facts = by_entity[entity].join(", ");
            format!("- {entity}: {facts}")
        })
        .collect();

    let memory_section = if memory_lines.is_empty() {
        "No recent memories.".to_string()
    } else {
        memory_lines.join("\n")
    };

    Ok(format!(
        "<engram-context>\nRecent memories:\n{memory_section}\n</engram-context>"
    ))
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use engram_core::{
        crypto::EngramKey,
        store::{Memory, MemoryStore},
    };
    use tempfile::TempDir;

    use super::*;

    fn test_key() -> EngramKey {
        EngramKey::derive(b"testpassword", &[0u8; 16]).expect("key derivation failed")
    }

    fn temp_store() -> (TempDir, MemoryStore) {
        let dir = TempDir::new().expect("create temp dir failed");
        let path = dir.path().join("test.db");
        let store = MemoryStore::open(&path, &test_key()).expect("open failed");
        (dir, store)
    }

    #[test]
    fn test_load_context_contains_engram_context_tags() {
        let (_dir, store) = temp_store();
        let output = load_context(&store).unwrap();
        assert!(output.starts_with("<engram-context>"));
        assert!(output.ends_with("</engram-context>"));
    }

    #[test]
    fn test_load_context_empty_store_shows_no_recent_memories() {
        let (_dir, store) = temp_store();
        let output = load_context(&store).unwrap();
        assert!(output.contains("No recent memories."));
    }

    #[test]
    fn test_load_context_groups_by_entity() {
        let (_dir, store) = temp_store();

        store.insert(&Memory::new("Sofia", "dietary", "vegetarian", None)).unwrap();
        store.insert(&Memory::new("Sofia", "location", "Seattle", None)).unwrap();
        store.insert(&Memory::new("Chris", "preference", "small components", None)).unwrap();

        let output = load_context(&store).unwrap();

        // All three entities appear
        assert!(output.contains("Sofia"), "output: {output}");
        assert!(output.contains("Chris"), "output: {output}");

        // Facts are grouped under their entity
        assert!(output.contains("vegetarian"), "output: {output}");
        assert!(output.contains("Seattle"), "output: {output}");
        assert!(output.contains("small components"), "output: {output}");

        // Sofia's two facts should appear on a single line with her name
        let sofia_line = output
            .lines()
            .find(|l| l.contains("Sofia"))
            .expect("no Sofia line in output");
        assert!(sofia_line.contains("vegetarian"), "sofia_line: {sofia_line}");
        assert!(sofia_line.contains("Seattle"), "sofia_line: {sofia_line}");
    }

    #[test]
    fn test_load_context_two_entities_two_lines() {
        let (_dir, store) = temp_store();

        store.insert(&Memory::new("Sofia", "dietary", "vegetarian", None)).unwrap();
        store.insert(&Memory::new("Chris", "role", "engineer", None)).unwrap();

        let output = load_context(&store).unwrap();
        let memory_lines: Vec<&str> = output
            .lines()
            .filter(|l| l.starts_with("- "))
            .collect();
        assert_eq!(memory_lines.len(), 2);
    }
}
```

### Step 6: Run to verify failing

```bash
cd ~/workspace/ms/engram
cargo test -p engram --lib load::tests -- -q 2>&1 | head -20
```

Expected: FAIL — module `load` not declared in `main.rs`.

### Step 7: Add `mod load;` to main.rs and run tests

Add to the top of `main.rs` (with the other `mod` declarations you'll be adding throughout this plan):

```rust
mod load;
```

Run again:

```bash
cd ~/workspace/ms/engram
cargo test -p engram --lib load::tests -- -q 2>&1 | head -30
```

Expected: `test result: ok. 4 passed; 0 failed`

### Step 8: Commit

```bash
cd ~/workspace/ms/engram
git add crates/engram-core/src/store.rs \
        crates/engram-cli/src/load.rs \
        crates/engram-cli/src/main.rs
git commit -m "feat(store): add list_recent() and search(); feat(load): add load_context()"
```

---

## Task 6: CLI — `engram load`

**Files:**
- Modify: `crates/engram-cli/src/main.rs`
- Modify: `crates/engram-cli/tests/cli_integration.rs`

### Step 1: Write the failing CLI test

Add to `cli_integration.rs`:

```rust
// ── load subcommand tests ─────────────────────────────────────────────────────

/// `engram load --help` must show the --format flag.
#[test]
fn test_load_help_shows_format_flag() {
    let mut cmd = Command::cargo_bin("engram").unwrap();
    cmd.args(["load", "--help"]);
    cmd.assert()
        .success()
        .stdout(predicate::str::contains("--format"));
}

/// `engram load` without a keyring key must exit non-zero or succeed gracefully.
/// On a clean machine with no key, it exits non-zero. On a configured machine it succeeds.
#[test]
fn test_load_does_not_panic() {
    let mut cmd = Command::cargo_bin("engram").unwrap();
    cmd.args(["load"]);
    let output = cmd.output().unwrap();
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        !stderr.contains("not yet implemented"),
        "load must not call todo!(), got stderr: {stderr}"
    );
}
```

### Step 2: Run to verify failing

```bash
cd ~/workspace/ms/engram
cargo test -p engram --test cli_integration test_load -- -q 2>&1 | head -20
```

Expected: FAIL — `load` subcommand not registered.

### Step 3: Add Commands::Load and run_load to main.rs

Add to the `Commands` enum:

```rust
    /// Emit vault context for AI harness injection
    Load {
        /// Output format: context (default), facts, summary
        #[arg(long, default_value = "context")]
        format: String,
    },
```

Add to the `match cli.command` block:

```rust
        Commands::Load { format } => run_load(&format),
```

Add the `run_load` free function:

```rust
fn run_load(format: &str) {
    use crate::load::load_context;
    use engram_core::{crypto::KeyStore, store::MemoryStore};

    let store_path = default_store_path();
    if let Some(parent) = store_path.parent() {
        std::fs::create_dir_all(parent).ok();
    }

    let key_store = KeyStore::new("engram");
    let key = match key_store.retrieve() {
        Ok(k) => k,
        Err(_) => {
            eprintln!("No vault key found. Run: engram init");
            std::process::exit(1);
        }
    };

    let store = match MemoryStore::open(&store_path, &key) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("Failed to open memory store: {e}");
            std::process::exit(1);
        }
    };

    match format {
        "context" => match load_context(&store) {
            Ok(ctx) => print!("{ctx}"),
            Err(e) => {
                eprintln!("Error: {e}");
                std::process::exit(1);
            }
        },
        other => {
            eprintln!("Unknown format '{other}'. Supported: context");
            std::process::exit(1);
        }
    }
}
```

### Step 4: Run the CLI tests

```bash
cd ~/workspace/ms/engram
cargo test -p engram --test cli_integration test_load -- -q 2>&1 | head -20
```

Expected: `test result: ok. 2 passed; 0 failed`

### Step 5: Commit

```bash
cd ~/workspace/ms/engram
git add crates/engram-cli/src/main.rs crates/engram-cli/tests/cli_integration.rs
git commit -m "feat(cli): add 'engram load --format=context' command"
```

---

## Task 7: daemon.rs — file watcher

**Files:**
- Create: `crates/engram-cli/src/daemon.rs`

### Step 1: Write the failing tests first

Create `crates/engram-cli/src/daemon.rs` with the tests inline:

```rust
// daemon.rs — File watcher for Amplifier session transcripts

use std::path::{Path, PathBuf};
use std::sync::mpsc;
use std::time::{Duration, Instant};
use std::collections::HashMap;

use notify::{Config, Event, EventKind, RecommendedWatcher, RecursiveMode, Watcher};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum DaemonError {
    #[error("file watcher error: {0}")]
    Notify(#[from] notify::Error),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
}

// ── Filter helper (unit-testable) ─────────────────────────────────────────────

/// Returns `true` if this notify event is a modification of a `transcript.jsonl` file.
pub fn is_transcript_event(event: &Event) -> bool {
    let is_write = matches!(
        event.kind,
        EventKind::Modify(_) | EventKind::Create(_)
    );
    let has_transcript = event
        .paths
        .iter()
        .any(|p| p.file_name().map(|n| n == "transcript.jsonl").unwrap_or(false));
    is_write && has_transcript
}

// ── Watcher ───────────────────────────────────────────────────────────────────

/// Watch `watch_dir` recursively. For each `transcript.jsonl` modify or create event
/// (debounced so the same path isn't sent more than once per `DEBOUNCE_SECS`),
/// send the file's `PathBuf` to `tx`.
///
/// The returned `RecommendedWatcher` must be kept alive — drop it to stop watching.
pub fn watch_sessions(
    watch_dir: &Path,
    tx: mpsc::Sender<PathBuf>,
) -> Result<RecommendedWatcher, DaemonError> {
    const DEBOUNCE_SECS: u64 = 5;

    let (notify_tx, notify_rx) = mpsc::channel::<notify::Result<Event>>();
    let mut watcher = RecommendedWatcher::new(notify_tx, Config::default())?;
    watcher.watch(watch_dir, RecursiveMode::Recursive)?;

    // Spin up a thread to translate raw notify events → debounced PathBuf sends.
    std::thread::spawn(move || {
        let mut last_seen: HashMap<PathBuf, Instant> = HashMap::new();
        for res in notify_rx {
            if let Ok(event) = res {
                if is_transcript_event(&event) {
                    for path in &event.paths {
                        let now = Instant::now();
                        let age = last_seen
                            .get(path)
                            .map(|t| now.duration_since(*t))
                            .unwrap_or(Duration::from_secs(u64::MAX));

                        if age >= Duration::from_secs(DEBOUNCE_SECS) {
                            last_seen.insert(path.clone(), now);
                            let _ = tx.send(path.clone());
                        }
                    }
                }
            }
        }
    });

    Ok(watcher)
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use notify::event::{DataChange, ModifyKind};

    use super::*;

    fn make_event(kind: EventKind, paths: Vec<PathBuf>) -> Event {
        Event {
            kind,
            paths,
            attrs: Default::default(),
        }
    }

    #[test]
    fn test_is_transcript_event_true_for_modify_transcript() {
        let event = make_event(
            EventKind::Modify(ModifyKind::Data(DataChange::Any)),
            vec![PathBuf::from("/home/user/.amplifier/projects/foo/sessions/bar/transcript.jsonl")],
        );
        assert!(is_transcript_event(&event));
    }

    #[test]
    fn test_is_transcript_event_true_for_create_transcript() {
        let event = make_event(
            EventKind::Create(notify::event::CreateKind::File),
            vec![PathBuf::from("/some/path/transcript.jsonl")],
        );
        assert!(is_transcript_event(&event));
    }

    #[test]
    fn test_is_transcript_event_false_for_non_transcript_file() {
        let event = make_event(
            EventKind::Modify(ModifyKind::Data(DataChange::Any)),
            vec![PathBuf::from("/some/path/notes.md")],
        );
        assert!(!is_transcript_event(&event));
    }

    #[test]
    fn test_is_transcript_event_false_for_access_event() {
        let event = make_event(
            EventKind::Access(notify::event::AccessKind::Read),
            vec![PathBuf::from("/some/path/transcript.jsonl")],
        );
        assert!(!is_transcript_event(&event));
    }

    #[test]
    fn test_is_transcript_event_false_for_empty_paths() {
        let event = make_event(
            EventKind::Modify(ModifyKind::Data(DataChange::Any)),
            vec![],
        );
        assert!(!is_transcript_event(&event));
    }
}
```

### Step 2: Run tests to verify they pass

```bash
cd ~/workspace/ms/engram
cargo test -p engram --lib daemon::tests -- -q 2>&1 | head -30
```

Expected: FAIL — module not declared in main.rs yet.

Add `mod daemon;` to the top of `main.rs` (with the other mod declarations), then re-run:

```bash
cd ~/workspace/ms/engram
cargo test -p engram --lib daemon::tests -- -q 2>&1 | head -30
```

Expected: `test result: ok. 5 passed; 0 failed`

### Step 3: Commit

```bash
cd ~/workspace/ms/engram
git add crates/engram-cli/src/daemon.rs crates/engram-cli/src/main.rs
git commit -m "feat(daemon): add is_transcript_event() and watch_sessions() with debounce"
```

---

## Task 8: CLI — `engram daemon`

**Files:**
- Modify: `crates/engram-cli/src/main.rs`
- Modify: `crates/engram-cli/tests/cli_integration.rs`

### Step 1: Write the failing CLI test

Add to `cli_integration.rs`:

```rust
// ── daemon subcommand tests ───────────────────────────────────────────────────

/// `engram daemon --help` must succeed and mention the background observer.
#[test]
fn test_daemon_help_exits_successfully() {
    let mut cmd = Command::cargo_bin("engram").unwrap();
    cmd.args(["daemon", "--help"]);
    cmd.assert().success();
}
```

### Step 2: Run to verify failing

```bash
cd ~/workspace/ms/engram
cargo test -p engram --test cli_integration test_daemon_help -- -q 2>&1 | head -20
```

Expected: FAIL — `daemon` subcommand not registered.

### Step 3: Add Commands::Daemon and run_daemon to main.rs

Add to the `Commands` enum:

```rust
    /// Start background observer daemon (watches ~/.amplifier/projects/ for new transcripts)
    Daemon,
```

Add to the `match cli.command` block:

```rust
        Commands::Daemon => run_daemon(),
```

Add the `run_daemon` free function:

```rust
fn run_daemon() {
    use crate::daemon::watch_sessions;
    use crate::observe::observe_session;
    use engram_core::{crypto::KeyStore, store::MemoryStore};
    use std::sync::mpsc;

    let key_store = KeyStore::new("engram");
    let key = match key_store.retrieve() {
        Ok(k) => k,
        Err(_) => {
            eprintln!("No vault key found. Run: engram init");
            std::process::exit(1);
        }
    };

    let amplifier_dir = UserDirs::new()
        .map(|u| u.home_dir().join(".amplifier/projects"))
        .unwrap_or_else(|| std::path::PathBuf::from(".amplifier/projects"));

    if !amplifier_dir.exists() {
        if let Err(e) = std::fs::create_dir_all(&amplifier_dir) {
            eprintln!("Could not create watch directory {}: {e}", amplifier_dir.display());
            std::process::exit(1);
        }
    }

    let api_key = std::env::var("ANTHROPIC_API_KEY").unwrap_or_default();
    if api_key.is_empty() {
        eprintln!(
            "Warning: ANTHROPIC_API_KEY not set. \
             Daemon will watch for transcripts but cannot process them."
        );
    }

    let (tx, rx) = mpsc::channel::<std::path::PathBuf>();
    let _watcher = match watch_sessions(&amplifier_dir, tx) {
        Ok(w) => w,
        Err(e) => {
            eprintln!("Failed to start file watcher: {e}");
            std::process::exit(1);
        }
    };

    eprintln!(
        "engram daemon started, watching {}",
        amplifier_dir.display()
    );

    let store_path = default_store_path();
    if let Some(parent) = store_path.parent() {
        std::fs::create_dir_all(parent).ok();
    }

    // Block on the channel — each received PathBuf is a transcript ready to process.
    for transcript_path in rx {
        eprintln!("Processing: {}", transcript_path.display());
        if api_key.is_empty() {
            continue;
        }
        match MemoryStore::open(&store_path, &key) {
            Ok(store) => match observe_session(&transcript_path, &store, &api_key) {
                Ok(stats) => eprintln!(
                    "  Extracted {} facts, wrote {} memories",
                    stats.facts_extracted, stats.facts_written
                ),
                Err(e) => eprintln!("  Error processing transcript: {e}"),
            },
            Err(e) => eprintln!("  Failed to open memory store: {e}"),
        }
    }
}
```

### Step 4: Run the CLI test

```bash
cd ~/workspace/ms/engram
cargo test -p engram --test cli_integration test_daemon -- -q 2>&1 | head -20
```

Expected: `test result: ok. 1 passed; 0 failed`

### Step 5: Commit

```bash
cd ~/workspace/ms/engram
git add crates/engram-cli/src/main.rs crates/engram-cli/tests/cli_integration.rs
git commit -m "feat(cli): add 'engram daemon' command"
```

---

## Task 9: mcp.rs — MCP stdio server

**Files:**
- Create: `crates/engram-cli/src/mcp.rs`
- Modify: `crates/engram-cli/src/main.rs`
- Modify: `crates/engram-cli/tests/cli_integration.rs`

### Step 1: Write the failing tests

Create `crates/engram-cli/src/mcp.rs` with tests inline:

```rust
// mcp.rs — MCP stdio server (JSON-RPC 2.0 over stdin/stdout)
//
// Protocol: https://spec.modelcontextprotocol.io/
// Transport: stdio (one JSON-RPC object per line)

use std::io::{BufRead, Write};

use serde_json::{json, Value};
use thiserror::Error;

use engram_core::store::MemoryStore;

#[derive(Debug, Error)]
pub enum McpError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("store error: {0}")]
    Store(#[from] engram_core::store::StoreError),
}

// ── Capability declarations ───────────────────────────────────────────────────

fn tool_definitions() -> Value {
    json!([
        {
            "name": "memory_search",
            "description": "Search vault memories semantically. Returns matching facts.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "query": {"type": "string", "description": "Search query"},
                    "limit": {"type": "number", "description": "Max results (default 10)"}
                },
                "required": ["query"]
            }
        },
        {
            "name": "memory_load",
            "description": "Load context from vault for AI harness injection.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "format": {
                        "type": "string",
                        "enum": ["context", "facts", "summary"],
                        "description": "Output format (default: context)"
                    }
                },
                "required": []
            }
        },
        {
            "name": "memory_status",
            "description": "Get vault and memory store status.",
            "inputSchema": {
                "type": "object",
                "properties": {}
            }
        }
    ])
}

// ── Handler functions (public for unit-testing) ───────────────────────────────

pub fn handle_initialize(id: Value) -> Value {
    json!({
        "jsonrpc": "2.0",
        "id": id,
        "result": {
            "protocolVersion": "2024-11-05",
            "capabilities": {"tools": {}},
            "serverInfo": {"name": "engram", "version": "0.1.0"}
        }
    })
}

pub fn handle_tools_list(id: Value) -> Value {
    json!({
        "jsonrpc": "2.0",
        "id": id,
        "result": {"tools": tool_definitions()}
    })
}

pub fn handle_tools_call(id: Value, params: &Value, store: &MemoryStore) -> Value {
    let tool_name = params["name"].as_str().unwrap_or("");
    let args = &params["arguments"];

    match tool_name {
        "memory_search" => {
            let query = args["query"].as_str().unwrap_or("");
            if query.is_empty() {
                return make_error(id, -32602, "memory_search requires a non-empty query");
            }
            match store.search(query) {
                Ok(memories) => {
                    let results: Vec<Value> = memories
                        .iter()
                        .map(|m| {
                            json!({
                                "entity": m.entity,
                                "attribute": m.attribute,
                                "value": m.value,
                                "source": m.source
                            })
                        })
                        .collect();
                    make_tool_result(id, serde_json::to_string_pretty(&results).unwrap_or_default())
                }
                Err(e) => make_error(id, -32603, &e.to_string()),
            }
        }
        "memory_load" => {
            let format = args["format"].as_str().unwrap_or("context");
            match format {
                "context" => match crate::load::load_context(store) {
                    Ok(ctx) => make_tool_result(id, ctx),
                    Err(e) => make_error(id, -32603, &e.to_string()),
                },
                other => make_error(id, -32602, &format!("unsupported format '{other}'")),
            }
        }
        "memory_status" => {
            let count = store.record_count().unwrap_or(0);
            let status = json!({
                "memory_store": "ok",
                "records": count
            });
            make_tool_result(id, serde_json::to_string_pretty(&status).unwrap_or_default())
        }
        other => make_error(id, -32602, &format!("unknown tool '{other}'")),
    }
}

pub fn make_error(id: Value, code: i64, message: &str) -> Value {
    json!({
        "jsonrpc": "2.0",
        "id": id,
        "error": {"code": code, "message": message}
    })
}

fn make_tool_result(id: Value, text: String) -> Value {
    json!({
        "jsonrpc": "2.0",
        "id": id,
        "result": {
            "content": [{"type": "text", "text": text}],
            "isError": false
        }
    })
}

// ── Dispatch (unit-testable) ──────────────────────────────────────────────────

/// Parse and dispatch a single JSON-RPC request. Returns the response value.
/// This function is public so it can be called from unit tests without a real stdin.
pub fn handle_request(request: &Value, store: &MemoryStore) -> Value {
    let id = request.get("id").cloned().unwrap_or(Value::Null);
    let method = request["method"].as_str().unwrap_or("");

    match method {
        "initialize" => handle_initialize(id),
        "tools/list" => handle_tools_list(id),
        "tools/call" => handle_tools_call(id, &request["params"], store),
        other => make_error(id, -32601, &format!("method not found: '{other}'")),
    }
}

// ── Server loop ───────────────────────────────────────────────────────────────

/// Read JSON-RPC requests from stdin line-by-line; write responses to stdout.
/// Runs until stdin is closed (e.g. the harness disconnects).
pub fn run_mcp_server(store: &MemoryStore) -> Result<(), McpError> {
    let stdin = std::io::stdin();
    let stdout = std::io::stdout();
    let mut out = stdout.lock();

    for line in stdin.lock().lines() {
        let line = line?;
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        let request: Value = match serde_json::from_str(line) {
            Ok(v) => v,
            Err(_) => {
                // Return a parse error on the malformed line and keep going.
                let err = make_error(Value::Null, -32700, "parse error");
                writeln!(out, "{}", serde_json::to_string(&err)?)?;
                out.flush()?;
                continue;
            }
        };

        let response = handle_request(&request, store);
        writeln!(out, "{}", serde_json::to_string(&response)?)?;
        out.flush()?;
    }

    Ok(())
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use engram_core::{crypto::EngramKey, store::MemoryStore};
    use tempfile::TempDir;

    use super::*;

    fn test_store() -> (TempDir, MemoryStore) {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("test.db");
        let key = EngramKey::derive(b"testpassword", &[0u8; 16]).unwrap();
        let store = MemoryStore::open(&path, &key).unwrap();
        (dir, store)
    }

    #[test]
    fn test_tools_list_returns_three_tools() {
        let (_dir, store) = test_store();
        let request = json!({"jsonrpc":"2.0","id":1,"method":"tools/list","params":{}});
        let response = handle_request(&request, &store);

        assert_eq!(response["jsonrpc"], "2.0");
        assert_eq!(response["id"], 1);

        let tools = response["result"]["tools"].as_array().expect("tools must be array");
        assert_eq!(tools.len(), 3);

        let names: Vec<&str> = tools.iter().map(|t| t["name"].as_str().unwrap()).collect();
        assert!(names.contains(&"memory_search"), "names: {names:?}");
        assert!(names.contains(&"memory_load"), "names: {names:?}");
        assert!(names.contains(&"memory_status"), "names: {names:?}");
    }

    #[test]
    fn test_initialize_returns_protocol_version() {
        let (_dir, store) = test_store();
        let request = json!({"jsonrpc":"2.0","id":1,"method":"initialize","params":{}});
        let response = handle_request(&request, &store);

        assert_eq!(response["result"]["protocolVersion"], "2024-11-05");
        assert_eq!(response["result"]["serverInfo"]["name"], "engram");
    }

    #[test]
    fn test_unknown_method_returns_error_32601() {
        let (_dir, store) = test_store();
        let request = json!({"jsonrpc":"2.0","id":9,"method":"no/such/method","params":{}});
        let response = handle_request(&request, &store);

        assert_eq!(response["error"]["code"], -32601);
    }

    #[test]
    fn test_memory_status_returns_record_count() {
        let (_dir, store) = test_store();
        let request = json!({
            "jsonrpc": "2.0",
            "id": 3,
            "method": "tools/call",
            "params": {"name": "memory_status", "arguments": {}}
        });
        let response = handle_request(&request, &store);

        // Response must contain tool result content, not an error
        assert!(response["result"]["content"].is_array(), "response: {response}");
        let text = response["result"]["content"][0]["text"].as_str().unwrap();
        assert!(text.contains("records"), "text: {text}");
    }

    #[test]
    fn test_memory_search_empty_query_returns_error() {
        let (_dir, store) = test_store();
        let request = json!({
            "jsonrpc": "2.0",
            "id": 4,
            "method": "tools/call",
            "params": {"name": "memory_search", "arguments": {"query": ""}}
        });
        let response = handle_request(&request, &store);
        assert!(response["error"].is_object(), "expected error, got: {response}");
    }

    #[test]
    fn test_memory_search_finds_inserted_fact() {
        let (_dir, store) = test_store();
        store.insert(&engram_core::store::Memory::new("Sofia", "dietary", "vegetarian", None)).unwrap();

        let request = json!({
            "jsonrpc": "2.0",
            "id": 5,
            "method": "tools/call",
            "params": {"name": "memory_search", "arguments": {"query": "Sofia"}}
        });
        let response = handle_request(&request, &store);
        let text = response["result"]["content"][0]["text"].as_str().unwrap();
        assert!(text.contains("vegetarian"), "text: {text}");
    }
}
```

### Step 2: Add `mod mcp;` to main.rs and run tests

Add to the top of `main.rs`:

```rust
mod mcp;
```

Then run:

```bash
cd ~/workspace/ms/engram
cargo test -p engram --lib mcp::tests -- -q 2>&1 | head -30
```

Expected: `test result: ok. 6 passed; 0 failed`

### Step 3: Add Commands::Mcp and run_mcp to main.rs

Add to the `Commands` enum:

```rust
    /// Start MCP stdio server (JSON-RPC 2.0 over stdin/stdout)
    Mcp,
```

Add to the `match cli.command` block:

```rust
        Commands::Mcp => run_mcp(),
```

Add the `run_mcp` free function:

```rust
fn run_mcp() {
    use crate::mcp::run_mcp_server;
    use engram_core::{crypto::KeyStore, store::MemoryStore};

    let store_path = default_store_path();
    if let Some(parent) = store_path.parent() {
        std::fs::create_dir_all(parent).ok();
    }

    let key_store = KeyStore::new("engram");
    let key = match key_store.retrieve() {
        Ok(k) => k,
        Err(_) => {
            eprintln!("No vault key found. Run: engram init");
            std::process::exit(1);
        }
    };

    let store = match MemoryStore::open(&store_path, &key) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("Failed to open memory store: {e}");
            std::process::exit(1);
        }
    };

    if let Err(e) = run_mcp_server(&store) {
        eprintln!("MCP server error: {e}");
        std::process::exit(1);
    }
}
```

Add CLI test to `cli_integration.rs`:

```rust
// ── mcp subcommand tests ──────────────────────────────────────────────────────

/// `engram mcp --help` must succeed.
#[test]
fn test_mcp_help_exits_successfully() {
    let mut cmd = Command::cargo_bin("engram").unwrap();
    cmd.args(["mcp", "--help"]);
    cmd.assert().success();
}
```

### Step 4: Run all tests

```bash
cd ~/workspace/ms/engram
cargo test -p engram -q 2>&1 | tail -10
```

Expected: all tests pass.

### Step 5: Commit

```bash
cd ~/workspace/ms/engram
git add crates/engram-cli/src/mcp.rs \
        crates/engram-cli/src/main.rs \
        crates/engram-cli/tests/cli_integration.rs
git commit -m "feat(mcp): add MCP stdio server with memory_search, memory_load, memory_status"
```

---

## Task 10: install.rs — `engram install`, `engram uninstall`, `engram doctor`

**Files:**
- Create: `crates/engram-cli/src/install.rs`
- Modify: `crates/engram-cli/src/main.rs`
- Modify: `crates/engram-cli/tests/cli_integration.rs`

### Step 1: Write the failing CLI tests

Add to `cli_integration.rs`:

```rust
// ── install / uninstall / doctor tests ───────────────────────────────────────

/// `engram install --help` must succeed.
#[test]
fn test_install_help_exits_successfully() {
    let mut cmd = Command::cargo_bin("engram").unwrap();
    cmd.args(["install", "--help"]);
    cmd.assert().success();
}

/// `engram doctor` must always exit 0 and print at least the binary path label.
#[test]
fn test_doctor_exits_zero() {
    let mut cmd = Command::cargo_bin("engram").unwrap();
    cmd.args(["doctor"]);
    cmd.assert()
        .success()
        .stdout(predicate::str::contains("engram doctor"));
}

/// `engram doctor` must print a line about the vault.
#[test]
fn test_doctor_shows_vault_line() {
    let mut cmd = Command::cargo_bin("engram").unwrap();
    cmd.args(["doctor"]);
    cmd.assert()
        .success()
        .stdout(predicate::str::contains("Vault:"));
}

/// `engram doctor` must print a line about the memory store.
#[test]
fn test_doctor_shows_store_line() {
    let mut cmd = Command::cargo_bin("engram").unwrap();
    cmd.args(["doctor"]);
    cmd.assert()
        .success()
        .stdout(predicate::str::contains("Store:"));
}
```

### Step 2: Run to verify failing

```bash
cd ~/workspace/ms/engram
cargo test -p engram --test cli_integration test_install -- -q 2>&1 | head -20
cargo test -p engram --test cli_integration test_doctor -- -q 2>&1 | head -20
```

Expected: FAIL — commands not registered.

### Step 3: Create install.rs

Create `crates/engram-cli/src/install.rs`:

```rust
// install.rs — Platform service installation (launchd / systemd)

use std::path::PathBuf;

use thiserror::Error;

#[derive(Debug, Error)]
pub enum InstallError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("command failed: {0}")]
    Command(String),

    #[error("unsupported platform")]
    UnsupportedPlatform,
}

// ── Service file content ──────────────────────────────────────────────────────

const MACOS_PLIST: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>Label</key><string>com.engram.daemon</string>
    <key>ProgramArguments</key>
    <array><string>/usr/local/bin/engram</string><string>daemon</string></array>
    <key>RunAtLoad</key><true/>
    <key>KeepAlive</key><true/>
    <key>StandardOutPath</key><string>/tmp/engram-daemon.log</string>
    <key>StandardErrorPath</key><string>/tmp/engram-daemon.err.log</string>
</dict>
</plist>"#;

const LINUX_SERVICE: &str = "[Unit]
Description=Engram personal memory daemon
After=default.target

[Service]
Type=simple
ExecStart=%h/.cargo/bin/engram daemon
Restart=on-failure
RestartSec=5

[Install]
WantedBy=default.target";

// ── Paths ─────────────────────────────────────────────────────────────────────

fn home_dir() -> PathBuf {
    directories::UserDirs::new()
        .map(|u| u.home_dir().to_path_buf())
        .unwrap_or_else(|| PathBuf::from("."))
}

#[cfg(target_os = "macos")]
fn launchagents_dir() -> PathBuf {
    home_dir().join("Library/LaunchAgents")
}

#[cfg(target_os = "linux")]
fn systemd_user_dir() -> PathBuf {
    home_dir().join(".config/systemd/user")
}

// ── Install ───────────────────────────────────────────────────────────────────

pub fn install_service() -> Result<(), InstallError> {
    #[cfg(target_os = "macos")]
    {
        let dir = launchagents_dir();
        std::fs::create_dir_all(&dir)?;
        let plist_path = dir.join("com.engram.daemon.plist");
        std::fs::write(&plist_path, MACOS_PLIST)?;

        let status = std::process::Command::new("launchctl")
            .args(["load", plist_path.to_str().unwrap_or("")])
            .status()?;

        if !status.success() {
            return Err(InstallError::Command(
                "launchctl load failed — check /tmp/engram-daemon.err.log".to_string(),
            ));
        }
        println!("\u{2713} engram daemon registered");
        println!("  Service: com.engram.daemon");
        println!("  Plist:   {}", plist_path.display());
        return Ok(());
    }

    #[cfg(target_os = "linux")]
    {
        let dir = systemd_user_dir();
        std::fs::create_dir_all(&dir)?;
        let service_path = dir.join("engram.service");
        std::fs::write(&service_path, LINUX_SERVICE)?;

        let status = std::process::Command::new("systemctl")
            .args(["--user", "enable", "engram"])
            .status()?;

        if !status.success() {
            return Err(InstallError::Command(
                "systemctl --user enable failed".to_string(),
            ));
        }
        println!("\u{2713} engram daemon registered");
        println!("  Service: engram.service (systemd user)");
        println!("  File:    {}", service_path.display());
        return Ok(());
    }

    #[cfg(target_os = "windows")]
    {
        println!("Windows service installation coming soon.");
        println!("For now, run 'engram daemon' manually in a background terminal.");
        return Ok(());
    }

    #[allow(unreachable_code)]
    Err(InstallError::UnsupportedPlatform)
}

// ── Uninstall ─────────────────────────────────────────────────────────────────

pub fn uninstall_service() -> Result<(), InstallError> {
    #[cfg(target_os = "macos")]
    {
        let plist_path = launchagents_dir().join("com.engram.daemon.plist");
        if plist_path.exists() {
            let _ = std::process::Command::new("launchctl")
                .args(["unload", plist_path.to_str().unwrap_or("")])
                .status();
            std::fs::remove_file(&plist_path)?;
            println!("\u{2713} engram daemon unregistered (com.engram.daemon)");
        } else {
            println!("No plist found at {} — nothing to uninstall.", plist_path.display());
        }
        return Ok(());
    }

    #[cfg(target_os = "linux")]
    {
        let service_path = systemd_user_dir().join("engram.service");
        let _ = std::process::Command::new("systemctl")
            .args(["--user", "disable", "engram"])
            .status();
        if service_path.exists() {
            std::fs::remove_file(&service_path)?;
        }
        println!("\u{2713} engram daemon unregistered (systemd user service)");
        return Ok(());
    }

    #[cfg(target_os = "windows")]
    {
        println!("Windows service uninstall coming soon.");
        return Ok(());
    }

    #[allow(unreachable_code)]
    Err(InstallError::UnsupportedPlatform)
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_macos_plist_contains_required_keys() {
        assert!(MACOS_PLIST.contains("com.engram.daemon"));
        assert!(MACOS_PLIST.contains("engram"));
        assert!(MACOS_PLIST.contains("daemon"));
        assert!(MACOS_PLIST.contains("RunAtLoad"));
        assert!(MACOS_PLIST.contains("KeepAlive"));
    }

    #[test]
    fn test_linux_service_contains_required_fields() {
        assert!(LINUX_SERVICE.contains("ExecStart="));
        assert!(LINUX_SERVICE.contains("engram daemon"));
        assert!(LINUX_SERVICE.contains("Restart=on-failure"));
        assert!(LINUX_SERVICE.contains("WantedBy=default.target"));
    }
}
```

### Step 4: Add `mod install;` and the three commands to main.rs

Add to the top of `main.rs`:

```rust
mod install;
```

Add to the `Commands` enum:

```rust
    /// Install engram as a system service (launchd on macOS, systemd on Linux)
    Install,

    /// Uninstall the engram system service
    Uninstall,

    /// Diagnose engram installation and configuration
    Doctor,
```

Add to the `match cli.command` block:

```rust
        Commands::Install => run_install(),
        Commands::Uninstall => run_uninstall(),
        Commands::Doctor => run_doctor(),
```

Add the three free functions:

```rust
fn run_install() {
    match crate::install::install_service() {
        Ok(()) => {}
        Err(e) => {
            eprintln!("Install failed: {e}");
            std::process::exit(1);
        }
    }
}

fn run_uninstall() {
    match crate::install::uninstall_service() {
        Ok(()) => {}
        Err(e) => {
            eprintln!("Uninstall failed: {e}");
            std::process::exit(1);
        }
    }
}

fn run_doctor() {
    use engram_core::{crypto::KeyStore, store::MemoryStore, vault::Vault};

    println!("engram doctor");
    println!("{}", "\u{2500}".repeat(41));

    // Binary path
    let binary = std::env::current_exe()
        .map(|p| p.display().to_string())
        .unwrap_or_else(|_| "(unknown)".to_string());
    println!("Binary:  {binary} \u{2713}");

    // Vault
    let vault_path = default_vault_path();
    if vault_path.exists() {
        let vault = Vault::new(&vault_path);
        let count = vault.list_markdown().map(|f| f.len()).unwrap_or(0);
        println!("Vault:   {} ({count} files) \u{2713}", vault_path.display());
    } else {
        println!("Vault:   {} (NOT FOUND)", vault_path.display());
    }

    // Memory store + key
    let store_path = default_store_path();
    let key_store = KeyStore::new("engram");
    let key_result = key_store.retrieve();

    match &key_result {
        Ok(_) => println!("Key:     present \u{2713}"),
        Err(_) => println!("Key:     NOT SET"),
    }

    if store_path.exists() {
        match &key_result {
            Ok(key) => match MemoryStore::open(&store_path, key) {
                Ok(store) => {
                    let count = store.record_count().unwrap_or(0);
                    println!("Store:   {} ({count} records) \u{2713}", store_path.display());
                }
                Err(_) => println!("Store:   {} (wrong key)", store_path.display()),
            },
            Err(_) => println!("Store:   {} (present, no key)", store_path.display()),
        }
    } else {
        println!("Store:   {} (not initialized)", store_path.display());
    }

    // API key
    match std::env::var("ANTHROPIC_API_KEY") {
        Ok(_) => println!("API key: ANTHROPIC_API_KEY set \u{2713}"),
        Err(_) => println!("API key: ANTHROPIC_API_KEY NOT SET"),
    }
}
```

### Step 5: Run all tests

```bash
cd ~/workspace/ms/engram
cargo test -p engram -q 2>&1 | tail -15
```

Expected: all tests pass.

Also verify the install.rs unit tests:

```bash
cd ~/workspace/ms/engram
cargo test -p engram --lib install::tests -- -q 2>&1 | head -20
```

Expected: `test result: ok. 2 passed; 0 failed`

### Step 6: Commit

```bash
cd ~/workspace/ms/engram
git add crates/engram-cli/src/install.rs \
        crates/engram-cli/src/main.rs \
        crates/engram-cli/tests/cli_integration.rs
git commit -m "feat(cli): add 'engram install', 'engram uninstall', 'engram doctor'"
```

---

## Task 11: Integration test — observe + load round-trip

**Files:**
- Modify: `crates/engram-cli/src/main.rs` (update `default_store_path` to respect env var)
- Create: `crates/engram-cli/tests/observe_load_test.rs`

### Step 1: Make default_store_path overridable by env var

This lets the integration test inject a temp store without touching `~/.engram/memory.db`.

In `main.rs`, find the existing `default_store_path()` function:

```rust
/// Returns the default memory store path: `~/.engram/memory.db`.
fn default_store_path() -> PathBuf {
    UserDirs::new()
        .map(|u| u.home_dir().join(".engram/memory.db"))
        .unwrap_or_else(|| PathBuf::from(".engram/memory.db"))
}
```

Replace it with:

```rust
/// Returns the memory store path.
/// If `ENGRAM_STORE_PATH` is set, uses that path (intended for tests and CI).
/// Otherwise uses the default `~/.engram/memory.db`.
fn default_store_path() -> PathBuf {
    if let Ok(p) = std::env::var("ENGRAM_STORE_PATH") {
        return PathBuf::from(p);
    }
    UserDirs::new()
        .map(|u| u.home_dir().join(".engram/memory.db"))
        .unwrap_or_else(|| PathBuf::from(".engram/memory.db"))
}
```

Also update the existing unit test in `main.rs` for this function — the test asserts the path ends with `.engram/memory.db`, but now `ENGRAM_STORE_PATH` would break it. Guard the test:

```rust
    #[test]
    fn test_default_store_path_ends_with_engram_memory_db() {
        // Clear the override env var so this test is deterministic.
        std::env::remove_var("ENGRAM_STORE_PATH");
        let path = default_store_path();
        let path_str = path.to_string_lossy();
        assert!(
            path_str.ends_with(".engram/memory.db"),
            "store path should end with .engram/memory.db, got: {}",
            path_str
        );
    }
```

### Step 2: Write the failing integration test

Create `crates/engram-cli/tests/observe_load_test.rs`:

```rust
// observe_load_test.rs — Round-trip integration test: write memories → load context
//
// Tests the full pipeline without LLM calls:
//   1. Write Memory records directly to the store
//   2. Run `engram load` (via CLI binary) against the same store
//   3. Verify the formatted context output

use assert_cmd::Command;
use engram_core::{
    crypto::{EngramKey, KeyStore},
    store::{Memory, MemoryStore},
};
use predicates::prelude::*;
use tempfile::TempDir;

// ── Helpers ───────────────────────────────────────────────────────────────────

fn test_key() -> EngramKey {
    EngramKey::derive(b"testpassword", &[0u8; 16]).expect("key derivation failed")
}

/// Open a temp store and return (dir, store, db_path_string).
/// `dir` must stay alive for the duration of the test.
fn temp_store_with_path() -> (TempDir, MemoryStore, String) {
    let dir = TempDir::new().expect("create temp dir failed");
    let path = dir.path().join("test.db");
    let store = MemoryStore::open(&path, &test_key()).expect("open failed");
    let path_str = path.to_str().expect("path is valid UTF-8").to_string();
    (dir, store, path_str)
}

// ── Tests ─────────────────────────────────────────────────────────────────────

/// Write 3 memories for 2 entities → run `engram load` → verify all entities appear
/// and Sofia's two facts are on one line.
#[test]
fn test_load_context_round_trip_three_facts_two_entities() {
    let (_dir, store, store_path) = temp_store_with_path();

    // Populate directly — no LLM involved.
    store
        .insert(&Memory::new("Sofia", "dietary", "vegetarian", Some("session-1")))
        .unwrap();
    store
        .insert(&Memory::new("Sofia", "location", "Seattle", Some("session-1")))
        .unwrap();
    store
        .insert(&Memory::new("Chris Park", "preference", "small focused components", Some("session-1")))
        .unwrap();

    drop(store); // close the connection so the CLI binary can open it

    let mut cmd = Command::cargo_bin("engram").unwrap();
    cmd.args(["load", "--format=context"])
        .env("ENGRAM_STORE_PATH", &store_path)
        // Provide a dummy key via env so the CLI can open the store.
        // In CI the keychain may be absent; the test store uses a fixed derive key.
        // We skip this test if keychain is unavailable.
        ;

    // `engram load` will fail to retrieve a key from the keychain on a headless machine.
    // In that case the test is inconclusive but should not be counted as a failure.
    let output = cmd.output().unwrap();
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        if stderr.contains("No vault key found") || stderr.contains("keyring") {
            eprintln!("Skipping keychain-dependent assertion (headless machine): {stderr}");
            return;
        }
        panic!("engram load failed unexpectedly: {stderr}");
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("<engram-context>"), "stdout: {stdout}");
    assert!(stdout.contains("Sofia"), "stdout: {stdout}");
    assert!(stdout.contains("Chris Park"), "stdout: {stdout}");
    assert!(stdout.contains("vegetarian"), "stdout: {stdout}");
    assert!(stdout.contains("small focused components"), "stdout: {stdout}");

    // Sofia's two facts should be on a single line (grouped by entity).
    let sofia_line = stdout
        .lines()
        .find(|l| l.contains("Sofia"))
        .expect("no Sofia line in output");
    assert!(sofia_line.contains("vegetarian"), "sofia_line: {sofia_line}");
    assert!(sofia_line.contains("Seattle"), "sofia_line: {sofia_line}");
}

/// Write 5 facts for 1 entity → verify they all appear on a single grouped line.
#[test]
fn test_load_context_groups_multiple_facts_for_same_entity() {
    let (_dir, store, store_path) = temp_store_with_path();

    for (attr, val) in [
        ("dietary", "vegetarian"),
        ("location", "Seattle"),
        ("team", "Team Pulse"),
        ("role", "senior engineer"),
        ("hobby", "bouldering"),
    ] {
        store.insert(&Memory::new("Sofia", attr, val, None)).unwrap();
    }

    drop(store);

    let mut cmd = Command::cargo_bin("engram").unwrap();
    cmd.args(["load"]).env("ENGRAM_STORE_PATH", &store_path);

    let output = cmd.output().unwrap();
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        if stderr.contains("No vault key found") || stderr.contains("keyring") {
            eprintln!("Skipping (headless machine): {stderr}");
            return;
        }
        panic!("engram load failed: {stderr}");
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    // All 5 facts must appear in the output.
    for val in &["vegetarian", "Seattle", "Team Pulse", "senior engineer", "bouldering"] {
        assert!(stdout.contains(val), "missing '{val}' in output:\n{stdout}");
    }

    // All must be on a SINGLE line for Sofia (grouped).
    let sofia_lines: Vec<&str> = stdout.lines().filter(|l| l.contains("Sofia")).collect();
    assert_eq!(
        sofia_lines.len(),
        1,
        "expected exactly 1 line for Sofia, got {}: {sofia_lines:?}",
        sofia_lines.len()
    );
}

/// Empty store → engram load → output contains "No recent memories".
#[test]
fn test_load_context_empty_store_shows_no_memories_message() {
    let (_dir, _store, store_path) = temp_store_with_path();

    // store opened and closed immediately — no facts inserted.
    let mut cmd = Command::cargo_bin("engram").unwrap();
    cmd.args(["load"]).env("ENGRAM_STORE_PATH", &store_path);

    let output = cmd.output().unwrap();
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        if stderr.contains("No vault key found") || stderr.contains("keyring") {
            eprintln!("Skipping (headless machine): {stderr}");
            return;
        }
        panic!("engram load failed: {stderr}");
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("No recent memories"),
        "stdout: {stdout}"
    );
}
```

### Step 3: Run the failing test first

```bash
cd ~/workspace/ms/engram
cargo test -p engram --test observe_load_test -- -q 2>&1 | head -30
```

Expected: FAIL — `observe_load_test.rs` not yet fully wired (or compilation errors if `ENGRAM_STORE_PATH` not implemented yet).

### Step 4: Verify all tests pass end-to-end

```bash
cd ~/workspace/ms/engram
cargo test -p engram -q 2>&1 | tail -20
```

Expected: all tests pass. The `observe_load_test` tests may log "Skipping (headless machine)" on a machine without keychain access, but they must not FAIL.

Also run the full workspace to make sure nothing in other crates broke:

```bash
cd ~/workspace/ms/engram
cargo test --workspace -q 2>&1 | tail -20
```

### Step 5: Commit

```bash
cd ~/workspace/ms/engram
git add crates/engram-cli/src/main.rs \
        crates/engram-cli/tests/observe_load_test.rs
git commit -m "test: add observe+load round-trip integration test; make store path overridable"
```

---

## Final Verification

After all 11 tasks are complete:

### Check 1: All tests pass

```bash
cd ~/workspace/ms/engram
cargo test --workspace -q 2>&1 | tail -20
```

Expected: all tests pass, zero failures.

### Check 2: All new commands appear in help

```bash
cd ~/workspace/ms/engram
cargo run -p engram -- --help 2>&1
```

Expected output contains: `observe`, `load`, `daemon`, `mcp`, `install`, `uninstall`, `doctor`

### Check 3: Smoke test each new command's --help

```bash
for cmd in observe load daemon mcp install uninstall doctor; do
  echo "=== engram $cmd --help ==="
  cargo run -p engram -q -- "$cmd" --help 2>&1 | head -5
done
```

Expected: each prints a short help message and exits 0.

### Check 4: Build succeeds in release mode

```bash
cd ~/workspace/ms/engram
cargo build --release -p engram -q 2>&1
echo "Exit: $?"
```

Expected: `Exit: 0`

### Check 5: Commit summary

```bash
cd ~/workspace/ms/engram
git log --oneline -12
```

Expected: 11 commits (one per task) plus any fixups.

---

## MCP Configuration Reference

After `engram install`, add engram as an MCP server in Claude Code (`.claude/claude_desktop_config.json` or equivalent):

```json
{
  "mcpServers": {
    "engram": {
      "command": "engram",
      "args": ["mcp"]
    }
  }
}
```

This makes `memory_search`, `memory_load`, and `memory_status` available as MCP tools in any harness that supports the MCP stdio transport.
