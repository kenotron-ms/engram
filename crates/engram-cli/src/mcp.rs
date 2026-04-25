// mcp.rs — MCP stdio server (JSON-RPC 2.0 over stdin/stdout)
//
// Implements the Model Context Protocol (https://spec.modelcontextprotocol.io/)
// exposing three memory tools: memory_search, memory_load, memory_status.

use std::io::{self, BufRead, Write};

use engram_core::store::{MemoryStore, StoreError};
use serde_json::{json, Value};
use thiserror::Error;

/// Errors that can occur during MCP server operation.
#[derive(Debug, Error)]
pub enum McpError {
    #[error("IO error: {0}")]
    Io(#[from] io::Error),
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),
    #[error("store error: {0}")]
    Store(#[from] StoreError),
}

/// Returns the JSON array of 3 MCP tool definitions.
///
/// Tools:
/// - `memory_search`: query (required string), limit (optional number)
/// - `memory_load`:   format (optional string enum: context/facts/summary)
/// - `memory_status`: no parameters
pub fn tool_definitions() -> Value {
    json!([
        {
            "name": "memory_search",
            "description": "Search memories by query string",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "query": {
                        "type": "string",
                        "description": "Search query string"
                    },
                    "limit": {
                        "type": "number",
                        "description": "Maximum number of results to return"
                    }
                },
                "required": ["query"]
            }
        },
        {
            "name": "memory_load",
            "description": "Load recent memories as an AI context block",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "format": {
                        "type": "string",
                        "enum": ["context", "facts", "summary"],
                        "description": "Output format (default: context)"
                    }
                }
            }
        },
        {
            "name": "memory_status",
            "description": "Get memory store status and record count",
            "inputSchema": {
                "type": "object",
                "properties": {}
            }
        }
    ])
}

/// Build a JSON-RPC 2.0 error response.
pub fn make_error(id: &Value, code: i64, message: &str) -> Value {
    json!({
        "jsonrpc": "2.0",
        "id": id,
        "error": {
            "code": code,
            "message": message
        }
    })
}

/// Build a successful tool result response with plain-text content.
pub fn make_tool_result(id: &Value, text: &str) -> Value {
    json!({
        "jsonrpc": "2.0",
        "id": id,
        "result": {
            "content": [
                {
                    "type": "text",
                    "text": text
                }
            ]
        }
    })
}

/// Handle the `initialize` method — return protocol version and server info.
pub fn handle_initialize(id: &Value) -> Value {
    json!({
        "jsonrpc": "2.0",
        "id": id,
        "result": {
            "protocolVersion": "2024-11-05",
            "capabilities": {
                "tools": {}
            },
            "serverInfo": {
                "name": "engram",
                "version": "0.1.0"
            }
        }
    })
}

/// Handle the `tools/list` method — return the full tool array.
pub fn handle_tools_list(id: &Value) -> Value {
    json!({
        "jsonrpc": "2.0",
        "id": id,
        "result": {
            "tools": tool_definitions()
        }
    })
}

/// Handle the `tools/call` method — dispatch by tool name.
///
/// - `memory_search` — calls `store.search(query)`, formats results
/// - `memory_load`   — calls `crate::load::load_context(store)`
/// - `memory_status` — returns `record_count`
/// - unknown tool    — returns -32602 error
pub fn handle_tools_call(id: &Value, params: &Value, store: &MemoryStore) -> Value {
    let name = match params.get("name").and_then(|n| n.as_str()) {
        Some(n) => n,
        None => return make_error(id, -32602, "missing tool name"),
    };

    match name {
        "memory_search" => {
            // Extract the required `query` argument; reject empty strings.
            let query = match params
                .get("arguments")
                .and_then(|a| a.get("query"))
                .and_then(|q| q.as_str())
            {
                Some(q) if !q.is_empty() => q.to_string(),
                Some(_) => return make_error(id, -32602, "query cannot be empty"),
                None => return make_error(id, -32602, "missing required parameter: query"),
            };

            match store.search(&query) {
                Ok(results) => {
                    let text = if results.is_empty() {
                        "No results found.".to_string()
                    } else {
                        results
                            .iter()
                            .map(|m| format!("{}: {} = {}", m.entity, m.attribute, m.value))
                            .collect::<Vec<_>>()
                            .join("\n")
                    };
                    make_tool_result(id, &text)
                }
                Err(e) => make_error(id, -32603, &format!("store error: {}", e)),
            }
        }

        "memory_load" => match crate::load::load_context(store) {
            Ok(text) => make_tool_result(id, &text),
            Err(e) => make_error(id, -32603, &format!("load error: {}", e)),
        },

        "memory_status" => match store.record_count() {
            Ok(count) => make_tool_result(id, &format!("record_count: {}", count)),
            Err(e) => make_error(id, -32603, &format!("store error: {}", e)),
        },

        _ => make_error(id, -32602, &format!("unknown tool: {}", name)),
    }
}

/// Dispatch a parsed JSON-RPC 2.0 request to the appropriate handler.
///
/// Method routing:
/// - `initialize`  → `handle_initialize`
/// - `tools/list`  → `handle_tools_list`
/// - `tools/call`  → `handle_tools_call`
/// - unknown       → -32601 error
pub fn handle_request(request: &Value, store: &MemoryStore) -> Value {
    let id = request.get("id").unwrap_or(&Value::Null);
    let method = match request.get("method").and_then(|m| m.as_str()) {
        Some(m) => m,
        None => return make_error(id, -32600, "invalid request: missing method"),
    };

    match method {
        "initialize" => handle_initialize(id),
        "tools/list" => handle_tools_list(id),
        "tools/call" => {
            let params = match request.get("params") {
                Some(p) => p,
                None => return make_error(id, -32602, "missing params for tools/call"),
            };
            handle_tools_call(id, params, store)
        }
        _ => make_error(id, -32601, &format!("method not found: {}", method)),
    }
}

