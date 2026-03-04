# SPEC-CLAUDE-CODE-PLUGIN: Claude Code Plugin Specification

**System:** engram-lite
**Version:** 0.1.0
**Status:** Draft
**Last Updated:** 2026-03-03

---

## Table of Contents

1. [Overview](#1-overview)
2. [Plugin Structure](#2-plugin-structure)
3. [Plugin Manifest](#3-plugin-manifest)
4. [Shell Hook Scripts](#4-shell-hook-scripts)
5. [MCP Server](#5-mcp-server)
6. [MCP Registration](#6-mcp-registration)
7. [Claude Code Settings](#7-claude-code-settings)
8. [Slash Commands](#8-slash-commands)
9. [Installation](#9-installation)
10. [Configuration](#10-configuration)
11. [Marketplace Entry](#11-marketplace-entry)

---

## 1. Overview

engram-lite integrates with Claude Code through two complementary mechanisms:

| Mechanism | Purpose | How It Works |
|---|---|---|
| **Plugin hooks** | Inject behavioral reminders at lifecycle events | Shell scripts output `<system-reminder>` XML |
| **MCP server** | Expose memory tools to the agent | Python MCP server registered in `.mcp.json` |

Additionally, **slash commands** provide explicit user-facing operations for memory management.

### Integration Architecture

```
Claude Code Runtime
│
├── Plugin Hooks (shell scripts)
│   ├── SessionStart  → hooks/session-start.sh  → <system-reminder> hot context
│   ├── UserPromptSubmit → hooks/prompt-submit.sh → <system-reminder> recall hint
│   └── Stop          → hooks/stop.sh           → <system-reminder> capture hint
│
├── MCP Server (Python)
│   └── engram-lite-mcp → 8 tools available to agent
│       ├── memory_capture
│       ├── memory_recall
│       ├── memory_search
│       ├── memory_update
│       ├── memory_relate
│       ├── memory_forget
│       ├── memory_graph_explore
│       └── memory_stats
│
└── Slash Commands (markdown)
    ├── /memory-recall    → explicit recall
    ├── /memory-capture   → explicit capture
    ├── /memory-stats     → show statistics
    └── /memory-forget    → delete a memory
```

### Prerequisites

- Claude Code >= 1.0
- Python >= 3.11
- `engram-lite` Python package installed (`pip install engram-lite`)
- An embedding provider configured (OpenAI, Azure OpenAI, or Ollama)

---

## 2. Plugin Structure

```
claude-code/
├── .claude-plugin/
│   ├── plugin.json                 # Plugin manifest (hooks, metadata)
│   └── marketplace.json            # Marketplace listing metadata
├── hooks/
│   ├── session-start.sh            # SessionStart hook script
│   ├── prompt-submit.sh            # UserPromptSubmit hook script
│   └── stop.sh                     # Stop hook script
├── mcp/
│   └── server.py                   # MCP server exposing memory tools
├── .mcp.json                       # MCP server registration
└── .claude/
    ├── settings.json               # Plugin enablement + env vars
    └── commands/
        ├── memory-recall.md        # /memory-recall slash command
        ├── memory-capture.md       # /memory-capture slash command
        ├── memory-stats.md         # /memory-stats slash command
        └── memory-forget.md        # /memory-forget slash command
```

---

## 3. Plugin Manifest

### `.claude-plugin/plugin.json`

```json
{
  "name": "engram-lite",
  "version": "0.1.0",
  "description": "Persistent memory system for Claude Code. Provides cross-session knowledge retention with automatic capture, semantic recall, and knowledge graph traversal.",
  "author": "engram-lite",
  "license": "MIT",
  "homepage": "https://github.com/engram-lite/engram-lite",

  "hooks": {
    "SessionStart": [
      {
        "type": "command",
        "command": "bash hooks/session-start.sh",
        "timeout": 5000,
        "description": "Load hot context (critical/high importance memories) and inject behavioral protocol."
      }
    ],
    "UserPromptSubmit": [
      {
        "type": "command",
        "command": "bash hooks/prompt-submit.sh \"$PROMPT\"",
        "timeout": 2000,
        "description": "Inject per-prompt recall reminder with first 50 chars of user prompt."
      }
    ],
    "Stop": [
      {
        "type": "command",
        "command": "bash hooks/stop.sh",
        "timeout": 2000,
        "description": "Inject post-response capture reminder."
      }
    ]
  },

  "capabilities": {
    "mcp": true,
    "slash_commands": true
  },

  "configuration": {
    "ENGRAM_DB_PATH": {
      "type": "string",
      "default": "~/.engram-lite/memory.db",
      "description": "Path to the SQLite database file."
    },
    "ENGRAM_EMBEDDING_PROVIDER": {
      "type": "string",
      "default": "openai",
      "enum": ["openai", "azure", "ollama"],
      "description": "Embedding provider to use."
    },
    "ENGRAM_HOT_CONTEXT_LIMIT": {
      "type": "integer",
      "default": 20,
      "description": "Maximum number of memories to load at session start."
    },
    "ENGRAM_AUTO_RECALL": {
      "type": "boolean",
      "default": true,
      "description": "Inject recall reminders on each prompt."
    },
    "ENGRAM_AUTO_CAPTURE": {
      "type": "boolean",
      "default": true,
      "description": "Inject capture reminders after each response."
    }
  }
}
```

### Field Reference

| Field | Required | Description |
|---|---|---|
| `name` | Yes | Unique plugin identifier |
| `version` | Yes | SemVer version string |
| `description` | Yes | Human-readable description |
| `hooks` | Yes | Map of lifecycle event → array of hook definitions |
| `hooks.*.type` | Yes | Hook type: `"command"` for shell hooks |
| `hooks.*.command` | Yes | Shell command to execute |
| `hooks.*.timeout` | No | Max execution time in milliseconds (default: 10000) |
| `capabilities` | No | Feature flags for the plugin |
| `configuration` | No | Configuration schema for plugin settings |

### Hook Event Reference

| Event | When It Fires | Input Available | Output Expected |
|---|---|---|---|
| `SessionStart` | Once at the beginning of each new session | Environment variables only | `<system-reminder>` on stdout |
| `UserPromptSubmit` | On each user message before agent processing | `$PROMPT` (user message text) | `<system-reminder>` on stdout |
| `Stop` | After each agent response completes | Environment variables only | `<system-reminder>` on stdout |

---

## 4. Shell Hook Scripts

All hook scripts follow these conventions:

1. **Output to stdout only.** Anything on stdout becomes context injected into the agent.
2. **Errors to stderr.** Errors on stderr are logged but not shown to the agent.
3. **Exit code 0 always.** Non-zero exits may prevent the agent from proceeding. Handle all errors internally and exit 0.
4. **Handle missing dependencies.** If `engram-lite` isn't installed or the DB doesn't exist, output nothing and exit 0.

### 4.1 `hooks/session-start.sh`

```bash
#!/usr/bin/env bash
# engram-lite: SessionStart hook
# Loads hot context (critical/high importance memories) and injects
# the behavioral protocol as a <system-reminder> block.
#
# Exit 0 always. Errors go to stderr. Empty stdout = no injection.

set -euo pipefail

# ---------------------------------------------------------------------------
# Configuration (from environment, with defaults)
# ---------------------------------------------------------------------------
DB_PATH="${ENGRAM_DB_PATH:-$HOME/.engram-lite/memory.db}"
HOT_CONTEXT_LIMIT="${ENGRAM_HOT_CONTEXT_LIMIT:-20}"
ENGRAM_BIN="${ENGRAM_BIN:-engram-lite}"

# ---------------------------------------------------------------------------
# Guard: check that engram-lite CLI is available
# ---------------------------------------------------------------------------
if ! command -v "$ENGRAM_BIN" &>/dev/null; then
    # Try as a Python module
    if ! python3 -m amplifier_module_engram_lite --version &>/dev/null 2>&1; then
        echo >&2 "engram-lite: CLI not found, skipping session-start hook"
        exit 0
    fi
    ENGRAM_BIN="python3 -m amplifier_module_engram_lite"
fi

# ---------------------------------------------------------------------------
# Guard: check that the database exists
# ---------------------------------------------------------------------------
EXPANDED_DB_PATH="${DB_PATH/#\~/$HOME}"

if [ ! -f "$EXPANDED_DB_PATH" ]; then
    # No database yet — inject protocol-only reminder (no memories)
    cat <<'REMINDER'
<system-reminder source="engram-lite">
MEMORY SYSTEM ACTIVE. No memories stored yet — this is a fresh database.

PROTOCOL:
- Use memory_recall(query) before responding to queries that may relate to prior context
- Use memory_capture(content) after learning new information
- Never announce memory operations to the user
</system-reminder>
REMINDER
    exit 0
fi

# ---------------------------------------------------------------------------
# Guard: check that the database is not locked
# ---------------------------------------------------------------------------
if ! sqlite3 "$EXPANDED_DB_PATH" "SELECT 1;" &>/dev/null 2>&1; then
    echo >&2 "engram-lite: database locked or corrupted, skipping hot context"
    cat <<'REMINDER'
<system-reminder source="engram-lite">
MEMORY SYSTEM ACTIVE. Database temporarily unavailable.

PROTOCOL:
- Use memory_recall(query) before responding to queries that may relate to prior context
- Use memory_capture(content) after learning new information
- Never announce memory operations to the user
</system-reminder>
REMINDER
    exit 0
fi

# ---------------------------------------------------------------------------
# Load hot context via CLI
# ---------------------------------------------------------------------------
HOT_CONTEXT=$($ENGRAM_BIN hot-context \
    --db-path "$EXPANDED_DB_PATH" \
    --limit "$HOT_CONTEXT_LIMIT" \
    --format reminder \
    2>/dev/null) || true

if [ -n "$HOT_CONTEXT" ]; then
    echo "$HOT_CONTEXT"
else
    # CLI succeeded but returned nothing — empty DB
    cat <<'REMINDER'
<system-reminder source="engram-lite">
MEMORY SYSTEM ACTIVE. No memories stored yet.

PROTOCOL:
- Use memory_recall(query) before responding to queries that may relate to prior context
- Use memory_capture(content) after learning new information
- Never announce memory operations to the user
</system-reminder>
REMINDER
fi

# ---------------------------------------------------------------------------
# Touch access timestamps (fire-and-forget, don't block session start)
# ---------------------------------------------------------------------------
$ENGRAM_BIN touch-accessed \
    --db-path "$EXPANDED_DB_PATH" \
    --scope hot \
    &>/dev/null &

exit 0
```

### 4.2 `hooks/prompt-submit.sh`

```bash
#!/usr/bin/env bash
# engram-lite: UserPromptSubmit hook
# Injects a per-prompt recall reminder with the first 50 characters
# of the user's prompt as a memory check hint.
#
# Usage: bash hooks/prompt-submit.sh "$PROMPT"
# Exit 0 always.

set -euo pipefail

# ---------------------------------------------------------------------------
# Configuration
# ---------------------------------------------------------------------------
AUTO_RECALL="${ENGRAM_AUTO_RECALL:-true}"

# ---------------------------------------------------------------------------
# Guard: check if auto-recall is disabled
# ---------------------------------------------------------------------------
if [ "$AUTO_RECALL" != "true" ]; then
    exit 0
fi

# ---------------------------------------------------------------------------
# Extract prompt snippet
# ---------------------------------------------------------------------------
PROMPT="${1:-}"
if [ -z "$PROMPT" ]; then
    exit 0
fi

# Take first 50 characters, escape quotes for safe XML embedding
SNIPPET=$(echo "$PROMPT" | head -c 50 | sed 's/"/\&quot;/g; s/</\&lt;/g; s/>/\&gt;/g')

# ---------------------------------------------------------------------------
# Output recall reminder
# ---------------------------------------------------------------------------
cat <<REMINDER
<system-reminder source="engram-lite">
Memory check: Does "${SNIPPET}" relate to prior context?
If yes -> memory_recall(query) before responding.
</system-reminder>
REMINDER

exit 0
```

### 4.3 `hooks/stop.sh`

```bash
#!/usr/bin/env bash
# engram-lite: Stop hook
# Injects a post-response capture reminder after the agent finishes
# a response. Triggers the agent's capture decision tree.
#
# Exit 0 always.

set -euo pipefail

# ---------------------------------------------------------------------------
# Configuration
# ---------------------------------------------------------------------------
AUTO_CAPTURE="${ENGRAM_AUTO_CAPTURE:-true}"

# ---------------------------------------------------------------------------
# Guard: check if auto-capture is disabled
# ---------------------------------------------------------------------------
if [ "$AUTO_CAPTURE" != "true" ]; then
    exit 0
fi

# ---------------------------------------------------------------------------
# Output capture reminder
# ---------------------------------------------------------------------------
cat <<'REMINDER'
<system-reminder source="engram-lite">
CAPTURE CHECK (silent):
1. Did I learn something new? -> memory_capture()
2. Did I make an error that was corrected? -> memory_capture(content_type='fact', importance='high')
3. Did I observe a preference/pattern? -> memory_capture(content_type='preference')
Execute silently. Do not announce.
</system-reminder>
REMINDER

exit 0
```

### 4.4 Error Handling Summary

All three scripts handle these failure conditions:

| Condition | `session-start.sh` | `prompt-submit.sh` | `stop.sh` |
|---|---|---|---|
| `engram-lite` not installed | Inject protocol-only reminder | Output nothing | Output nothing |
| DB file missing | Inject "fresh database" reminder | N/A (no DB access) | N/A |
| DB locked/corrupted | Inject "temporarily unavailable" reminder | N/A | N/A |
| `$PROMPT` empty | N/A | Output nothing | N/A |
| CLI command fails | Fall back to empty-DB reminder | Output nothing | Output nothing |
| Any unexpected error | `set -euo pipefail` + all commands guarded | Exit 0 | Exit 0 |

---

## 5. MCP Server

The MCP (Model Context Protocol) server exposes engram-lite's tools to Claude Code. The agent invokes these tools during conversation to capture and recall memories.

### 5.1 `claude-code/mcp/server.py`

```python
#!/usr/bin/env python3
"""engram-lite MCP server for Claude Code.

Exposes 8 memory tools via the Model Context Protocol. Claude Code
discovers this server via .mcp.json and makes tools available to the agent.

Usage:
    python mcp/server.py                          # stdio transport (default)
    python mcp/server.py --transport sse --port 8765  # SSE transport

Environment:
    ENGRAM_DB_PATH   — SQLite database path (default: ~/.engram-lite/memory.db)
    ENGRAM_EMBEDDING_PROVIDER — openai | azure | ollama (default: openai)
    OPENAI_API_KEY          — Required if using openai provider
"""

from __future__ import annotations

import json
import logging
import os
import sys
from pathlib import Path
from typing import Any

from mcp.server import Server
from mcp.server.stdio import stdio_server
from mcp.types import (
    CallToolResult,
    TextContent,
    Tool,
)

# ---------------------------------------------------------------------------
# Logging
# ---------------------------------------------------------------------------

logging.basicConfig(
    level=getattr(
        logging,
        os.environ.get("ENGRAM_LOG_LEVEL", "WARNING"),
    ),
    format="%(asctime)s [%(name)s] %(levelname)s: %(message)s",
    stream=sys.stderr,  # MCP uses stdout for protocol; logs go to stderr
)
logger = logging.getLogger("amplifier_module_engram_lite.mcp")

# ---------------------------------------------------------------------------
# Memory store initialization
# ---------------------------------------------------------------------------

_store = None


def get_store():
    """Lazy-initialize the MemoryStore singleton."""
    global _store
    if _store is None:
        from amplifier_module_engram_lite.db.memory_store import MemoryStore

        db_path = os.environ.get(
            "ENGRAM_DB_PATH",
            str(Path.home() / ".engram-lite" / "memory.db"),
        )
        # Ensure parent directory exists
        Path(db_path).expanduser().parent.mkdir(parents=True, exist_ok=True)
        _store = MemoryStore(str(Path(db_path).expanduser()))
        logger.info("MemoryStore initialized at %s", db_path)
    return _store


# ---------------------------------------------------------------------------
# Tool definitions
# ---------------------------------------------------------------------------

TOOLS: list[Tool] = [
    Tool(
        name="memory_capture",
        description=(
            "Store new knowledge as a persistent memory. Use after learning "
            "new facts, preferences, decisions, or patterns from the conversation. "
            "Write content conclusion-first. Never capture verbatim code or quotes."
        ),
        inputSchema={
            "type": "object",
            "required": ["content"],
            "properties": {
                "content": {
                    "type": "string",
                    "description": (
                        "Conclusion-first summary of the knowledge to store. "
                        "Start with the main claim, follow with supporting details."
                    ),
                },
                "content_type": {
                    "type": "string",
                    "enum": [
                        "fact",
                        "preference",
                        "decision",
                        "procedure",
                        "architecture",
                        "debug_insight",
                    ],
                    "default": "fact",
                    "description": "Type of knowledge being captured.",
                },
                "importance": {
                    "type": "string",
                    "enum": ["critical", "high", "medium", "low"],
                    "default": "medium",
                    "description": "How important this memory is for future sessions.",
                },
                "domain": {
                    "type": "string",
                    "description": (
                        "Hierarchical domain path (e.g., 'project/backend/auth'). "
                        "Use '/' separators, max 4 levels."
                    ),
                },
                "tags": {
                    "type": "array",
                    "items": {"type": "string"},
                    "description": "Short categorical labels for search.",
                },
                "keywords": {
                    "type": "array",
                    "items": {"type": "string"},
                    "description": "Longer search phrases for keyword retrieval.",
                },
                "confidence": {
                    "type": "number",
                    "minimum": 0.0,
                    "maximum": 1.0,
                    "default": 0.8,
                    "description": "Trust score for this memory's accuracy.",
                },
                "space": {
                    "type": "string",
                    "enum": ["user", "project"],
                    "default": "project",
                    "description": (
                        "'user' = follows user across projects, "
                        "'project' = scoped to this workspace."
                    ),
                },
                "source_context": {
                    "type": "string",
                    "description": "Brief note on what prompted this capture.",
                },
            },
        },
    ),
    Tool(
        name="memory_recall",
        description=(
            "Retrieve memories relevant to a query. Uses dual-route retrieval: "
            "fast vector similarity (System 1) and graph traversal (System 2). "
            "Use before responding to queries that may relate to prior context."
        ),
        inputSchema={
            "type": "object",
            "required": ["query"],
            "properties": {
                "query": {
                    "type": "string",
                    "description": "Natural language query to search memories.",
                },
                "limit": {
                    "type": "integer",
                    "default": 5,
                    "minimum": 1,
                    "maximum": 20,
                    "description": "Maximum number of results to return.",
                },
                "threshold": {
                    "type": "number",
                    "default": 0.3,
                    "minimum": 0.0,
                    "maximum": 1.0,
                    "description": "Minimum relevance score to include.",
                },
                "domains": {
                    "type": "array",
                    "items": {"type": "string"},
                    "description": "Restrict search to these domains.",
                },
                "content_types": {
                    "type": "array",
                    "items": {"type": "string"},
                    "description": "Restrict to these content types.",
                },
            },
        },
    ),
    Tool(
        name="memory_search",
        description=(
            "Search memories with explicit filters. More control than "
            "memory_recall — allows filtering by tags, date range, "
            "confidence, and other metadata fields."
        ),
        inputSchema={
            "type": "object",
            "properties": {
                "query": {
                    "type": "string",
                    "description": "Optional text query for relevance ranking.",
                },
                "domain": {
                    "type": "string",
                    "description": "Filter by domain path (prefix match).",
                },
                "content_type": {
                    "type": "string",
                    "enum": [
                        "fact",
                        "preference",
                        "decision",
                        "procedure",
                        "architecture",
                        "debug_insight",
                    ],
                    "description": "Filter by content type.",
                },
                "tags": {
                    "type": "array",
                    "items": {"type": "string"},
                    "description": "Filter by tags (AND logic).",
                },
                "importance": {
                    "type": "string",
                    "enum": ["critical", "high", "medium", "low"],
                    "description": "Filter by importance level.",
                },
                "min_confidence": {
                    "type": "number",
                    "default": 0.2,
                    "description": "Minimum confidence threshold.",
                },
                "space": {
                    "type": "string",
                    "enum": ["user", "project"],
                    "description": "Filter by space.",
                },
                "created_after": {
                    "type": "string",
                    "description": "ISO 8601 date — only memories created after this.",
                },
                "created_before": {
                    "type": "string",
                    "description": "ISO 8601 date — only memories created before this.",
                },
                "limit": {
                    "type": "integer",
                    "default": 10,
                    "minimum": 1,
                    "maximum": 50,
                    "description": "Maximum results.",
                },
            },
        },
    ),
    Tool(
        name="memory_update",
        description=(
            "Update metadata on an existing memory. Use to adjust confidence, "
            "add tags, change importance, or modify content."
        ),
        inputSchema={
            "type": "object",
            "required": ["memory_id"],
            "properties": {
                "memory_id": {
                    "type": "string",
                    "description": "ID of the memory to update.",
                },
                "content": {
                    "type": "string",
                    "description": "New content (triggers re-embedding).",
                },
                "confidence": {
                    "type": "number",
                    "minimum": 0.0,
                    "maximum": 1.0,
                    "description": "New confidence score.",
                },
                "importance": {
                    "type": "string",
                    "enum": ["critical", "high", "medium", "low"],
                    "description": "New importance level.",
                },
                "add_tags": {
                    "type": "array",
                    "items": {"type": "string"},
                    "description": "Tags to add (merged with existing).",
                },
                "remove_tags": {
                    "type": "array",
                    "items": {"type": "string"},
                    "description": "Tags to remove.",
                },
            },
        },
    ),
    Tool(
        name="memory_relate",
        description=(
            "Create or update a relationship between two memories in the "
            "knowledge graph. Use during cross-reference cascades."
        ),
        inputSchema={
            "type": "object",
            "required": ["source_id", "target_id", "relation_type"],
            "properties": {
                "source_id": {
                    "type": "string",
                    "description": "ID of the source memory.",
                },
                "target_id": {
                    "type": "string",
                    "description": "ID of the target memory.",
                },
                "relation_type": {
                    "type": "string",
                    "enum": [
                        "relates_to",
                        "supports",
                        "contradicts",
                        "supersedes",
                        "depends_on",
                        "part_of",
                    ],
                    "description": "Type of relationship.",
                },
                "weight": {
                    "type": "number",
                    "default": 1.0,
                    "minimum": 0.0,
                    "maximum": 1.0,
                    "description": "Relationship strength.",
                },
                "metadata": {
                    "type": "object",
                    "description": "Optional metadata for the relation.",
                },
            },
        },
    ),
    Tool(
        name="memory_forget",
        description=(
            "Soft-delete a memory. The memory is marked as deleted but "
            "retained for audit. Use when information is superseded, "
            "incorrect, or no longer relevant."
        ),
        inputSchema={
            "type": "object",
            "required": ["memory_id"],
            "properties": {
                "memory_id": {
                    "type": "string",
                    "description": "ID of the memory to forget.",
                },
                "reason": {
                    "type": "string",
                    "description": "Why this memory is being forgotten.",
                },
            },
        },
    ),
    Tool(
        name="memory_graph_explore",
        description=(
            "Traverse the knowledge graph starting from a memory or domain. "
            "Returns connected memories and their relationships. "
            "Use for System 2 deliberate retrieval."
        ),
        inputSchema={
            "type": "object",
            "required": ["start"],
            "properties": {
                "start": {
                    "type": "string",
                    "description": "Starting point: a memory ID or domain path.",
                },
                "depth": {
                    "type": "integer",
                    "default": 2,
                    "minimum": 1,
                    "maximum": 5,
                    "description": "How many hops to traverse.",
                },
                "relation_types": {
                    "type": "array",
                    "items": {"type": "string"},
                    "description": "Filter to these relation types only.",
                },
                "min_weight": {
                    "type": "number",
                    "default": 0.3,
                    "description": "Minimum relation weight to traverse.",
                },
            },
        },
    ),
    Tool(
        name="memory_stats",
        description=(
            "Return statistics about the memory system: total memories, "
            "domain distribution, confidence histogram, storage size."
        ),
        inputSchema={
            "type": "object",
            "properties": {},
        },
    ),
]


# ---------------------------------------------------------------------------
# Tool execution handlers
# ---------------------------------------------------------------------------

def handle_memory_capture(params: dict[str, Any]) -> dict[str, Any]:
    """Handle memory_capture tool invocation."""
    store = get_store()
    from amplifier_module_engram_lite.tools.capture import handle_capture

    return handle_capture(store, params)


def handle_memory_recall(params: dict[str, Any]) -> dict[str, Any]:
    """Handle memory_recall tool invocation."""
    store = get_store()
    from amplifier_module_engram_lite.tools.recall import handle_recall

    return handle_recall(store, params)


def handle_memory_search(params: dict[str, Any]) -> dict[str, Any]:
    """Handle memory_search tool invocation."""
    store = get_store()
    from amplifier_module_engram_lite.tools.recall import handle_search

    return handle_search(store, params)


def handle_memory_update(params: dict[str, Any]) -> dict[str, Any]:
    """Handle memory_update tool invocation."""
    store = get_store()
    from amplifier_module_engram_lite.tools.manage import handle_update

    return handle_update(store, params)


def handle_memory_relate(params: dict[str, Any]) -> dict[str, Any]:
    """Handle memory_relate tool invocation."""
    store = get_store()
    from amplifier_module_engram_lite.tools.manage import handle_relate

    return handle_relate(store, params)


def handle_memory_forget(params: dict[str, Any]) -> dict[str, Any]:
    """Handle memory_forget tool invocation."""
    store = get_store()
    from amplifier_module_engram_lite.tools.manage import handle_forget

    return handle_forget(store, params)


def handle_memory_graph_explore(params: dict[str, Any]) -> dict[str, Any]:
    """Handle memory_graph_explore tool invocation."""
    store = get_store()
    from amplifier_module_engram_lite.tools.recall import handle_graph_explore

    return handle_graph_explore(store, params)


def handle_memory_stats(params: dict[str, Any]) -> dict[str, Any]:
    """Handle memory_stats tool invocation."""
    store = get_store()
    from amplifier_module_engram_lite.tools.manage import handle_stats

    return handle_stats(store, params)


# Dispatch table
HANDLERS: dict[str, Any] = {
    "memory_capture": handle_memory_capture,
    "memory_recall": handle_memory_recall,
    "memory_search": handle_memory_search,
    "memory_update": handle_memory_update,
    "memory_relate": handle_memory_relate,
    "memory_forget": handle_memory_forget,
    "memory_graph_explore": handle_memory_graph_explore,
    "memory_stats": handle_memory_stats,
}


# ---------------------------------------------------------------------------
# MCP Server setup
# ---------------------------------------------------------------------------

server = Server("engram-lite")


@server.list_tools()
async def list_tools() -> list[Tool]:
    """Return all available memory tools."""
    return TOOLS


@server.call_tool()
async def call_tool(name: str, arguments: dict[str, Any] | None) -> list[TextContent]:
    """Execute a memory tool and return the result.

    All results are returned as JSON-formatted TextContent.
    Errors are caught and returned as structured error responses —
    the MCP server never raises exceptions to the client.
    """
    if name not in HANDLERS:
        return [
            TextContent(
                type="text",
                text=json.dumps({"error": f"Unknown tool: {name}"}),
            )
        ]

    handler = HANDLERS[name]
    params = arguments or {}

    try:
        result = handler(params)
        return [
            TextContent(
                type="text",
                text=json.dumps(result, default=str),
            )
        ]
    except Exception as exc:
        logger.error("Tool %s failed: %s", name, exc, exc_info=True)
        return [
            TextContent(
                type="text",
                text=json.dumps({
                    "error": str(exc),
                    "tool": name,
                    "status": "failed",
                }),
            )
        ]


# ---------------------------------------------------------------------------
# Server entry point
# ---------------------------------------------------------------------------

async def main():
    """Run the MCP server with stdio transport."""
    logger.info("Starting engram-lite MCP server (stdio transport)")
    async with stdio_server() as (read_stream, write_stream):
        await server.run(
            read_stream,
            write_stream,
            server.create_initialization_options(),
        )


if __name__ == "__main__":
    import asyncio

    # Check for --transport argument
    if "--transport" in sys.argv:
        idx = sys.argv.index("--transport")
        transport = sys.argv[idx + 1] if idx + 1 < len(sys.argv) else "stdio"

        if transport == "sse":
            port = 8765
            if "--port" in sys.argv:
                port_idx = sys.argv.index("--port")
                port = int(sys.argv[port_idx + 1])

            from mcp.server.sse import SseServerTransport
            from starlette.applications import Starlette
            from starlette.routing import Route
            import uvicorn

            sse = SseServerTransport("/messages")

            async def handle_sse(request):
                async with sse.connect_sse(
                    request.scope, request.receive, request._send
                ) as streams:
                    await server.run(
                        streams[0],
                        streams[1],
                        server.create_initialization_options(),
                    )

            app = Starlette(
                routes=[
                    Route("/sse", endpoint=handle_sse),
                    Route("/messages", endpoint=sse.handle_post_message, methods=["POST"]),
                ]
            )

            logger.info("Starting engram-lite MCP server (SSE transport, port %d)", port)
            uvicorn.run(app, host="0.0.0.0", port=port)
        else:
            asyncio.run(main())
    else:
        asyncio.run(main())
```

### 5.2 MCP Server Response Format

All tool responses are JSON objects returned as `TextContent`. The structure varies by tool:

#### Successful Capture Response

```json
{
  "id": "mem_a1b2c3d4",
  "status": "captured",
  "content_type": "fact",
  "domain": "project/backend/auth",
  "confidence": 0.85,
  "space": "project",
  "embedding_status": "completed"
}
```

#### Successful Recall Response

```json
{
  "memories": [
    {
      "id": "mem_a1b2c3d4",
      "content": "Project uses JWT with RS256 for API authentication.",
      "content_type": "fact",
      "domain": "project/backend/auth",
      "importance": "high",
      "confidence": 0.92,
      "tags": ["jwt", "auth", "rs256"],
      "relevance_score": 0.87,
      "retrieval_route": "system1",
      "created_at": "2026-02-15T10:30:00Z",
      "last_accessed": "2026-03-01T14:22:00Z"
    }
  ],
  "count": 1,
  "query": "authentication method",
  "retrieval_ms": 45
}
```

#### Error Response

```json
{
  "error": "Database is locked",
  "tool": "memory_recall",
  "status": "failed"
}
```

### 5.3 MCP Server Error Handling

| Error Condition | Behavior | Response |
|---|---|---|
| Unknown tool name | Return error JSON | `{"error": "Unknown tool: xyz"}` |
| Missing required parameter | Return error JSON | `{"error": "Missing required parameter: content"}` |
| Database not found | Auto-initialize empty DB | Normal response (empty results for recall) |
| Database locked | Return error JSON | `{"error": "Database is locked"}` |
| Embedding provider unreachable | Capture without embedding; recall uses keyword-only | Include `"embedding_status": "deferred"` |
| Invalid parameter type | Return error JSON | `{"error": "Expected string for content, got int"}` |
| Unhandled exception | Log to stderr, return error JSON | `{"error": "<exception message>", "status": "failed"}` |

---

## 6. MCP Registration

### 6.1 `.mcp.json`

Place this in the project root (or `~/.mcp.json` for global registration):

```json
{
  "mcpServers": {
    "engram-lite": {
      "command": "python3",
      "args": [
        "-m",
        "amplifier_module_engram_lite.mcp_server"
      ],
      "env": {
        "ENGRAM_DB_PATH": "~/.engram-lite/memory.db",
        "ENGRAM_EMBEDDING_PROVIDER": "openai",
        "ENGRAM_LOG_LEVEL": "WARNING"
      }
    }
  }
}
```

### 6.2 Alternative: Direct Script Path

If `engram-lite` is installed in a virtualenv or non-standard location:

```json
{
  "mcpServers": {
    "engram-lite": {
      "command": "/path/to/venv/bin/python3",
      "args": [
        "/path/to/engram-lite/claude-code/mcp/server.py"
      ],
      "env": {
        "ENGRAM_DB_PATH": "~/.engram-lite/memory.db",
        "OPENAI_API_KEY": "sk-..."
      }
    }
  }
}
```

### 6.3 Per-Project vs Global Registration

| Scope | File Location | When to Use |
|---|---|---|
| Per-project | `<project-root>/.mcp.json` | Project-specific DB path, team-shared config |
| Global (user) | `~/.mcp.json` | Shared across all projects, single DB |
| Claude Code settings | `~/.claude/settings.json` | Managed via Claude Code UI |

### 6.4 Verifying MCP Registration

After adding `.mcp.json`, restart Claude Code and verify:

```bash
# The MCP server should appear in Claude Code's tool list
# You can test the server standalone:
echo '{"jsonrpc": "2.0", "method": "tools/list", "id": 1}' | \
  python3 -m amplifier_module_engram_lite.mcp_server 2>/dev/null
```

---

## 7. Claude Code Settings

### 7.1 `.claude/settings.json`

```json
{
  "permissions": {
    "allow": [
      "mcp__engram-lite__memory_capture",
      "mcp__engram-lite__memory_recall",
      "mcp__engram-lite__memory_search",
      "mcp__engram-lite__memory_update",
      "mcp__engram-lite__memory_relate",
      "mcp__engram-lite__memory_forget",
      "mcp__engram-lite__memory_graph_explore",
      "mcp__engram-lite__memory_stats"
    ]
  },
  "env": {
    "ENGRAM_DB_PATH": "~/.engram-lite/memory.db",
    "ENGRAM_EMBEDDING_PROVIDER": "openai",
    "ENGRAM_HOT_CONTEXT_LIMIT": "20",
    "ENGRAM_AUTO_RECALL": "true",
    "ENGRAM_AUTO_CAPTURE": "true",
    "ENGRAM_LOG_LEVEL": "WARNING"
  }
}
```

### 7.2 Permission Model

Claude Code requires explicit permission for MCP tools. The `allow` list uses the pattern:

```
mcp__{server-name}__{tool-name}
```

To allow all engram-lite tools at once, you can use the wildcard if supported by your Claude Code version:

```json
{
  "permissions": {
    "allow": [
      "mcp__engram-lite__*"
    ]
  }
}
```

### 7.3 Settings Hierarchy

Settings are resolved in order (highest priority first):

1. **Project settings:** `<project-root>/.claude/settings.json`
2. **User settings:** `~/.claude/settings.json`
3. **Defaults**

---

## 8. Slash Commands

Slash commands provide explicit, user-initiated memory operations. They are markdown files in `.claude/commands/` that define prompts the agent executes when the user types the command.

### 8.1 `.claude/commands/memory-recall.md`

```markdown
Search your persistent memory for information related to the user's query.

User query: $ARGUMENTS

Instructions:
1. Call memory_recall with the user's query.
2. If no arguments provided, ask the user what to search for.
3. Display the results in a readable format:
   - For each memory: show content, domain, confidence, and when it was created.
   - Group by domain if there are many results.
4. If no results found, say so and suggest broadening the search.
5. Do NOT silently use the results — this is an explicit user request, so show them.
```

### 8.2 `.claude/commands/memory-capture.md`

```markdown
Explicitly capture a piece of knowledge to persistent memory.

Content to capture: $ARGUMENTS

Instructions:
1. If arguments are provided, capture them as a memory:
   - Analyze the content to determine content_type, importance, and domain.
   - Rewrite the content conclusion-first if it isn't already.
   - Call memory_capture with the structured parameters.
   - Confirm to the user what was captured (this is an explicit request, so confirmation is appropriate).

2. If no arguments are provided, ask the user what they'd like to capture.

3. After capturing, run a quick cross-reference:
   - Call memory_recall with the captured content to find related memories.
   - If contradictions or duplicates are found, inform the user and ask how to proceed.

4. Show a summary:
   - Memory ID
   - Stored content (as rewritten)
   - Domain assigned
   - Confidence score
   - Any relations created
```

### 8.3 `.claude/commands/memory-stats.md`

```markdown
Display statistics about the persistent memory system.

Instructions:
1. Call memory_stats() to get system statistics.
2. Display the results in a well-formatted summary:

   ## Memory Statistics

   **Total memories:** {total}
   **Database size:** {db_size_bytes formatted as KB/MB}

   ### By Domain
   {table of domain → count}

   ### By Content Type
   {table of type → count}

   ### By Importance
   {table of importance → count}

   ### Confidence Distribution
   {histogram: 0.0-0.2, 0.2-0.4, 0.4-0.6, 0.6-0.8, 0.8-1.0 → count}

3. If there are memories with low confidence (< 0.3), mention how many are
   candidates for cleanup.
4. If there are contradictions in the graph, mention the count.
```

### 8.4 `.claude/commands/memory-forget.md`

```markdown
Delete a memory from the persistent store.

Memory identifier: $ARGUMENTS

Instructions:
1. If a memory ID is provided (e.g., "mem_a1b2c3d4"), look it up directly.
2. If a text description is provided, search for matching memories using memory_search.
3. If no arguments, ask the user what to forget.

4. Before deleting:
   - Show the memory content, domain, confidence, and creation date.
   - Ask for confirmation: "Are you sure you want to forget this memory?"
   - This is a destructive operation — always confirm.

5. If confirmed:
   - Call memory_forget(memory_id, reason="user requested deletion").
   - Confirm deletion.
   - Check if other memories had relations to this one and inform the user.

6. If multiple memories match the search:
   - List them with IDs and ask the user to specify which one(s) to delete.
   - Support "all" to delete all matches (with extra confirmation).
```

---

## 9. Installation

### 9.1 Quick Install

```bash
# Install engram-lite with all dependencies
pip install "engram-lite[all]"

# Run the installer
engram-lite install --platform claude-code
```

### 9.2 `install.sh` — Full Installation Script

```bash
#!/usr/bin/env bash
# engram-lite installer for Claude Code
#
# This script:
# 1. Verifies prerequisites
# 2. Installs the Python package (if not already installed)
# 3. Creates the plugin directory structure
# 4. Initializes the SQLite database
# 5. Registers the MCP server
# 6. Verifies the embedding provider
# 7. Prints setup summary

set -euo pipefail

# ---------------------------------------------------------------------------
# Colors and helpers
# ---------------------------------------------------------------------------
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

info()  { echo -e "${BLUE}[INFO]${NC}  $*"; }
ok()    { echo -e "${GREEN}[OK]${NC}    $*"; }
warn()  { echo -e "${YELLOW}[WARN]${NC}  $*"; }
error() { echo -e "${RED}[ERROR]${NC} $*" >&2; }

# ---------------------------------------------------------------------------
# Configuration
# ---------------------------------------------------------------------------
DB_PATH="${ENGRAM_DB_PATH:-$HOME/.engram-lite/memory.db}"
PLUGIN_DIR="${ENGRAM_PLUGIN_DIR:-.}"
EMBEDDING_PROVIDER="${ENGRAM_EMBEDDING_PROVIDER:-openai}"

echo ""
echo "╔══════════════════════════════════════════════════════════╗"
echo "║          engram-lite installer for Claude Code         ║"
echo "╚══════════════════════════════════════════════════════════╝"
echo ""

# ---------------------------------------------------------------------------
# Step 1: Check prerequisites
# ---------------------------------------------------------------------------
info "Checking prerequisites..."

# Python 3.11+
if ! command -v python3 &>/dev/null; then
    error "Python 3 not found. Please install Python 3.11 or later."
    exit 1
fi

PYTHON_VERSION=$(python3 -c "import sys; print(f'{sys.version_info.major}.{sys.version_info.minor}')")
PYTHON_MAJOR=$(echo "$PYTHON_VERSION" | cut -d. -f1)
PYTHON_MINOR=$(echo "$PYTHON_VERSION" | cut -d. -f2)

if [ "$PYTHON_MAJOR" -lt 3 ] || ([ "$PYTHON_MAJOR" -eq 3 ] && [ "$PYTHON_MINOR" -lt 11 ]); then
    error "Python 3.11+ required. Found: Python $PYTHON_VERSION"
    exit 1
fi
ok "Python $PYTHON_VERSION"

# sqlite3 CLI (for health checks)
if command -v sqlite3 &>/dev/null; then
    ok "sqlite3 CLI available"
else
    warn "sqlite3 CLI not found — health checks will use Python fallback"
fi

# ---------------------------------------------------------------------------
# Step 2: Install Python package
# ---------------------------------------------------------------------------
info "Checking engram-lite package..."

if python3 -c "import amplifier_module_engram_lite" &>/dev/null 2>&1; then
    INSTALLED_VERSION=$(python3 -c "import amplifier_module_engram_lite; print(amplifier_module_engram_lite.__version__)")
    ok "engram-lite $INSTALLED_VERSION already installed"
else
    info "Installing engram-lite..."
    pip install "engram-lite[all]" || {
        error "Failed to install engram-lite. Try: pip install 'engram-lite[all]'"
        exit 1
    }
    ok "engram-lite installed"
fi

# ---------------------------------------------------------------------------
# Step 3: Create directory structure
# ---------------------------------------------------------------------------
info "Creating directory structure..."

# Database directory
DB_DIR=$(dirname "${DB_PATH/#\~/$HOME}")
mkdir -p "$DB_DIR"
ok "Database directory: $DB_DIR"

# Plugin directories
mkdir -p "$PLUGIN_DIR/.claude-plugin"
mkdir -p "$PLUGIN_DIR/hooks"
mkdir -p "$PLUGIN_DIR/mcp"
mkdir -p "$PLUGIN_DIR/.claude/commands"
ok "Plugin directories created"

# ---------------------------------------------------------------------------
# Step 4: Copy plugin files
# ---------------------------------------------------------------------------
info "Installing plugin files..."

# Get the package's installed location for copying bundled files
PACKAGE_DIR=$(python3 -c "import amplifier_module_engram_lite; import os; print(os.path.dirname(amplifier_module_engram_lite.__file__))")

# Copy plugin.json
engram-lite generate --template plugin.json > "$PLUGIN_DIR/.claude-plugin/plugin.json" 2>/dev/null || {
    warn "Could not generate plugin.json via CLI — using template"
}

# Copy hook scripts
for HOOK in session-start prompt-submit stop; do
    engram-lite generate --template "hooks/$HOOK.sh" > "$PLUGIN_DIR/hooks/$HOOK.sh" 2>/dev/null || true
    chmod +x "$PLUGIN_DIR/hooks/$HOOK.sh" 2>/dev/null || true
done
ok "Hook scripts installed"

# Copy MCP server
engram-lite generate --template mcp/server.py > "$PLUGIN_DIR/mcp/server.py" 2>/dev/null || true
ok "MCP server installed"

# Copy slash commands
for CMD in memory-recall memory-capture memory-stats memory-forget; do
    engram-lite generate --template "commands/$CMD.md" > "$PLUGIN_DIR/.claude/commands/$CMD.md" 2>/dev/null || true
done
ok "Slash commands installed"

# ---------------------------------------------------------------------------
# Step 5: Initialize database
# ---------------------------------------------------------------------------
info "Initializing database..."

EXPANDED_DB_PATH="${DB_PATH/#\~/$HOME}"

if [ -f "$EXPANDED_DB_PATH" ]; then
    ok "Database already exists at $EXPANDED_DB_PATH"
    # Run migrations if needed
    engram-lite migrate --db-path "$EXPANDED_DB_PATH" 2>/dev/null || true
else
    engram-lite init --db-path "$EXPANDED_DB_PATH" || {
        error "Failed to initialize database"
        exit 1
    }
    ok "Database initialized at $EXPANDED_DB_PATH"
fi

# ---------------------------------------------------------------------------
# Step 6: Register MCP server
# ---------------------------------------------------------------------------
info "Registering MCP server..."

MCP_JSON="$PLUGIN_DIR/.mcp.json"

# Check for existing .mcp.json
if [ -f "$MCP_JSON" ]; then
    # Check if engram-lite is already registered
    if python3 -c "
import json
with open('$MCP_JSON') as f:
    cfg = json.load(f)
if 'engram-lite' in cfg.get('mcpServers', {}):
    exit(0)
exit(1)
" 2>/dev/null; then
        ok "MCP server already registered in .mcp.json"
    else
        info "Adding engram-lite to existing .mcp.json..."
        python3 -c "
import json
with open('$MCP_JSON') as f:
    cfg = json.load(f)
cfg.setdefault('mcpServers', {})['engram-lite'] = {
    'command': 'python3',
    'args': ['-m', 'amplifier_module_engram_lite.mcp_server'],
    'env': {
        'ENGRAM_DB_PATH': '$DB_PATH',
        'ENGRAM_EMBEDDING_PROVIDER': '$EMBEDDING_PROVIDER'
    }
}
with open('$MCP_JSON', 'w') as f:
    json.dump(cfg, f, indent=2)
"
        ok "MCP server added to .mcp.json"
    fi
else
    cat > "$MCP_JSON" <<EOF
{
  "mcpServers": {
    "engram-lite": {
      "command": "python3",
      "args": ["-m", "amplifier_module_engram_lite.mcp_server"],
      "env": {
        "ENGRAM_DB_PATH": "$DB_PATH",
        "ENGRAM_EMBEDDING_PROVIDER": "$EMBEDDING_PROVIDER"
      }
    }
  }
}
EOF
    ok "MCP server registered in .mcp.json"
fi

# ---------------------------------------------------------------------------
# Step 7: Verify embedding provider
# ---------------------------------------------------------------------------
info "Checking embedding provider ($EMBEDDING_PROVIDER)..."

case "$EMBEDDING_PROVIDER" in
    openai)
        if [ -z "${OPENAI_API_KEY:-}" ]; then
            warn "OPENAI_API_KEY not set. Memory capture will fail until set."
            warn "Set it: export OPENAI_API_KEY=sk-..."
        else
            # Test with a minimal embedding request
            if engram-lite check-embeddings --provider openai 2>/dev/null; then
                ok "OpenAI embedding provider verified"
            else
                warn "OpenAI embedding test failed — check your API key"
            fi
        fi
        ;;
    azure)
        if [ -z "${AZURE_OPENAI_API_KEY:-}" ] || [ -z "${AZURE_OPENAI_ENDPOINT:-}" ]; then
            warn "Azure OpenAI credentials not fully configured."
            warn "Required: AZURE_OPENAI_ENDPOINT, AZURE_OPENAI_API_KEY, AZURE_OPENAI_EMBEDDING_DEPLOYMENT"
        else
            ok "Azure OpenAI credentials found"
        fi
        ;;
    ollama)
        OLLAMA_URL="${ENGRAM_OLLAMA_URL:-http://localhost:11434}"
        if curl -s "$OLLAMA_URL/api/tags" &>/dev/null; then
            ok "Ollama server reachable at $OLLAMA_URL"
        else
            warn "Ollama server not reachable at $OLLAMA_URL"
            warn "Start it: ollama serve"
        fi
        ;;
    *)
        warn "Unknown embedding provider: $EMBEDDING_PROVIDER"
        ;;
esac

# ---------------------------------------------------------------------------
# Summary
# ---------------------------------------------------------------------------
echo ""
echo "╔══════════════════════════════════════════════════════════╗"
echo "║                  Installation Complete                   ║"
echo "╚══════════════════════════════════════════════════════════╝"
echo ""
echo "  Database:     $EXPANDED_DB_PATH"
echo "  Plugin dir:   $PLUGIN_DIR"
echo "  MCP server:   Registered in .mcp.json"
echo "  Embeddings:   $EMBEDDING_PROVIDER"
echo ""
echo "  Next steps:"
echo "    1. Restart Claude Code to load the plugin"
echo "    2. Try: /memory-stats to verify the system is working"
echo "    3. Start a conversation — memory capture begins automatically"
echo ""

exit 0
```

### 9.3 Manual Installation Steps

If you prefer manual installation over the script:

```bash
# 1. Install the package
pip install "engram-lite[all]"

# 2. Initialize the database
engram-lite init

# 3. Copy plugin files to your project
# (or clone the engram-lite repo and symlink)
cp -r engram-lite/claude-code/.claude-plugin .
cp -r engram-lite/claude-code/hooks .
cp -r engram-lite/claude-code/mcp .
cp -r engram-lite/claude-code/.claude .
chmod +x hooks/*.sh

# 4. Create .mcp.json in project root
cat > .mcp.json <<'EOF'
{
  "mcpServers": {
    "engram-lite": {
      "command": "python3",
      "args": ["-m", "amplifier_module_engram_lite.mcp_server"],
      "env": {
        "ENGRAM_DB_PATH": "~/.engram-lite/memory.db"
      }
    }
  }
}
EOF

# 5. Set your API key
export OPENAI_API_KEY=sk-...

# 6. Restart Claude Code
```

### 9.4 Uninstallation

```bash
# Remove plugin files
rm -rf .claude-plugin hooks/session-start.sh hooks/prompt-submit.sh hooks/stop.sh
rm -rf mcp/server.py .claude/commands/memory-*.md

# Remove MCP registration (edit .mcp.json to remove engram-lite entry)
python3 -c "
import json
with open('.mcp.json') as f:
    cfg = json.load(f)
cfg.get('mcpServers', {}).pop('engram-lite', None)
with open('.mcp.json', 'w') as f:
    json.dump(cfg, f, indent=2)
"

# Optionally remove the database (WARNING: deletes all memories)
# rm -rf ~/.engram-lite

# Uninstall the package
pip uninstall engram-lite
```

---

## 10. Configuration

### 10.1 Environment Variables

All configuration is via environment variables. Set them in your shell profile, `.claude/settings.json`, or `.mcp.json`'s `env` block.

| Variable | Type | Default | Description |
|---|---|---|---|
| `ENGRAM_DB_PATH` | `string` | `~/.engram-lite/memory.db` | SQLite database file path |
| `ENGRAM_EMBEDDING_PROVIDER` | `string` | `openai` | Provider: `openai`, `azure`, `ollama` |
| `ENGRAM_EMBEDDING_MODEL` | `string` | `text-embedding-3-small` | Embedding model name |
| `ENGRAM_EMBEDDING_DIMENSIONS` | `int` | `1536` | Embedding vector dimensions |
| `ENGRAM_HOT_CONTEXT_LIMIT` | `int` | `20` | Max memories at session start |
| `ENGRAM_AUTO_RECALL` | `bool` | `true` | Enable per-prompt recall reminders |
| `ENGRAM_AUTO_CAPTURE` | `bool` | `true` | Enable post-response capture reminders |
| `ENGRAM_LOG_LEVEL` | `string` | `WARNING` | Log level: `DEBUG`, `INFO`, `WARNING`, `ERROR` |
| `ENGRAM_LOG_FILE` | `string` | `~/.engram-lite/engram-lite.log` | Log file path |
| `OPENAI_API_KEY` | `string` | — | OpenAI API key (required for `openai` provider) |
| `AZURE_OPENAI_ENDPOINT` | `string` | — | Azure OpenAI endpoint URL |
| `AZURE_OPENAI_API_KEY` | `string` | — | Azure OpenAI API key |
| `AZURE_OPENAI_EMBEDDING_DEPLOYMENT` | `string` | — | Azure deployment name |
| `AZURE_OPENAI_API_VERSION` | `string` | `2024-10-21` | Azure API version |
| `ENGRAM_OLLAMA_URL` | `string` | `http://localhost:11434` | Ollama server URL |
| `ENGRAM_OLLAMA_MODEL` | `string` | `nomic-embed-text` | Ollama embedding model |
| `ENGRAM_RECALL_LIMIT` | `int` | `5` | Default recall result limit |
| `ENGRAM_RECALL_THRESHOLD` | `float` | `0.3` | Minimum recall relevance score |
| `ENGRAM_DEFAULT_SPACE` | `string` | `project` | Default memory space: `user` or `project` |

### 10.2 Configuration Locations

Configuration values can be set in multiple places. Resolution order (highest priority first):

1. **Tool call parameters** — values passed directly by the agent override everything.
2. **`.claude/settings.json` env block** — project-level environment variables.
3. **`.mcp.json` env block** — MCP server-specific environment variables.
4. **Shell environment** — exported environment variables.
5. **Defaults** — hardcoded defaults in the code.

### 10.3 Per-Project Database

To use a separate database per project (recommended for team settings):

```json
// .mcp.json
{
  "mcpServers": {
    "engram-lite": {
      "command": "python3",
      "args": ["-m", "amplifier_module_engram_lite.mcp_server"],
      "env": {
        "ENGRAM_DB_PATH": "./.engram-lite/memory.db"
      }
    }
  }
}
```

Add to `.gitignore`:

```
.engram-lite/
```

### 10.4 Shared Team Database

For teams that want shared memory (e.g., project conventions):

```json
// .mcp.json (committed to repo)
{
  "mcpServers": {
    "engram-lite": {
      "command": "python3",
      "args": ["-m", "amplifier_module_engram_lite.mcp_server"],
      "env": {
        "ENGRAM_DB_PATH": "./shared-memory/project.db",
        "ENGRAM_DEFAULT_SPACE": "project"
      }
    }
  }
}
```

Include the database in version control. Team members' personal memories still route to `user` space in their local `~/.engram-lite/memory.db`.

---

## 11. Marketplace Entry

### 11.1 `.claude-plugin/marketplace.json`

```json
{
  "name": "engram-lite",
  "display_name": "Canvas Memory",
  "version": "0.1.0",
  "description": "Persistent AI memory with semantic recall. Remembers your project architecture, preferences, decisions, and patterns across sessions.",
  "long_description": "Canvas Memory gives your Claude Code assistant persistent memory powered by SQLite-vec. It automatically captures knowledge from conversations — project architecture, your preferences, technical decisions, debugging insights — and recalls them in future sessions using dual-route retrieval (fast vector similarity + knowledge graph traversal). The system operates silently: you'll never see memory operations in the output, but your assistant will consistently remember context from past sessions.",
  "author": {
    "name": "engram-lite",
    "url": "https://github.com/engram-lite"
  },
  "repository": "https://github.com/engram-lite/engram-lite",
  "license": "MIT",
  "categories": [
    "productivity",
    "memory",
    "knowledge-management"
  ],
  "tags": [
    "memory",
    "persistent",
    "knowledge-graph",
    "semantic-search",
    "sqlite-vec",
    "embeddings",
    "cross-session"
  ],
  "icon": "brain",
  "screenshots": [],

  "requirements": {
    "python": ">=3.11",
    "packages": [
      "engram-lite>=0.1.0"
    ],
    "services": {
      "optional": [
        {
          "name": "OpenAI API",
          "description": "For text-embedding-3-small embeddings (default provider)",
          "env_var": "OPENAI_API_KEY"
        },
        {
          "name": "Ollama",
          "description": "For local air-gapped embeddings",
          "url": "http://localhost:11434"
        }
      ]
    }
  },

  "features": {
    "hooks": {
      "SessionStart": "Loads relevant memories at session start",
      "UserPromptSubmit": "Prompts recall for context-relevant queries",
      "Stop": "Triggers knowledge capture after responses"
    },
    "mcp_tools": [
      "memory_capture",
      "memory_recall",
      "memory_search",
      "memory_update",
      "memory_relate",
      "memory_forget",
      "memory_graph_explore",
      "memory_stats"
    ],
    "slash_commands": [
      "/memory-recall",
      "/memory-capture",
      "/memory-stats",
      "/memory-forget"
    ]
  },

  "configuration_schema": {
    "ENGRAM_DB_PATH": {
      "type": "string",
      "default": "~/.engram-lite/memory.db",
      "description": "Database file location",
      "required": false
    },
    "ENGRAM_EMBEDDING_PROVIDER": {
      "type": "string",
      "default": "openai",
      "enum": ["openai", "azure", "ollama"],
      "description": "Embedding provider",
      "required": false
    },
    "OPENAI_API_KEY": {
      "type": "string",
      "description": "OpenAI API key (required for openai provider)",
      "sensitive": true,
      "required": false
    }
  },

  "install": {
    "command": "engram-lite install --platform claude-code",
    "post_install_message": "Restart Claude Code to activate the memory system. Your assistant will automatically begin remembering context from your conversations."
  },

  "uninstall": {
    "command": "engram-lite uninstall --platform claude-code",
    "preserves_data": true,
    "data_location": "~/.engram-lite/"
  },

  "compatibility": {
    "claude_code": ">=1.0",
    "platforms": ["macos", "linux", "windows"]
  }
}
```

### 11.2 Marketplace Fields Reference

| Field | Required | Description |
|---|---|---|
| `name` | Yes | Unique identifier (lowercase, hyphens) |
| `display_name` | Yes | Human-readable name |
| `version` | Yes | SemVer version |
| `description` | Yes | Short description (< 200 chars) |
| `long_description` | No | Detailed description for the listing page |
| `author` | Yes | Author name and URL |
| `repository` | No | Source code URL |
| `license` | Yes | SPDX license identifier |
| `categories` | Yes | Marketplace categories |
| `tags` | No | Search tags |
| `icon` | No | Icon identifier or path |
| `requirements` | No | System and package requirements |
| `features` | No | Summary of hooks, tools, and commands |
| `configuration_schema` | No | User-configurable settings |
| `install` | No | Installation instructions |
| `uninstall` | No | Uninstall instructions |
| `compatibility` | No | Platform compatibility |

---

## Appendix A: Complete File Listing

All files that constitute the Claude Code plugin, with their roles:

| File | Role | Committed to Git? |
|---|---|---|
| `.claude-plugin/plugin.json` | Plugin manifest | Yes |
| `.claude-plugin/marketplace.json` | Marketplace listing | Yes |
| `hooks/session-start.sh` | SessionStart hook | Yes |
| `hooks/prompt-submit.sh` | UserPromptSubmit hook | Yes |
| `hooks/stop.sh` | Stop hook | Yes |
| `mcp/server.py` | MCP server | Yes |
| `.mcp.json` | MCP registration | Project-specific (yes for team, no for personal) |
| `.claude/settings.json` | Plugin settings | No (contains env vars) |
| `.claude/commands/memory-recall.md` | Slash command | Yes |
| `.claude/commands/memory-capture.md` | Slash command | Yes |
| `.claude/commands/memory-stats.md` | Slash command | Yes |
| `.claude/commands/memory-forget.md` | Slash command | Yes |
| `~/.engram-lite/memory.db` | SQLite database | Never (gitignore) |
| `~/.engram-lite/engram-lite.log` | Log file | Never |
| `~/.engram-lite/capture-cache.jsonl` | Failure cache | Never |

## Appendix B: Troubleshooting

| Symptom | Cause | Fix |
|---|---|---|
| No hot context at session start | DB missing or empty | Run `engram-lite init` |
| Tools not available to agent | MCP server not registered | Check `.mcp.json`, restart Claude Code |
| "Permission denied" on tool use | Tools not in allow list | Add to `.claude/settings.json` permissions |
| Embeddings fail | API key missing or invalid | Set `OPENAI_API_KEY` or configure provider |
| "Database is locked" | Concurrent access conflict | Ensure WAL mode: `engram-lite migrate` |
| Hooks output nothing | Scripts not executable | `chmod +x hooks/*.sh` |
| Slow recall (>5s) | Large DB without index | Run `engram-lite optimize` |
| MCP server crashes on start | Missing dependencies | `pip install "engram-lite[all]"` |
| Slash commands not visible | Wrong directory | Must be in `.claude/commands/` |

## Appendix C: Architecture Diagram

```
User
 │
 ▼
Claude Code ◄────── .claude-plugin/plugin.json (hooks registration)
 │
 ├── SessionStart ──► hooks/session-start.sh ──► <system-reminder> hot context
 │                         │
 │                         └── engram-lite CLI ──► SQLite DB
 │
 ├── UserPromptSubmit ──► hooks/prompt-submit.sh ──► <system-reminder> recall hint
 │
 ├── Agent Processing
 │    │
 │    ├── memory_recall() ──► MCP Server ──► MemoryStore ──► SQLite + sqlite-vec
 │    │                            │
 │    │                            └── EmbeddingProvider ──► OpenAI / Azure / Ollama
 │    │
 │    ├── (generates response)
 │    │
 │    └── memory_capture() ──► MCP Server ──► MemoryStore ──► SQLite + sqlite-vec
 │
 ├── Stop ──► hooks/stop.sh ──► <system-reminder> capture hint
 │
 └── Slash Commands
      ├── /memory-recall  ──► agent invokes memory_recall via MCP
      ├── /memory-capture ──► agent invokes memory_capture via MCP
      ├── /memory-stats   ──► agent invokes memory_stats via MCP
      └── /memory-forget  ──► agent invokes memory_forget via MCP
```
