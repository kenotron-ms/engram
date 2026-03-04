# SPEC-TOOLS: Tools API Specification

> engram-lite tool definitions — MCP tools and Amplifier tool modules
> Version: 0.1.0 | Status: Draft

---

## Table of Contents

1. [Tool Architecture](#1-tool-architecture)
2. [Tool Contract](#2-tool-contract)
3. [Tool Reference](#3-tool-reference)
   - [memory_capture](#31-memory_capture)
   - [memory_recall](#32-memory_recall)
   - [memory_search](#33-memory_search)
   - [memory_update](#34-memory_update)
   - [memory_relate](#35-memory_relate)
   - [memory_forget](#36-memory_forget)
   - [memory_graph_explore](#37-memory_graph_explore)
   - [memory_stats](#38-memory_stats)
   - [memory_index](#39-memory_index)
4. [Capture Decision Tree](#4-capture-decision-tree)
5. [Retrieve Decision Tree](#5-retrieve-decision-tree)
6. [Cross-Reference Cascade](#6-cross-reference-cascade)
7. [Tool Composition Patterns](#7-tool-composition-patterns)

---

## 1. Tool Architecture

engram-lite tools are exposed through two interfaces:

### MCP Server (Claude Code)

Tools are served via the Model Context Protocol over stdio transport. Claude Code discovers them through MCP server registration:

```json
{
    "mcpServers": {
        "engram-lite": {
            "command": "engram-lite",
            "args": ["mcp-server"],
            "env": {
                "ENGRAM_USER_DB": "~/.engram-lite/user.db",
                "ENGRAM_PROJECT_DB": ".engram-lite/project.db"
            }
        }
    }
}
```

Each tool maps to an MCP `tools/call` handler with JSON Schema input validation.

### Amplifier Tool Module

Tools are registered as an Amplifier tool module via a Python package:

```python
# amplifier_module_engram_lite/amplifier_tools.py
from amplifier.tools import ToolModule, tool

class CanvasMemoryTools(ToolModule):
    name = "engram-lite"
    description = "Persistent memory system for AI agents"

    @tool
    def memory_capture(self, content: str, **kwargs) -> dict: ...

    @tool
    def memory_recall(self, query: str, **kwargs) -> dict: ...

    # ... etc
```

### Shared Implementation

Both interfaces delegate to the same core library:

```
┌──────────────────┐     ┌──────────────────┐
│  MCP Server       │     │  Amplifier Module  │
│  (stdio JSON-RPC) │     │  (Python API)      │
└────────┬─────────┘     └────────┬─────────┘
         │                        │
         └──────────┬─────────────┘
                    │
            ┌───────▼───────┐
            │  Core Library   │
            │  amplifier_module_engram_lite  │
            │  .core          │
            └───────┬───────┘
                    │
         ┌──────────┼──────────┐
         │          │          │
    ┌────▼───┐ ┌───▼────┐ ┌──▼─────┐
    │Retrieval│ │ Graph  │ │Embedding│
    │ Engine  │ │ Engine │ │ Engine  │
    └────┬───┘ └───┬────┘ └──┬─────┘
         │         │         │
         └─────────┼─────────┘
                   │
            ┌──────▼──────┐
            │   SQLite     │
            │  + sqlite-vec│
            └─────────────┘
```

---

## 2. Tool Contract

### Input Validation

All tools validate inputs against JSON Schema before execution. Invalid inputs return a structured error:

```json
{
    "error": {
        "code": "INVALID_INPUT",
        "message": "Parameter 'importance' must be one of: critical, high, medium, low",
        "param": "importance",
        "received": "urgent"
    }
}
```

### Error Response Format

All errors follow a consistent structure:

```json
{
    "error": {
        "code": "ERROR_CODE",
        "message": "Human-readable description",
        "param": "optional_field_name",
        "received": "optional_invalid_value"
    }
}
```

| Error Code | HTTP Analog | When |
|---|---|---|
| `INVALID_INPUT` | 400 | Schema validation failure |
| `NOT_FOUND` | 404 | Memory ID or node ID doesn't exist |
| `CONFLICT` | 409 | Duplicate relation, contradicting update |
| `DB_ERROR` | 500 | SQLite error (locked, corrupt, etc.) |
| `EMBEDDING_ERROR` | 502 | Embedding model unavailable or failed |

### Access Timestamp Update

Every successful `memory_recall` and `memory_search` that returns a memory updates its `accessed_at` timestamp:

```sql
UPDATE memories SET accessed_at = datetime('now')
WHERE memory_id IN (:returned_memory_ids);
```

This is critical for recency decay scoring and session pre-loading.

---

## 3. Tool Reference

---

### 3.1 `memory_capture`

**Purpose:** Store a new memory in the engram-lite system.

**When the AI should use this tool:**
- The user shares a preference, decision, or important fact
- A new piece of project knowledge is discovered during conversation
- A correction is made to previously held information
- A pattern is observed for the second or more time
- The user explicitly asks the AI to "remember" something

> **Side effect:** Capturing automatically updates the appropriate `MEMORY.md` index file (user, project, or local scope). No separate call is needed — the return value includes the generated entry.

#### Input Schema

```json
{
    "type": "object",
    "properties": {
        "content": {
            "type": "string",
            "description": "The information to memorize. Will be summarized and stored.",
            "minLength": 1,
            "maxLength": 10000
        },
        "content_type": {
            "type": "string",
            "enum": ["fact", "preference", "event", "skill", "entity", "relationship", "decision"],
            "default": "fact",
            "description": "Category of information being stored."
        },
        "tags": {
            "type": "array",
            "items": {"type": "string", "maxLength": 50},
            "default": [],
            "maxItems": 20,
            "description": "Searchable tags for categorization."
        },
        "domain": {
            "type": "string",
            "maxLength": 200,
            "description": "Hierarchical domain path (e.g., 'project/backend/auth'). Inferred if not provided."
        },
        "space": {
            "type": "string",
            "enum": ["user", "project", "local"],
            "default": "user",
            "description": "'user' = persists across projects (~/.engram/MEMORY.md); 'project' = scoped to current project (.engram/MEMORY.md); 'local' = machine-specific, gitignored (.engram/MEMORY.local.md)."
        },
        "importance": {
            "type": "string",
            "enum": ["critical", "high", "medium", "low"],
            "default": "medium",
            "description": "How important this memory is for future recall."
        },
        "relates_to": {
            "type": "array",
            "items": {"type": "string"},
            "default": [],
            "description": "Memory IDs that this new memory relates to."
        },
        "expires_in_days": {
            "type": "integer",
            "minimum": 1,
            "description": "Number of days until this memory expires. Null = permanent."
        }
    },
    "required": ["content"]
}
```

#### Return Schema

```json
{
    "type": "object",
    "properties": {
        "memory_id":      {"type": "string", "description": "Unique identifier for the stored memory."},
        "summary":        {"type": "string", "description": "Generated summary of the stored content."},
        "domain":         {"type": "string", "description": "Assigned domain (provided or inferred)."},
        "tags":           {"type": "array", "items": {"type": "string"}},
        "keywords_count":    {"type": "integer", "description": "Number of extracted keywords for FTS."},
        "memory_md_entry":   {"type": "string", "description": "Compressed entry line written to MEMORY.md (≤100 chars)."},
        "memory_md_file":    {"type": "string", "description": "Path of the MEMORY.md file that was updated."}
    },
    "required": ["memory_id", "summary", "domain", "tags", "keywords_count", "memory_md_entry", "memory_md_file"]
}
```

#### Error Cases

| Error Code | Condition |
|---|---|
| `INVALID_INPUT` | `content` is empty or exceeds 10,000 chars |
| `INVALID_INPUT` | `content_type` not in allowed enum |
| `INVALID_INPUT` | `importance` not in allowed enum |
| `NOT_FOUND` | A `memory_id` in `relates_to` does not exist |
| `EMBEDDING_ERROR` | Embedding model failed to encode the content |
| `DB_ERROR` | SQLite write failure |

#### Example

**Call:**
```json
{
    "tool": "memory_capture",
    "arguments": {
        "content": "The user prefers Rust over Go for systems programming. They mentioned this is because of the borrow checker providing memory safety guarantees without garbage collection overhead.",
        "content_type": "preference",
        "tags": ["rust", "go", "programming-language", "systems"],
        "domain": "user/preferences/languages",
        "space": "user",
        "importance": "high"
    }
}
```

**Response:**
```json
{
    "memory_id": "mem_7f3a2b1c",
    "summary": "User prefers Rust over Go for systems programming due to borrow checker providing memory safety without GC overhead.",
    "domain": "user/preferences/languages",
    "tags": ["rust", "go", "programming-language", "systems"],
    "keywords_count": 8,
    "memory_md_entry": "- [pref] Rust over Go for systems programming — borrow checker, no GC",
    "memory_md_file": "~/.engram/MEMORY.md"
}
```

#### Performance

- Expected latency: <100ms (embedding: ~50ms, DB write: ~10ms, FTS update: ~10ms, graph update: ~20ms)
- Embedding is the bottleneck; batched captures should be considered for bulk imports

#### Capture Pipeline

The full internal pipeline (8 steps):

1. Validate input (`content_type`, `space`, `domain`)
2. Generate summary (LLM call — inductive, conclusion-first, ≤200 chars)
3. Extract tags and keywords (LLM call)
4. Generate embedding (provider call)
5. Write to DB (`INSERT` into `memories` + `memory_vectors` + `memory_tags` + `memory_fts`)
6. Run cross-reference cascade (find relations, update graph nodes)
7. **Update MEMORY.md** (generate entry, append to section, prune if needed)
8. Return `{memory_id, summary, domain, tags, memory_md_entry, memory_md_file}`

#### MEMORY.md Integration (Step 7)

After writing to the DB, `memory_capture` generates a compressed entry line and appends it to the appropriate MEMORY.md file:

| `space` | File | Scope |
|---|---|---|
| `'user'` | `~/.engram/MEMORY.md` | Cross-project, follows the user |
| `'project'` | `.engram/MEMORY.md` | Project-scoped, committed to repo |
| `'local'` | `.engram/MEMORY.local.md` | Machine-specific, gitignored |

**Entry generation rule:** Convert `summary` → single MEMORY.md line, ≤100 chars. Actionable conclusion first. Strip evidence.

```
memory.summary = "User prefers inductive writing — conclusion-first, evidence below."
→ MEMORY.md entry: - [pref] Inductive writing (conclusion-first) — applies everywhere
```

**Content-type → entry type mapping:**

| `content_type` | MEMORY.md `[type]` tag | Section |
|---|---|---|
| `preference` | `[pref]` | `## You` (user) |
| `fact` | `[arch]` or `[status]` | `## Project` (project) |
| `decision` | `[decision]` | `## Project` (project) |
| `skill` | `[skill]` | `## You` (user) |
| `entity` (person) | `[person]` | `## You` (user) |
| `relationship` | `[pattern]` | whichever space matches |
| `event` | `[event]` | `## Now` (refreshed, not appended) |
| `constraint` | `[constraint]` | `## You` (user) |

**Pruning:** After adding the new entry, if the target section exceeds 60 entries, the entry with the lowest `confidence × importance_weight` score is removed from MEMORY.md (it remains in the DB).

---

### 3.2 `memory_recall`

**Purpose:** Retrieve relevant memories using the dual-route retrieval system.

**When the AI should use this tool:**
- The user asks a question that might be answered by prior context
- The conversation touches a topic where the user has stated preferences
- Before suggesting an approach, check if there are relevant decisions on record
- When starting work in a domain, recall what's known about it
- When the user references something discussed previously

#### Input Schema

```json
{
    "type": "object",
    "properties": {
        "query": {
            "type": "string",
            "description": "Natural language query describing what to recall.",
            "minLength": 1,
            "maxLength": 1000
        },
        "route": {
            "type": "string",
            "enum": ["auto", "vector", "graph", "hybrid", "keyword"],
            "default": "auto",
            "description": "Retrieval route. 'auto' selects based on query analysis."
        },
        "limit": {
            "type": "integer",
            "minimum": 1,
            "maximum": 50,
            "default": 5,
            "description": "Maximum number of memories to return."
        },
        "domain": {
            "type": "string",
            "description": "Restrict results to this domain subtree."
        },
        "space": {
            "type": "string",
            "enum": ["user", "project"],
            "description": "Restrict to space. Null = search both."
        },
        "min_confidence": {
            "type": "number",
            "minimum": 0.0,
            "maximum": 1.0,
            "default": 0.5,
            "description": "Minimum confidence threshold."
        },
        "content_types": {
            "type": "array",
            "items": {
                "type": "string",
                "enum": ["fact", "preference", "event", "skill", "entity", "relationship", "decision"]
            },
            "default": [],
            "description": "Filter by content type. Empty = all types."
        },
        "include_detail": {
            "type": "boolean",
            "default": false,
            "description": "Include full content from cold tier (detail field)."
        }
    },
    "required": ["query"]
}
```

#### Return Schema

```json
{
    "type": "array",
    "items": {
        "type": "object",
        "properties": {
            "memory_id":    {"type": "string"},
            "summary":      {"type": "string"},
            "detail":       {"type": "string", "description": "Only present if include_detail=true."},
            "domain":       {"type": "string"},
            "tags":         {"type": "array", "items": {"type": "string"}},
            "content_type": {"type": "string"},
            "confidence":   {"type": "number"},
            "importance":   {"type": "string"},
            "score":        {"type": "number", "description": "Retrieval relevance score."},
            "created_at":   {"type": "string", "format": "date-time"},
            "accessed_at":  {"type": "string", "format": "date-time"},
            "relations":    {
                "type": "array",
                "items": {
                    "type": "object",
                    "properties": {
                        "relation_type": {"type": "string"},
                        "related_id":    {"type": "string"},
                        "direction":     {"type": "string", "enum": ["from", "to"]}
                    }
                },
                "description": "Related memories (always included for cross-reference context)."
            }
        },
        "required": ["memory_id", "summary", "domain", "confidence", "importance", "score"]
    }
}
```

#### Error Cases

| Error Code | Condition |
|---|---|
| `INVALID_INPUT` | `query` is empty or exceeds 1,000 chars |
| `INVALID_INPUT` | `route` not in allowed enum |
| `INVALID_INPUT` | `limit` out of range |
| `EMBEDDING_ERROR` | Embedding model failed |
| `DB_ERROR` | SQLite read failure |

#### Example

**Call:**
```json
{
    "tool": "memory_recall",
    "arguments": {
        "query": "What are the user's programming language preferences?",
        "route": "auto",
        "limit": 3,
        "content_types": ["preference"],
        "space": "user"
    }
}
```

**Response:**
```json
[
    {
        "memory_id": "mem_7f3a2b1c",
        "summary": "User prefers Rust over Go for systems programming due to borrow checker providing memory safety without GC overhead.",
        "domain": "user/preferences/languages",
        "tags": ["rust", "go", "programming-language", "systems"],
        "content_type": "preference",
        "confidence": 0.95,
        "importance": "high",
        "score": 0.0312,
        "created_at": "2025-01-15T10:30:00Z",
        "accessed_at": "2025-02-20T14:22:00Z",
        "relations": []
    },
    {
        "memory_id": "mem_2c4d6e8f",
        "summary": "User prefers TypeScript over JavaScript for all new frontend and backend code.",
        "domain": "user/preferences/languages",
        "tags": ["typescript", "javascript", "frontend", "backend"],
        "content_type": "preference",
        "confidence": 0.90,
        "importance": "high",
        "score": 0.0287,
        "created_at": "2025-01-10T08:15:00Z",
        "accessed_at": "2025-02-18T09:45:00Z",
        "relations": [
            {"relation_type": "relates-to", "related_id": "mem_9a8b7c6d", "direction": "from"}
        ]
    }
]
```

#### Performance

| Route | Expected Latency (10k memories) |
|---|---|
| `vector` | <50ms |
| `keyword` | <30ms |
| `graph` | <200ms |
| `hybrid` | <200ms (parallel) |
| `auto` | <5ms analysis + route latency |

---

### 3.3 `memory_search`

**Purpose:** Simplified search interface without route logic. Always uses System-1 (vector + BM25) with basic filtering.

**When the AI should use this tool:**
- Quick lookup when the full `memory_recall` options aren't needed
- Programmatic search (e.g., finding all memories in a domain)
- When building a list of memories to pass to other tools

#### Input Schema

```json
{
    "type": "object",
    "properties": {
        "query": {
            "type": "string",
            "description": "Search query.",
            "minLength": 1,
            "maxLength": 1000
        },
        "domain": {
            "type": "string",
            "description": "Restrict to domain subtree."
        },
        "limit": {
            "type": "integer",
            "minimum": 1,
            "maximum": 100,
            "default": 10,
            "description": "Maximum results."
        },
        "filters": {
            "type": "object",
            "properties": {
                "space":         {"type": "string", "enum": ["user", "project"]},
                "content_type":  {"type": "string"},
                "importance":    {"type": "string"},
                "min_confidence": {"type": "number"},
                "created_after": {"type": "string", "format": "date-time"},
                "created_before": {"type": "string", "format": "date-time"},
                "tags":          {"type": "array", "items": {"type": "string"}}
            },
            "description": "Additional filter criteria."
        }
    },
    "required": ["query"]
}
```

#### Return Schema

```json
{
    "type": "array",
    "items": {
        "type": "object",
        "properties": {
            "memory_id":    {"type": "string"},
            "summary":      {"type": "string"},
            "domain":       {"type": "string"},
            "tags":         {"type": "array", "items": {"type": "string"}},
            "content_type": {"type": "string"},
            "confidence":   {"type": "number"},
            "importance":   {"type": "string"},
            "score":        {"type": "number"}
        },
        "required": ["memory_id", "summary", "score"]
    }
}
```

#### Error Cases

| Error Code | Condition |
|---|---|
| `INVALID_INPUT` | `query` is empty |
| `INVALID_INPUT` | `filters` contains unknown keys |
| `DB_ERROR` | SQLite failure |

#### Example

**Call:**
```json
{
    "tool": "memory_search",
    "arguments": {
        "query": "deployment configuration",
        "domain": "project/backend",
        "limit": 5,
        "filters": {
            "importance": "high",
            "content_type": "decision"
        }
    }
}
```

**Response:**
```json
[
    {
        "memory_id": "mem_abc123",
        "summary": "Decided to use Kubernetes with Helm charts for all backend deployments.",
        "domain": "project/backend/deployment",
        "tags": ["kubernetes", "helm", "deployment"],
        "content_type": "decision",
        "confidence": 0.95,
        "importance": "high",
        "score": 0.0298
    }
]
```

#### Performance

- Expected latency: <50ms (always uses System-1)

---

### 3.4 `memory_update`

**Purpose:** Modify an existing memory's metadata or content.

**When the AI should use this tool:**
- Information becomes more or less certain (adjust confidence)
- A memory needs correction (update content/summary)
- Tags need to be added or refined
- Importance has changed (e.g., a feature becomes critical)
- Additional detail should be attached to an existing memory

#### Input Schema

```json
{
    "type": "object",
    "properties": {
        "memory_id": {
            "type": "string",
            "description": "ID of the memory to update."
        },
        "content": {
            "type": "string",
            "maxLength": 10000,
            "description": "New content. Triggers re-summarization and re-embedding."
        },
        "summary": {
            "type": "string",
            "maxLength": 500,
            "description": "Directly set the summary (skips LLM summarization)."
        },
        "tags": {
            "type": "array",
            "items": {"type": "string"},
            "description": "New tag list. Null = no change; empty array = clear all tags."
        },
        "importance": {
            "type": "string",
            "enum": ["critical", "high", "medium", "low"],
            "description": "New importance level."
        },
        "confidence": {
            "type": "number",
            "minimum": 0.0,
            "maximum": 1.0,
            "description": "New confidence score."
        },
        "detail": {
            "type": "string",
            "maxLength": 50000,
            "description": "Set or replace the cold-tier detail content."
        }
    },
    "required": ["memory_id"]
}
```

#### Return Schema

```json
{
    "type": "object",
    "properties": {
        "success":      {"type": "boolean"},
        "memory_id":    {"type": "string"},
        "changes_made": {
            "type": "array",
            "items": {"type": "string"},
            "description": "List of fields that were changed."
        }
    },
    "required": ["success", "memory_id", "changes_made"]
}
```

#### Error Cases

| Error Code | Condition |
|---|---|
| `NOT_FOUND` | `memory_id` does not exist |
| `INVALID_INPUT` | No update fields provided (at least one required besides `memory_id`) |
| `INVALID_INPUT` | `importance` not in allowed enum |
| `INVALID_INPUT` | `confidence` out of range |
| `EMBEDDING_ERROR` | Re-embedding failed (when `content` is updated) |
| `DB_ERROR` | SQLite write failure |

#### Example

**Call:**
```json
{
    "tool": "memory_update",
    "arguments": {
        "memory_id": "mem_7f3a2b1c",
        "confidence": 1.0,
        "tags": ["rust", "go", "systems-programming", "language-preference", "borrow-checker"],
        "importance": "critical"
    }
}
```

**Response:**
```json
{
    "success": true,
    "memory_id": "mem_7f3a2b1c",
    "changes_made": ["confidence: 0.95 → 1.0", "tags: updated (5 tags)", "importance: high → critical"]
}
```

#### Side Effects

When `content` is updated:
1. Summary is regenerated via LLM
2. Embedding is recomputed and updated in `memory_vectors`
3. FTS index is updated in `memory_fts`
4. `updated_at` timestamp is set
5. Graph node assignments may change if domain shifts
6. **MEMORY.md is updated:** The old summary is fuzzy-matched in the corresponding MEMORY.md file and replaced with the new compressed entry. If `importance` changed to `'low'`, the entry is removed from MEMORY.md (it stays in DB).

#### Performance

- Metadata-only update: <20ms
- Content update (re-embed + MEMORY.md): <120ms

---

### 3.5 `memory_relate`

**Purpose:** Create a typed, weighted relation between two memories.

**When the AI should use this tool:**
- A new memory logically connects to an existing one
- Two memories contradict each other (critical to record)
- A newer memory supersedes an older one
- A memory is part of a larger concept
- A decision was made in a specific context

#### Input Schema

```json
{
    "type": "object",
    "properties": {
        "from_id": {
            "type": "string",
            "description": "Source memory ID."
        },
        "to_id": {
            "type": "string",
            "description": "Target memory ID."
        },
        "relation_type": {
            "type": "string",
            "enum": [
                "relates-to", "supports", "contradicts", "supersedes",
                "exemplifies", "part-of", "caused-by", "decided-in", "applies-to"
            ],
            "description": "The type of relationship."
        },
        "strength": {
            "type": "number",
            "minimum": 0.0,
            "maximum": 1.0,
            "default": 0.5,
            "description": "Strength of the relationship. 1.0 = strongest."
        }
    },
    "required": ["from_id", "to_id", "relation_type"]
}
```

#### Relation Type Semantics

| Type | Meaning | Directionality | Example |
|---|---|---|---|
| `relates-to` | General association | Symmetric | Two memories about the same feature |
| `supports` | Evidence / reinforcement | A supports B | A test result supporting a design decision |
| `contradicts` | Conflicting information | Symmetric | Two incompatible facts |
| `supersedes` | A replaces B | A supersedes B | Updated config replacing old config |
| `exemplifies` | A is an example of B | A exemplifies B | A specific case of a general pattern |
| `part-of` | A is part of B | A part-of B | A function being part of a module |
| `caused-by` | A was caused by B | A caused-by B | A bug caused by a config change |
| `decided-in` | A was decided in context B | A decided-in B | A decision made during a meeting |
| `applies-to` | A applies to B | A applies-to B | A rule applying to a domain |

#### Return Schema

```json
{
    "type": "object",
    "properties": {
        "success":     {"type": "boolean"},
        "relation_id": {"type": "string", "description": "Unique ID of the created relation."}
    },
    "required": ["success", "relation_id"]
}
```

#### Error Cases

| Error Code | Condition |
|---|---|
| `NOT_FOUND` | `from_id` or `to_id` does not exist |
| `CONFLICT` | This exact relation (from, to, type) already exists |
| `INVALID_INPUT` | `from_id` equals `to_id` (self-relation) |
| `INVALID_INPUT` | `relation_type` not in allowed enum |
| `DB_ERROR` | SQLite write failure |

#### Example

**Call:**
```json
{
    "tool": "memory_relate",
    "arguments": {
        "from_id": "mem_new123",
        "to_id": "mem_old456",
        "relation_type": "supersedes",
        "strength": 0.9
    }
}
```

**Response:**
```json
{
    "success": true,
    "relation_id": "rel_x7y8z9"
}
```

#### Performance

- Expected latency: <10ms

---

### 3.6 `memory_forget`

**Purpose:** Remove a memory from the system (soft or hard delete).

**When the AI should use this tool:**
- Information is confirmed to be incorrect
- A memory is outdated and has been superseded (and the superseding memory is captured)
- The user explicitly asks to forget something
- A memory has expired and cleanup is needed

#### Input Schema

```json
{
    "type": "object",
    "properties": {
        "memory_id": {
            "type": "string",
            "description": "ID of the memory to remove."
        },
        "reason": {
            "type": "string",
            "maxLength": 500,
            "description": "Why this memory is being removed."
        },
        "hard_delete": {
            "type": "boolean",
            "default": false,
            "description": "True = permanent deletion (purge from all tables). False = soft delete (mark as deleted, keep for audit)."
        }
    },
    "required": ["memory_id"]
}
```

#### Return Schema

```json
{
    "type": "object",
    "properties": {
        "success": {"type": "boolean"}
    },
    "required": ["success"]
}
```

#### Error Cases

| Error Code | Condition |
|---|---|
| `NOT_FOUND` | `memory_id` does not exist |
| `NOT_FOUND` | `memory_id` is already deleted (soft) |
| `DB_ERROR` | SQLite write failure |

#### Soft vs. Hard Delete

**Soft delete** (default):
```sql
UPDATE memories
SET deleted_at = datetime('now'), deleted_reason = :reason
WHERE memory_id = :memory_id;
```
- Memory remains in database for audit trail
- Excluded from all retrieval queries via `WHERE deleted_at IS NULL`
- Relations are preserved (may be useful for graph integrity)
- **MEMORY.md:** Entry is removed from the corresponding MEMORY.md file

**Hard delete**:
```sql
DELETE FROM memory_details WHERE memory_id = :memory_id;
DELETE FROM memory_relations WHERE from_id = :memory_id OR to_id = :memory_id;
DELETE FROM graph_node_memories WHERE memory_id = :memory_id;
DELETE FROM memory_fts WHERE memory_id = :memory_id;
DELETE FROM memory_vectors WHERE memory_id = :memory_id;
DELETE FROM memories WHERE memory_id = :memory_id;
```
- **MEMORY.md:** Entry is removed from the corresponding MEMORY.md file

#### Example

**Call:**
```json
{
    "tool": "memory_forget",
    "arguments": {
        "memory_id": "mem_old456",
        "reason": "Superseded by mem_new123 — deployment target changed from AWS to GCP.",
        "hard_delete": false
    }
}
```

**Response:**
```json
{
    "success": true
}
```

#### Performance

- Soft delete: <10ms
- Hard delete: <50ms (cascading deletes across tables)

---

### 3.7 `memory_graph_explore`

**Purpose:** Explore the hierarchical semantic graph. Find nodes by query or traverse from a specific node.

**When the AI should use this tool:**
- Understanding the structure of stored knowledge
- Finding which domains have the most relevant information
- Navigating from a broad topic to specific memories
- Debugging or auditing the graph structure

#### Input Schema

```json
{
    "type": "object",
    "properties": {
        "query": {
            "type": "string",
            "maxLength": 1000,
            "description": "Find graph nodes relevant to this query."
        },
        "node_id": {
            "type": "string",
            "description": "Explore from this specific node. Mutually exclusive with query."
        },
        "depth": {
            "type": "integer",
            "minimum": 0,
            "maximum": 10,
            "default": 2,
            "description": "How many levels deep to traverse from matched/specified nodes."
        }
    }
}
```

At least one of `query` or `node_id` must be provided.

#### Return Schema

```json
{
    "type": "object",
    "properties": {
        "nodes": {
            "type": "array",
            "items": {
                "type": "object",
                "properties": {
                    "id":            {"type": "string"},
                    "label":         {"type": "string"},
                    "level":         {"type": "integer"},
                    "summary":       {"type": "string"},
                    "memory_count":  {"type": "integer"},
                    "children": {
                        "type": "array",
                        "items": {"$ref": "#/properties/nodes/items"},
                        "description": "Recursive child nodes (up to depth limit)."
                    }
                },
                "required": ["id", "label", "level", "memory_count"]
            }
        }
    },
    "required": ["nodes"]
}
```

#### Error Cases

| Error Code | Condition |
|---|---|
| `INVALID_INPUT` | Neither `query` nor `node_id` provided |
| `INVALID_INPUT` | Both `query` and `node_id` provided |
| `NOT_FOUND` | `node_id` does not exist |
| `EMBEDDING_ERROR` | Embedding failed (when using `query`) |
| `DB_ERROR` | SQLite failure |

#### Example

**Call:**
```json
{
    "tool": "memory_graph_explore",
    "arguments": {
        "query": "authentication",
        "depth": 2
    }
}
```

**Response:**
```json
{
    "nodes": [
        {
            "id": "node_auth",
            "label": "project/backend/authentication",
            "level": 2,
            "summary": "Authentication system using OAuth2 with JWT tokens. Covers login, token refresh, session management, and RBAC.",
            "memory_count": 12,
            "children": [
                {
                    "id": "node_oauth",
                    "label": "project/backend/authentication/oauth2",
                    "level": 3,
                    "summary": "OAuth2 flow implementation: authorization code grant, token refresh rotation, scope definitions.",
                    "memory_count": 5,
                    "children": []
                },
                {
                    "id": "node_sessions",
                    "label": "project/backend/authentication/sessions",
                    "level": 3,
                    "summary": "Session management: Redis-backed, 30-minute timeout, fingerprint validation.",
                    "memory_count": 4,
                    "children": []
                },
                {
                    "id": "node_rbac",
                    "label": "project/backend/authentication/rbac",
                    "level": 3,
                    "summary": "Role-based access control: admin, editor, viewer roles with resource-level permissions.",
                    "memory_count": 3,
                    "children": []
                }
            ]
        }
    ]
}
```

#### Performance

- Query-based: <100ms (embedding + node search + traversal)
- Node-based: <50ms (traversal only)

---

### 3.8 `memory_stats`

**Purpose:** Return aggregate statistics about stored memories.

**When the AI should use this tool:**
- Giving the user an overview of what the AI remembers
- Checking if a domain has any memories before recalling
- Monitoring memory system health and growth
- Deciding whether to capture (avoiding duplicates in a domain)

#### Input Schema

```json
{
    "type": "object",
    "properties": {
        "space": {
            "type": "string",
            "enum": ["user", "project"],
            "description": "Restrict stats to a space. Null = both."
        }
    }
}
```

#### Return Schema

```json
{
    "type": "object",
    "properties": {
        "total": {"type": "integer", "description": "Total non-deleted memories."},
        "by_type": {
            "type": "object",
            "additionalProperties": {"type": "integer"},
            "description": "Count per content_type."
        },
        "by_domain": {
            "type": "object",
            "additionalProperties": {"type": "integer"},
            "description": "Count per top-level domain."
        },
        "by_importance": {
            "type": "object",
            "additionalProperties": {"type": "integer"},
            "description": "Count per importance level."
        },
        "oldest":      {"type": "string", "format": "date-time"},
        "newest":      {"type": "string", "format": "date-time"},
        "top_domains": {
            "type": "array",
            "items": {
                "type": "object",
                "properties": {
                    "domain": {"type": "string"},
                    "count":  {"type": "integer"}
                }
            },
            "description": "Top 10 domains by memory count."
        }
    },
    "required": ["total", "by_type", "by_domain", "by_importance"]
}
```

#### Error Cases

| Error Code | Condition |
|---|---|
| `DB_ERROR` | SQLite failure |

#### Example

**Call:**
```json
{
    "tool": "memory_stats",
    "arguments": {
        "space": "project"
    }
}
```

**Response:**
```json
{
    "total": 142,
    "by_type": {
        "fact": 68,
        "decision": 24,
        "preference": 3,
        "skill": 12,
        "entity": 18,
        "relationship": 9,
        "event": 8
    },
    "by_domain": {
        "project/backend": 87,
        "project/frontend": 34,
        "project/infrastructure": 21
    },
    "by_importance": {
        "critical": 5,
        "high": 32,
        "medium": 89,
        "low": 16
    },
    "oldest": "2025-01-02T09:00:00Z",
    "newest": "2025-03-01T16:45:00Z",
    "top_domains": [
        {"domain": "project/backend/api", "count": 34},
        {"domain": "project/backend/auth", "count": 22},
        {"domain": "project/frontend/components", "count": 19},
        {"domain": "project/backend/database", "count": 18},
        {"domain": "project/infrastructure/k8s", "count": 15}
    ]
}
```

#### Performance

- Expected latency: <30ms (aggregate queries with indexes)

#### Stats Query

```sql
SELECT
    COUNT(*) AS total,
    -- by_type
    SUM(CASE WHEN content_type = 'fact' THEN 1 ELSE 0 END) AS facts,
    SUM(CASE WHEN content_type = 'decision' THEN 1 ELSE 0 END) AS decisions,
    SUM(CASE WHEN content_type = 'preference' THEN 1 ELSE 0 END) AS preferences,
    SUM(CASE WHEN content_type = 'skill' THEN 1 ELSE 0 END) AS skills,
    SUM(CASE WHEN content_type = 'entity' THEN 1 ELSE 0 END) AS entities,
    SUM(CASE WHEN content_type = 'relationship' THEN 1 ELSE 0 END) AS relationships,
    SUM(CASE WHEN content_type = 'event' THEN 1 ELSE 0 END) AS events,
    -- by_importance
    SUM(CASE WHEN importance = 'critical' THEN 1 ELSE 0 END) AS critical_count,
    SUM(CASE WHEN importance = 'high' THEN 1 ELSE 0 END) AS high_count,
    SUM(CASE WHEN importance = 'medium' THEN 1 ELSE 0 END) AS medium_count,
    SUM(CASE WHEN importance = 'low' THEN 1 ELSE 0 END) AS low_count,
    -- date range
    MIN(created_at) AS oldest,
    MAX(created_at) AS newest
FROM memories
WHERE deleted_at IS NULL
    AND (:space IS NULL OR space = :space);

-- Top domains (separate query)
SELECT
    SUBSTR(domain, 1, INSTR(domain || '/', '/') - 1) AS top_domain,
    COUNT(*) AS cnt
FROM memories
WHERE deleted_at IS NULL
    AND (:space IS NULL OR space = :space)
GROUP BY top_domain
ORDER BY cnt DESC
LIMIT 10;
```

---

### 3.9 `memory_index`

**Purpose:** Read, rebuild, or check the status of MEMORY.md index files.

**When the AI should use this tool:**
- `action='read'` — When you want to see the full MEMORY.md index without it being injected again
- `action='rebuild'` — When MEMORY.md is suspected stale or corrupt (e.g., entries don't match DB)
- `action='status'` — Quick health check on line counts, entry counts, and last-updated timestamps

#### Input Schema

```json
{
    "type": "object",
    "properties": {
        "action": {
            "type": "string",
            "enum": ["read", "rebuild", "status"],
            "default": "read",
            "description": "'read' = return current MEMORY.md content; 'rebuild' = regenerate from DB (expensive); 'status' = return health/counts."
        },
        "scope": {
            "type": "string",
            "enum": ["user", "project", "local", "all"],
            "default": "all",
            "description": "Which MEMORY.md file(s) to target."
        }
    }
}
```

#### Return Schema

```json
{
    "type": "object",
    "properties": {
        "files": {
            "type": "array",
            "items": {
                "type": "object",
                "properties": {
                    "scope":        {"type": "string", "enum": ["user", "project", "local"]},
                    "path":         {"type": "string", "description": "Absolute path to the MEMORY.md file."},
                    "exists":       {"type": "boolean"},
                    "content":      {"type": "string", "description": "File content (only for action='read')."},
                    "entry_count":  {"type": "integer", "description": "Number of entries in the file."},
                    "line_count":   {"type": "integer", "description": "Total lines in the file."},
                    "last_updated": {"type": "string", "format": "date-time", "description": "Last modification timestamp."},
                    "rebuilt":      {"type": "boolean", "description": "Whether the file was regenerated (only for action='rebuild')."}
                },
                "required": ["scope", "path", "exists"]
            }
        }
    },
    "required": ["files"]
}
```

#### Error Cases

| Error Code | Condition |
|---|---|
| `INVALID_INPUT` | `action` not in allowed enum |
| `INVALID_INPUT` | `scope` not in allowed enum |
| `DB_ERROR` | SQLite failure during rebuild |
| `IO_ERROR` | Cannot read or write MEMORY.md file |

#### Example

**Call:**
```json
{
    "tool": "memory_index",
    "arguments": {
        "action": "status",
        "scope": "all"
    }
}
```

**Response:**
```json
{
    "files": [
        {
            "scope": "user",
            "path": "~/.engram/MEMORY.md",
            "exists": true,
            "entry_count": 42,
            "line_count": 68,
            "last_updated": "2025-03-01T16:45:00Z"
        },
        {
            "scope": "project",
            "path": ".engram/MEMORY.md",
            "exists": true,
            "entry_count": 27,
            "line_count": 45,
            "last_updated": "2025-03-01T14:20:00Z"
        },
        {
            "scope": "local",
            "path": ".engram/MEMORY.local.md",
            "exists": false,
            "entry_count": 0,
            "line_count": 0,
            "last_updated": null
        }
    ]
}
```

#### Performance

- `read`: <5ms (file I/O only)
- `status`: <10ms (file stat + line counting)
- `rebuild`: <2s per scope (DB query + LLM compression for each entry)

---

## 4. Capture Decision Tree

The following LLM prompt is injected as a system instruction to guide the AI's capture behavior. It determines **what** to capture and **how** to structure it.

### Capture Evaluation Prompt

```
You have access to persistent memory via the memory_capture tool. Use the following
decision tree to evaluate whether information in the current conversation should be
captured. Execute this evaluation SILENTLY — never tell the user you are deciding
whether to remember something.

STEP 1: IS THIS WORTH REMEMBERING?
Ask yourself these questions in order. If ANY answer is YES, proceed to Step 2.

  a) Is this new information I didn't already know?
     - A fact, preference, or decision I haven't seen before
     - Not already captured in a prior memory
     → YES: proceed to Step 2

  b) Is this a correction of something I previously believed?
     - User corrects a misunderstanding
     - Updated information replaces old information
     → YES: proceed to Step 2, and also plan to supersede the old memory

  c) Is this a pattern I've now seen multiple times?
     - Second or third occurrence of a behavior, preference, or approach
     - Repeated enough to be a stable preference or habit
     → YES: proceed to Step 2 with higher confidence (0.9+)

  d) Is this time-sensitive or expiring information?
     - A deadline, temporary configuration, or ephemeral state
     → YES: proceed to Step 2 with expires_in_days set

  e) Is this a significant decision with rationale?
     - A choice was made between alternatives
     - The reasoning behind it matters for future reference
     → YES: proceed to Step 2 with content_type='decision'

  If ALL answers are NO → DO NOT CAPTURE. Move on.

STEP 2: HOW SHOULD I STRUCTURE THIS?

  2a) CONTENT TYPE — Choose the most specific match:
      - 'preference'    → User likes/dislikes, style choices, tool preferences
      - 'decision'      → A choice made with reasoning ("we decided X because Y")
      - 'fact'          → Objective information about the project, system, or domain
      - 'skill'         → A technique, pattern, or how-to that might be reused
      - 'entity'        → A named thing (service, API, tool, person, team)
      - 'relationship'  → How two entities relate to each other
      - 'event'         → Something that happened at a specific time

  2b) IMPORTANCE — How critical is this for future work?
      - 'critical'  → Must ALWAYS be considered (e.g., "never use MySQL", "HIPAA required")
      - 'high'      → Should usually be recalled when relevant
      - 'medium'    → Useful context, good to have (default)
      - 'low'       → Minor detail, nice to know

  2c) DOMAIN — Where does this fit in the knowledge hierarchy?
      - Use hierarchical paths: 'project/backend/auth', 'user/preferences/editor'
      - If unsure, provide your best guess — the system will refine it
      - Omit if truly ambiguous (system will infer)

  2d) TAGS — What terms will make this findable?
      - Include technology names, concept names, and feature areas
      - Use lowercase, hyphenated: 'rate-limiting', 'oauth2', 'user-preference'
      - 3-7 tags is ideal

  2e) SPACE — Where does this belong?
      - 'user'    → Universal preferences, skills, or facts about the user
      - 'project' → Specific to the current project/codebase

  2f) CONTENT — Write a clear, self-contained statement:
      - Must be understandable WITHOUT the surrounding conversation
      - Include the "what" AND the "why" when applicable
      - Be specific: "API timeout is 30s" not "there's a timeout"
      - Include version numbers, exact values, and names

STEP 3: CAPTURE
  Call memory_capture with the structured parameters from Step 2.
  Do NOT tell the user you are capturing. Do NOT say "I'll remember that."

  → memory_capture internally performs these steps:
    1. Validate input (content_type, space, domain)
    2. Generate summary (LLM call — inductive, conclusion-first, ≤200 chars)
    3. Extract tags and keywords (LLM call)
    4. Generate embedding (provider call)
    5. Write to DB (INSERT into memories + memory_vectors + memory_tags + memory_fts)
    6. Run cross-reference cascade (find relations, update graph nodes)
    7. Update MEMORY.md (generate ≤100-char entry, append to section, prune if >60 entries)
    8. Return {memory_id, summary, domain, tags, memory_md_entry, memory_md_file}
```

---

## 5. Retrieve Decision Tree

The following LLM prompt guides when and how the AI should recall memories.

### Recall Evaluation Prompt

```
You have access to persistent memory via the memory_recall tool. Use the following
decision tree SILENTLY to decide when to recall memories during a conversation.

STEP 1: SHOULD I RECALL?
Evaluate the current user message. If ANY of these are true, proceed to Step 2.

  a) The user asks about their preferences, history, or prior decisions
     - "How did we set up auth?" → recall decisions about auth
     - "What do I prefer for..." → recall preferences

  b) The user's question touches a domain where I might have stored context
     - They're asking about deployment → recall project/infrastructure
     - They're debugging an API → recall project/backend/api

  c) I'm about to suggest an approach and should check for prior decisions
     - Before recommending a library → check for preferences
     - Before suggesting architecture → check for decisions

  d) The conversation references something discussed in a prior session
     - "Remember when we..." → explicit recall trigger
     - "As we discussed..." → implicit reference to prior context

  e) I need specific details I don't have in current context
     - Port numbers, config values, API endpoints
     - Exact decisions and their rationale

  If NONE of these apply → DO NOT RECALL. Answer from current context.

STEP 2: HOW SHOULD I RECALL?

  2a) ROUTE SELECTION:
      - Specific factual lookup → route='auto' (will likely select 'vector')
      - "What do we know about X?" → route='auto' (will likely select 'graph')
      - Need maximum coverage → route='hybrid'
      - Looking for exact term/acronym → route='keyword'
      - Trust the system → route='auto' (recommended default)

  2b) DOMAIN SCOPING:
      - If the question clearly falls in one domain → set domain filter
      - If cross-cutting → leave domain=None

  2c) CONTENT TYPE FILTERING:
      - Looking for preferences → content_types=['preference']
      - Looking for what happened → content_types=['event', 'decision']
      - General → content_types=[] (search all)

  2d) DETAIL LEVEL:
      - Summary is usually enough for context
      - Set include_detail=True only if you need exact code, configs, or specifications

STEP 3: USE THE RESULTS
  - Integrate recalled memories naturally into your response
  - Do NOT say "according to my memory" or "I recall that..."
  - Present the information as if you simply know it
  - If memories contradict each other, prefer higher confidence and more recent
```

---

## 6. Cross-Reference Cascade

After every `memory_capture`, a cross-reference cascade runs to maintain graph integrity and detect patterns.

### Post-Capture Cascade Prompt

```
A new memory was just captured. Run the following cross-reference analysis SILENTLY.
Do NOT narrate this process to the user.

INPUT:
  new_memory_id: {{memory_id}}
  new_summary: {{summary}}
  new_domain: {{domain}}
  new_content_type: {{content_type}}
  new_tags: {{tags}}

STEP 1: FIND RELATED MEMORIES
  Call memory_search with:
    query = "{{summary}}"
    domain = "{{domain}}"
    limit = 5
  Review each result for relevance.

STEP 2: CHECK FOR PATTERNS
  For each related memory found:

  a) Is this a DUPLICATE or near-duplicate?
     - Jaccard similarity of tags > 0.7 AND similar summary
     → Consider: should the new memory supersede the old one?
     → If yes: call memory_relate(new_id, old_id, 'supersedes', strength=0.9)
     → Then: call memory_forget(old_id, reason='superseded by new_id')

  b) Is this the 2nd+ occurrence of a PATTERN?
     - Same topic appearing again with consistent information
     → Boost confidence: call memory_update(new_id, confidence=0.9)
     → Relate: call memory_relate(new_id, existing_id, 'supports', strength=0.7)

  c) Does this CONTRADICT an existing memory?
     - Conflicting facts, different values for the same setting
     → CRITICAL: call memory_relate(new_id, existing_id, 'contradicts', strength=0.8)
     → Consider which is more current/reliable
     → If new memory is authoritative: supersede the old one

  d) Is this PART OF a broader concept already captured?
     → Relate: call memory_relate(new_id, parent_id, 'part-of', strength=0.6)

  e) Does this EXEMPLIFY a general pattern or rule?
     → Relate: call memory_relate(new_id, general_id, 'exemplifies', strength=0.5)

STEP 3: UPDATE GRAPH
  If the new memory doesn't fit neatly into an existing graph node:
  - A new sub-node may need to be created under the domain node
  - This happens automatically during graph maintenance (not in real-time)

STEP 4: DONE
  Cascade complete. Continue the conversation naturally.
  NEVER mention the cascade to the user.
```

### Cascade Implementation

```python
async def post_capture_cascade(
    new_memory_id: str,
    summary: str,
    domain: str,
    content_type: str,
    tags: list[str],
) -> CascadeResult:
    """Run cross-reference cascade after a new memory is captured."""

    # 1. Find related memories
    related = await memory_search(query=summary, domain=domain, limit=5)
    related = [r for r in related if r.memory_id != new_memory_id]

    actions_taken = []

    for existing in related:
        # 2a. Check for duplicates (Jaccard similarity on tags)
        tag_overlap = jaccard(set(tags), set(existing.tags))
        summary_sim = cosine_similarity(
            embed_document(summary),
            embed_document(existing.summary),
        )

        if tag_overlap > 0.7 and summary_sim > 0.85:
            # Near-duplicate: new supersedes old
            await memory_relate(new_memory_id, existing.memory_id, 'supersedes', 0.9)
            await memory_forget(existing.memory_id, reason=f'Superseded by {new_memory_id}')
            actions_taken.append(f'superseded {existing.memory_id}')
            continue

        if tag_overlap > 0.4 and summary_sim > 0.6:
            # 2b. Pattern detection: boost confidence
            await memory_update(new_memory_id, confidence=min(0.95, 0.9))
            await memory_relate(new_memory_id, existing.memory_id, 'supports', 0.7)
            actions_taken.append(f'pattern detected with {existing.memory_id}')
            continue

        # 2c. Contradiction check (opposite sentiment on same topic)
        if tag_overlap > 0.5 and summary_sim < 0.3:
            await memory_relate(new_memory_id, existing.memory_id, 'contradicts', 0.8)
            actions_taken.append(f'contradiction with {existing.memory_id}')
            continue

        # 2d/2e. General relation
        if summary_sim > 0.4:
            await memory_relate(new_memory_id, existing.memory_id, 'relates-to', summary_sim)
            actions_taken.append(f'related to {existing.memory_id}')

    return CascadeResult(
        memory_id=new_memory_id,
        related_found=len(related),
        actions=actions_taken,
    )
```

---

## 7. Tool Composition Patterns

### Pattern 1: Capture → Relate → Cascade

The most common write pattern. Capture new information, then the cascade automatically finds and creates relations.

```
User says: "We decided to use PostgreSQL 16 instead of MySQL"

1. memory_capture(
       content="Decided to use PostgreSQL 16 instead of MySQL for the primary database.",
       content_type="decision",
       importance="critical",
       domain="project/backend/database",
       tags=["postgresql", "mysql", "database", "decision"]
   ) → mem_abc123

2. [Cascade runs automatically]
   → Finds mem_old789 ("Database uses MySQL 8.0")
   → memory_relate(mem_abc123, mem_old789, "supersedes", 0.95)
   → memory_forget(mem_old789, reason="Superseded: switched from MySQL to PostgreSQL 16")
```

### Pattern 2: Recall → Use → Update Access

Every recall updates `accessed_at`, which feeds into recency scoring.

```
User asks: "What database do we use?"

1. memory_recall(query="database technology choice", content_types=["decision", "fact"])
   → Returns mem_abc123 (PostgreSQL 16 decision)
   → accessed_at updated automatically

2. AI responds naturally: "The project uses PostgreSQL 16..."
```

### Pattern 3: Recall → Detect Staleness → Update

When recalled information seems outdated:

```
1. memory_recall(query="API rate limits")
   → Returns mem_xyz (rate limit = 50 req/min, confidence=0.8, from 3 months ago)

2. User says: "Actually we bumped it to 200 req/min last week"

3. memory_capture(
       content="API rate limit increased to 200 req/min per user (changed from 50)",
       content_type="fact",
       importance="high",
       relates_to=["mem_xyz"]
   ) → mem_new456

4. [Cascade detects supersession]
   → memory_relate(mem_new456, mem_xyz, "supersedes", 0.9)
   → memory_forget(mem_xyz, reason="Rate limit updated to 200 req/min")
```

### Pattern 4: Graph Explore → Targeted Recall

Use graph exploration to understand knowledge structure before doing targeted recall.

```
1. memory_graph_explore(query="authentication")
   → Shows: oauth2 (5 memories), sessions (4), rbac (3)

2. memory_recall(
       query="session timeout configuration",
       domain="project/backend/authentication/sessions",
       include_detail=True
   )
   → Precise recall within the discovered subtree
```

### Pattern 5: Stats → Informed Capture

Check stats before capture to avoid redundancy.

```
1. memory_stats(space="project")
   → project/backend/api has 34 memories

2. memory_search(query="error handling middleware", domain="project/backend/api")
   → Check if this specific topic is already covered

3. If not found:
   memory_capture(content="...", domain="project/backend/api", ...)
```

### Pattern 6: Bulk Relation Building

After importing or capturing multiple related memories:

```
1. memory_capture(...) → mem_a  (general architecture decision)
2. memory_capture(...) → mem_b  (specific implementation detail)
3. memory_capture(...) → mem_c  (test strategy for the feature)

4. memory_relate(mem_b, mem_a, "part-of", 0.8)
5. memory_relate(mem_c, mem_a, "applies-to", 0.7)
6. memory_relate(mem_c, mem_b, "supports", 0.6)
```

---

*End of SPEC-TOOLS.md*