/// Run the MCP stdio server.
///
/// Reads newline-delimited JSON-RPC 2.0 requests from stdin, dispatches each
/// to `handle_request`, and writes the JSON response followed by a newline to
/// stdout (flushed after every response).  Malformed JSON lines produce a
/// -32700 parse-error response.  The loop exits when stdin is closed.
pub fn run_mcp_server(store: &MemoryStore) -> Result<(), McpError> {
    let stdin = io::stdin();
    let stdout = io::stdout();
    let mut out = stdout.lock();

    for line in stdin.lock().lines() {
        let line = line?;
        if line.trim().is_empty() {
            continue;
        }

        let response = match serde_json::from_str::<Value>(&line) {
            Ok(request) => handle_request(&request, store),
            Err(_) => make_error(&Value::Null, -32700, "parse error"),
        };

        writeln!(out, "{}", serde_json::to_string(&response)?)?;
        out.flush()?;
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use engram_core::crypto::EngramKey;
    use engram_core::store::{Memory, MemoryStore};
    use serde_json::json;
    use tempfile::TempDir;

    fn test_key() -> EngramKey {
        EngramKey::derive(b"testpassword", &[0u8; 16]).expect("key derivation failed")
    }

    fn temp_store() -> (TempDir, MemoryStore) {
        let dir = TempDir::new().expect("create temp dir failed");
        let path = dir.path().join("test.db");
        let store = MemoryStore::open(&path, &test_key()).expect("open store failed");
        (dir, store)
    }

    /// tool_definitions() must return exactly 3 tools with correct names.
    #[test]
    fn test_tools_list_returns_three_tools() {
        let tools = tool_definitions();
        let arr = tools
            .as_array()
            .expect("tool_definitions should return a JSON array");
        assert_eq!(
            arr.len(),
            3,
            "should return exactly 3 tools, got {}",
            arr.len()
        );

        let names: Vec<&str> = arr
            .iter()
            .filter_map(|t| t.get("name").and_then(|n| n.as_str()))
            .collect();
        assert!(
            names.contains(&"memory_search"),
            "tools should include 'memory_search', got: {:?}",
            names
        );
        assert!(
            names.contains(&"memory_load"),
            "tools should include 'memory_load', got: {:?}",
            names
        );
        assert!(
            names.contains(&"memory_status"),
            "tools should include 'memory_status', got: {:?}",
            names
        );
    }

    /// handle_initialize must return protocolVersion '2024-11-05'.
    #[test]
    fn test_initialize_returns_protocol_version() {
        let id = json!(1);
        let response = handle_initialize(&id);
        let version = response["result"]["protocolVersion"]
            .as_str()
            .expect("should have a string protocolVersion field");
        assert_eq!(
            version, "2024-11-05",
            "protocolVersion should be '2024-11-05', got: {}",
            version
        );
    }

    /// Dispatching an unknown method must return JSON-RPC error code -32601.
    #[test]
    fn test_unknown_method_returns_error_32601() {
        let (_dir, store) = temp_store();
        let request = json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "unknown/method"
        });
        let response = handle_request(&request, &store);
        let code = response["error"]["code"]
            .as_i64()
            .expect("unknown method should produce an error.code field");
        assert_eq!(
            code, -32601,
            "unknown method should return error code -32601, got: {}",
            code
        );
    }

    /// memory_status tool must include the current record count.
    #[test]
    fn test_memory_status_returns_record_count() {
        let (_dir, store) = temp_store();
        let memory = Memory::new("Sofia", "dietary", "vegetarian", None);
        store.insert(&memory).expect("insert failed");

        let request = json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "tools/call",
            "params": {
                "name": "memory_status",
                "arguments": {}
            }
        });
        let response = handle_request(&request, &store);
        let text = response["result"]["content"][0]["text"]
            .as_str()
            .expect("memory_status should return text content");
        assert!(
            text.contains("record_count: 1"),
            "status text should contain 'record_count: 1', got: {}",
            text
        );
    }

    /// memory_search with an empty query string must return an error.
    #[test]
    fn test_memory_search_empty_query_returns_error() {
        let (_dir, store) = temp_store();
        let request = json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "tools/call",
            "params": {
                "name": "memory_search",
                "arguments": {
                    "query": ""
                }
            }
        });
        let response = handle_request(&request, &store);
        assert!(
            response.get("error").is_some(),
            "empty query should return an error response, got: {}",
            response
        );
    }

    /// memory_search must find a fact that was previously inserted.
    #[test]
    fn test_memory_search_finds_inserted_fact() {
        let (_dir, store) = temp_store();
        let memory = Memory::new("Sofia", "dietary", "vegetarian", None);
        store.insert(&memory).expect("insert failed");

        let request = json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "tools/call",
            "params": {
                "name": "memory_search",
                "arguments": {
                    "query": "vegetarian"
                }
            }
        });
        let response = handle_request(&request, &store);
        let text = response["result"]["content"][0]["text"]
            .as_str()
            .expect("memory_search should return text content");
        assert!(
            text.contains("vegetarian"),
            "search result should contain 'vegetarian', got: {}",
            text
        );
    }
}
