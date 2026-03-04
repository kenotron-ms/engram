# SPEC-AMPLIFIER-BUNDLE: Amplifier Bundle Specification

**System:** engram-lite
**Version:** 0.1.0
**Status:** Draft
**Last Updated:** 2026-03-03

---

## Table of Contents

1. [Overview](#1-overview)
2. [Bundle Structure](#2-bundle-structure)
3. [Behavior YAML](#3-behavior-yaml)
4. [Standalone Root Bundle](#4-standalone-root-bundle)
5. [Hook Module](#5-hook-module)
6. [Tool Module](#6-tool-module)
7. [Context Files](#7-context-files)
8. [Installation](#8-installation)
9. [Configuration](#9-configuration)
10. [Examples](#10-examples)

---

## 1. Overview

engram-lite integrates with Amplifier through the **behavior bundle** pattern. A behavior bundle combines hooks, tools, and context into a single distributable unit that can be composed into any Amplifier root bundle via deep-merge.

### Integration Points

| Amplifier Concept | engram-lite Component | Purpose |
|---|---|---|
| Hook module | `amplifier_module_engram_lite_amplifier_hook` | Injects memory context at session start, recall reminders per prompt, capture reminders after response |
| Tool module | `amplifier_module_engram_lite_amplifier_tool` | Exposes 8 memory tools to the agent |
| Context file | `memory-protocol.md` | Behavioral protocol injected at session start |
| Context file | `tool-guide.md` | Tool usage guidance for the agent |
| Behavior YAML | `engram-lite.yaml` | Declarative composition of all components |
| Root bundle | `bundle.md` | Standalone bundle for direct use |

### Prerequisites

- Amplifier >= 0.9.0
- Python >= 3.11
- `engram-lite` Python package installed
- An embedding provider configured (OpenAI, Azure OpenAI, or Ollama)

---

## 2. Bundle Structure

```
amplifier/
├── behaviors/
│   └── engram-lite.yaml          # Distributable behavior bundle
├── context/
│   ├── memory-protocol.md          # Behavioral protocol (session-start injection)
│   └── tool-guide.md               # Tool usage guide for the agent
└── bundle.md                       # Standalone root bundle (for direct use)
```

### Supporting Python Packages

The behavior bundle references two Python packages that are installed as entry points:

```
engram-lite-amplifier-hook/       # Hook module package
├── pyproject.toml
└── src/
    └── amplifier_module_engram_lite_amplifier_hook/
        ├── __init__.py             # mount() function + hook implementations
        └── config.py               # Configuration dataclass

engram-lite-amplifier-tool/       # Tool module package
├── pyproject.toml
└── src/
    └── amplifier_module_engram_lite_amplifier_tool/
        ├── __init__.py             # mount() function + tool registration
        └── schemas.py              # JSON schemas for tool parameters
```

In practice, both are part of the main `engram-lite` Python package and are exposed via entry points in the top-level `pyproject.toml`. They are documented as separate logical modules here for clarity.

---

## 3. Behavior YAML

### `amplifier/behaviors/engram-lite.yaml`

This is the primary distributable artifact. Other Amplifier bundles include this via deep-merge to gain memory capabilities.

```yaml
# engram-lite behavior bundle
# Provides persistent AI memory via SQLite-vec with dual-route retrieval.
#
# Usage: Include in your root bundle's behaviors list, or deep-merge
# into an existing bundle configuration.
#
# Requires: engram-lite Python package installed

name: engram-lite
version: 0.1.0
description: >
  Persistent memory system for AI agents. Provides automatic knowledge
  capture, semantic recall, and cross-session context continuity via
  a SQLite-vec backed store with dual-route retrieval (fast vector KNN
  and deliberate graph traversal).

# ---------------------------------------------------------------------------
# Tools: Memory operations exposed to the agent
# ---------------------------------------------------------------------------
tools:
  module: amplifier_module_engram_lite_amplifier_tool
  # The tool module is registered via the amplifier.modules entry point.
  # It exposes the following tools:
  #   - memory_capture     : Store new knowledge
  #   - memory_recall      : Retrieve relevant memories (auto-routed)
  #   - memory_search      : Explicit search with filters
  #   - memory_update      : Modify existing memory metadata
  #   - memory_relate      : Create/update graph edges between memories
  #   - memory_forget      : Soft-delete a memory
  #   - memory_graph_explore : Traverse the memory graph from a node
  #   - memory_stats       : Return memory system statistics

# ---------------------------------------------------------------------------
# Hooks: Lifecycle event handlers for the RETRIEVE-RESPOND-CAPTURE loop
# ---------------------------------------------------------------------------
hooks:
  module: amplifier_module_engram_lite_amplifier_hook
  # The hook module is registered via the amplifier.modules entry point.
  # It handles three lifecycle events:
  events:
    session:start:
      # Loads hot context (critical/high importance memories) and injects
      # them as a <system-reminder> block. Also injects the behavioral
      # protocol from the context file.
      handler: on_session_start
      timeout: 5000        # 5 seconds — DB query + formatting
      blocking: true       # Must complete before first agent response
      config:
        hot_context_limit: 20
        token_budget: 2000
        include_protocol: true

    prompt:submit:
      # Injects a per-prompt recall reminder. Extracts the first 50
      # characters of the user's prompt for the memory check hint.
      handler: on_prompt_submit
      timeout: 2000        # 2 seconds
      blocking: false
      config:
        auto_recall: true
        reminder_style: concise   # concise | verbose | minimal

    response:complete:
      # Injects a post-response capture reminder. Triggers the capture
      # decision tree in the agent's next reasoning step.
      handler: on_response_complete
      timeout: 2000        # 2 seconds
      blocking: false
      config:
        auto_capture: true
        reminder_style: concise   # concise | verbose | minimal

# ---------------------------------------------------------------------------
# Context: Files injected into the agent's context at session start
# ---------------------------------------------------------------------------
context:
  files:
    - path: context/memory-protocol.md
      # The behavioral protocol that governs the agent's memory behavior.
      # Injected at session start alongside hot context.
      inject_at: session_start
      priority: high

    - path: context/tool-guide.md
      # Brief tool usage guidance. Tells the agent what each tool does
      # and when to use it.
      inject_at: session_start
      priority: medium

# ---------------------------------------------------------------------------
# Config: Default configuration values (overridable per-installation)
# ---------------------------------------------------------------------------
config:
  # Database
  db_path: ~/.engram-lite/memory.db
  db_journal_mode: wal            # WAL mode for concurrent reads

  # Embedding provider
  embedding_provider: openai       # openai | azure | ollama
  embedding_model: text-embedding-3-small
  embedding_dimensions: 1536

  # OpenAI-specific
  openai_api_key: ${OPENAI_API_KEY}

  # Azure-specific (used when embedding_provider: azure)
  azure_endpoint: ${AZURE_OPENAI_ENDPOINT}
  azure_api_key: ${AZURE_OPENAI_API_KEY}
  azure_deployment: ${AZURE_OPENAI_EMBEDDING_DEPLOYMENT}
  azure_api_version: "2024-10-21"

  # Ollama-specific (used when embedding_provider: ollama)
  ollama_base_url: http://localhost:11434
  ollama_model: nomic-embed-text

  # Retrieval
  auto_recall: true               # Inject recall reminders per prompt
  auto_capture: true              # Inject capture reminders after response
  hot_context_limit: 20           # Max memories in hot context
  recall_limit: 5                 # Default recall result limit
  recall_threshold: 0.3           # Minimum relevance score for recall
  system1_weight: 0.6             # Weight for System 1 (fast) retrieval
  system2_weight: 0.4             # Weight for System 2 (graph) retrieval

  # Privacy
  default_space: project          # user | project
  pii_detection: true             # Enable basic PII detection on capture

  # Maintenance
  confidence_decay_enabled: true
  decay_grace_period_days: 90
  decay_rate_per_30d: 0.05
  min_confidence: 0.20
  gc_threshold: 0.15              # Confidence below which memories are GC candidates

  # Logging
  log_level: WARNING              # DEBUG | INFO | WARNING | ERROR
  log_file: ~/.engram-lite/engram-lite.log
```

---

## 4. Standalone Root Bundle

### `amplifier/bundle.md`

This is a complete root bundle for users who want engram-lite as their primary (or only) Amplifier behavior. It uses markdown format with YAML frontmatter.

```markdown
---
name: engram-lite-bundle
version: 0.1.0
description: >
  Standalone Amplifier bundle providing persistent AI memory.
  Uses engram-lite for cross-session knowledge retention with
  dual-route retrieval (vector KNN + graph traversal).

behaviors:
  - engram-lite

config:
  engram-lite:
    db_path: ~/.engram-lite/memory.db
    embedding_provider: openai
    embedding_model: text-embedding-3-small
    auto_recall: true
    auto_capture: true
    hot_context_limit: 20
    log_level: WARNING
---

# Canvas Memory

This bundle provides persistent memory for your AI assistant. It automatically
captures knowledge from your conversations and recalls it in future sessions.

## What It Does

- **Remembers** your project architecture, preferences, decisions, and patterns
- **Recalls** relevant context automatically when you ask related questions
- **Learns** from corrections and improves over time
- **Stays silent** — you'll never see memory operations in the output

## Quick Start

1. Install the engram-lite package:
   ```
   pip install engram-lite
   ```

2. Set your embedding provider API key:
   ```
   export OPENAI_API_KEY=sk-...
   ```

3. Add this bundle to your Amplifier configuration.

The memory system activates automatically. No further action needed.

## Configuration

Override defaults in the YAML frontmatter above or via environment variables:

| Variable | Description | Default |
|---|---|---|
| `ENGRAM_DB_PATH` | Database file location | `~/.engram-lite/memory.db` |
| `ENGRAM_EMBEDDING_PROVIDER` | `openai`, `azure`, or `ollama` | `openai` |
| `ENGRAM_HOT_CONTEXT_LIMIT` | Max memories at session start | `20` |
| `OPENAI_API_KEY` | OpenAI API key (if using openai provider) | — |

## Memory Tools

The following tools are available to the AI agent (used automatically):

- `memory_capture` — Store new knowledge
- `memory_recall` — Retrieve relevant memories
- `memory_search` — Search with explicit filters
- `memory_update` — Modify memory metadata
- `memory_relate` — Create graph connections
- `memory_forget` — Remove a memory
- `memory_graph_explore` — Traverse the knowledge graph
- `memory_stats` — View memory statistics
```

---

## 5. Hook Module

The hook module is a Python package that implements the three lifecycle hooks for the RETRIEVE-RESPOND-CAPTURE loop.

### 5.1 Package Configuration

#### `pyproject.toml` (hook module entry in top-level engram-lite)

The hooks are part of the main `engram-lite` package. The entry point registration lives in the top-level `pyproject.toml`:

```toml
[project]
name = "engram-lite"
version = "0.1.0"
description = "SQLite-vec persistent memory for AI agents"
requires-python = ">=3.11"
dependencies = [
    "sqlite-vec>=0.1.6",
    "numpy>=1.26",
    "httpx>=0.27",
]

[project.optional-dependencies]
amplifier = [
    "amplifier-sdk>=0.9.0",
]
openai = [
    "openai>=1.50",
]
azure = [
    "openai>=1.50",
]
ollama = [
    "httpx>=0.27",
]
all = [
    "engram-lite[amplifier,openai,azure,ollama]",
]

[project.entry-points."amplifier.modules"]
amplifier_module_engram_lite_amplifier_hook = "amplifier_module_engram_lite.hooks.amplifier_hook"
amplifier_module_engram_lite_amplifier_tool = "amplifier_module_engram_lite.tools"

[build-system]
requires = ["hatchling"]
build-backend = "hatchling.build"
```

### 5.2 Configuration Dataclass

#### `src/amplifier_module_engram_lite/hooks/config.py`

```python
"""Configuration for engram-lite Amplifier hooks."""

from __future__ import annotations

import os
from dataclasses import dataclass, field
from pathlib import Path
from typing import Literal


@dataclass(frozen=True)
class CanvasMemoryHookConfig:
    """Configuration for the engram-lite Amplifier hook module.

    Values are resolved in order:
    1. Explicit config from behavior YAML
    2. Environment variables (ENGRAM_*)
    3. Defaults defined here
    """

    # Database
    db_path: str = field(
        default_factory=lambda: os.environ.get(
            "ENGRAM_DB_PATH",
            str(Path.home() / ".engram-lite" / "memory.db"),
        )
    )

    # Embedding provider
    embedding_provider: Literal["openai", "azure", "ollama"] = field(
        default_factory=lambda: os.environ.get(
            "ENGRAM_EMBEDDING_PROVIDER", "openai"
        )
    )
    embedding_model: str = field(
        default_factory=lambda: os.environ.get(
            "ENGRAM_EMBEDDING_MODEL", "text-embedding-3-small"
        )
    )
    embedding_dimensions: int = 1536

    # Retrieval behavior
    auto_recall: bool = True
    auto_capture: bool = True
    hot_context_limit: int = 20
    token_budget: int = 2000
    recall_limit: int = 5
    recall_threshold: float = 0.3

    # Reminder style
    reminder_style: Literal["concise", "verbose", "minimal"] = "concise"

    # Privacy
    default_space: Literal["user", "project"] = "project"

    # Logging
    log_level: str = "WARNING"

    @classmethod
    def from_hook_config(cls, config: dict | None = None) -> CanvasMemoryHookConfig:
        """Create configuration from Amplifier hook config dict.

        Merges explicit config values over environment variables over defaults.
        """
        if config is None:
            return cls()
        # Filter to only fields that exist on the dataclass
        valid_fields = {f.name for f in cls.__dataclass_fields__.values()}
        filtered = {k: v for k, v in config.items() if k in valid_fields}
        return cls(**filtered)
```

### 5.3 Hook Module Implementation

#### `src/amplifier_module_engram_lite/hooks/amplifier_hook.py`

```python
"""Amplifier hook module for engram-lite.

Implements the RETRIEVE-RESPOND-CAPTURE loop by handling three lifecycle events:
- session:start   → Load hot context + inject behavioral protocol
- prompt:submit   → Inject per-prompt recall reminder
- response:complete → Inject post-response capture reminder

Registered via entry point:
    [project.entry-points."amplifier.modules"]
    amplifier_module_engram_lite_amplifier_hook = "amplifier_module_engram_lite.hooks.amplifier_hook"
"""

from __future__ import annotations

import logging
import sqlite3
from datetime import datetime, timezone
from pathlib import Path
from typing import TYPE_CHECKING, Any

if TYPE_CHECKING:
    pass

logger = logging.getLogger("amplifier_module_engram_lite.hooks")


# ---------------------------------------------------------------------------
# Configuration
# ---------------------------------------------------------------------------

class HookConfig:
    """Resolved hook configuration."""

    def __init__(self, raw_config: dict[str, Any] | None = None) -> None:
        import os

        cfg = raw_config or {}
        self.db_path: str = cfg.get(
            "db_path",
            os.environ.get(
                "ENGRAM_DB_PATH",
                str(Path.home() / ".engram-lite" / "memory.db"),
            ),
        )
        self.hot_context_limit: int = int(cfg.get("hot_context_limit", 20))
        self.token_budget: int = int(cfg.get("token_budget", 2000))
        self.auto_recall: bool = cfg.get("auto_recall", True)
        self.auto_capture: bool = cfg.get("auto_capture", True)
        self.reminder_style: str = cfg.get("reminder_style", "concise")
        self.include_protocol: bool = cfg.get("include_protocol", True)


# ---------------------------------------------------------------------------
# Hot context loader
# ---------------------------------------------------------------------------

def _load_hot_context(config: HookConfig) -> list[dict[str, Any]]:
    """Load critical and high-importance memories from the database.

    Returns a list of memory dicts sorted by importance then confidence.
    Returns an empty list if the database doesn't exist or is inaccessible.
    """
    db_path = Path(config.db_path).expanduser()
    if not db_path.exists():
        logger.info("Database not found at %s — skipping hot context", db_path)
        return []

    try:
        conn = sqlite3.connect(
            str(db_path),
            timeout=1.0,  # Don't wait long if locked
        )
        conn.row_factory = sqlite3.Row
        cursor = conn.execute(
            """
            SELECT id, content, content_type, domain, importance,
                   confidence, tags, created_at, last_accessed
            FROM memories
            WHERE importance IN ('critical', 'high')
              AND confidence >= 0.20
              AND deleted_at IS NULL
            ORDER BY
              CASE importance
                WHEN 'critical' THEN 0
                WHEN 'high' THEN 1
              END,
              confidence DESC,
              last_accessed DESC
            LIMIT ?
            """,
            (config.hot_context_limit,),
        )
        rows = [dict(row) for row in cursor.fetchall()]

        # Touch access timestamps
        now = datetime.now(timezone.utc).isoformat()
        ids = [r["id"] for r in rows]
        if ids:
            placeholders = ",".join("?" for _ in ids)
            conn.execute(
                f"""
                UPDATE memories
                SET last_accessed = ?, access_count = access_count + 1
                WHERE id IN ({placeholders})
                """,
                [now, *ids],
            )
            conn.commit()

        conn.close()
        return rows

    except sqlite3.OperationalError as exc:
        logger.warning("Database access error during hot context load: %s", exc)
        return []
    except Exception as exc:
        logger.error("Unexpected error loading hot context: %s", exc)
        return []


# ---------------------------------------------------------------------------
# Formatters
# ---------------------------------------------------------------------------

def _format_relative_time(iso_timestamp: str | None) -> str:
    """Convert ISO timestamp to human-readable relative time."""
    if not iso_timestamp:
        return "unknown"
    try:
        dt = datetime.fromisoformat(iso_timestamp)
        now = datetime.now(timezone.utc)
        delta = now - dt.replace(tzinfo=timezone.utc)
        days = delta.days
        if days == 0:
            return "today"
        if days == 1:
            return "yesterday"
        if days < 7:
            return f"{days} days ago"
        if days < 30:
            weeks = days // 7
            return f"{weeks} week{'s' if weeks > 1 else ''} ago"
        months = days // 30
        return f"{months} month{'s' if months > 1 else ''} ago"
    except (ValueError, TypeError):
        return "unknown"


def _format_memory(memory: dict[str, Any]) -> str:
    """Format a single memory for hot context display."""
    domain = memory.get("domain", "general")
    content = memory.get("content", "")
    confidence = memory.get("confidence", 0.0)
    last_accessed = _format_relative_time(memory.get("last_accessed"))
    return f"[{domain}] {content} (confidence: {confidence:.2f}, last: {last_accessed})"


def _format_hot_context(memories: list[dict[str, Any]], config: HookConfig) -> str:
    """Format all hot context memories into the session-start injection."""
    if not memories:
        return ""

    # Group by importance
    critical = [m for m in memories if m.get("importance") == "critical"]
    high = [m for m in memories if m.get("importance") == "high"]

    # Collect unique domains
    domains = sorted({m.get("domain", "general") for m in memories})

    parts: list[str] = []
    parts.append(f"[{len(memories)} memories across {', '.join(domains)}]")
    parts.append("")

    if critical:
        parts.append("## Critical Context")
        for m in critical:
            parts.append(_format_memory(m))
        parts.append("")

    if high:
        parts.append("## High-Importance Context")
        for m in high:
            parts.append(_format_memory(m))
        parts.append("")

    return "\n".join(parts)


# ---------------------------------------------------------------------------
# Injection builders
# ---------------------------------------------------------------------------

def _build_session_start_injection(
    hot_context: str,
    config: HookConfig,
) -> str:
    """Build the full session-start <system-reminder> injection."""
    parts: list[str] = []
    parts.append('<system-reminder source="engram-lite">')
    parts.append("MEMORY SYSTEM ACTIVE. Loaded context:")
    parts.append("")

    if hot_context:
        parts.append(hot_context)
    else:
        parts.append("[No memories loaded — new session or empty database]")
        parts.append("")

    parts.append("PROTOCOL:")
    parts.append(
        "- Use memory_recall(query) before responding to queries "
        "that may relate to prior context"
    )
    parts.append(
        "- Use memory_capture(content) after learning new information"
    )
    parts.append("- Never announce memory operations to the user")
    parts.append("</system-reminder>")

    return "\n".join(parts)


def _build_prompt_reminder(prompt_text: str, config: HookConfig) -> str:
    """Build the per-prompt recall reminder injection."""
    if not config.auto_recall:
        return ""

    snippet = prompt_text[:50].replace('"', "'")

    if config.reminder_style == "minimal":
        return (
            f'<system-reminder source="engram-lite">'
            f"Memory? -> memory_recall()"
            f"</system-reminder>"
        )

    if config.reminder_style == "verbose":
        return (
            f'<system-reminder source="engram-lite">\n'
            f'Memory check: Does "{snippet}" relate to prior context?\n'
            f"Consider:\n"
            f"- Have you discussed this topic before?\n"
            f"- Are there stored preferences that affect this response?\n"
            f"- Is this a continuation of a multi-session project?\n"
            f"If yes -> memory_recall(query) before responding.\n"
            f"</system-reminder>"
        )

    # concise (default)
    return (
        f'<system-reminder source="engram-lite">\n'
        f'Memory check: Does "{snippet}" relate to prior context?\n'
        f"If yes -> memory_recall(query) before responding.\n"
        f"</system-reminder>"
    )


def _build_capture_reminder(config: HookConfig) -> str:
    """Build the post-response capture reminder injection."""
    if not config.auto_capture:
        return ""

    if config.reminder_style == "minimal":
        return (
            '<system-reminder source="engram-lite">'
            "New info? -> memory_capture(). Silent."
            "</system-reminder>"
        )

    if config.reminder_style == "verbose":
        return (
            '<system-reminder source="engram-lite">\n'
            "CAPTURE CHECK (silent):\n"
            "1. Did I learn something new from this exchange? "
            "-> memory_capture()\n"
            "2. Did the user correct an error I made? "
            "-> memory_capture(content_type='fact', importance='high')\n"
            "3. Did I observe a preference or recurring pattern? "
            "-> memory_capture(content_type='preference')\n"
            "4. Was a significant decision made? "
            "-> memory_capture(content_type='decision', importance='high')\n"
            "5. Did I learn about project structure or architecture? "
            "-> memory_capture(content_type='architecture')\n"
            "After any capture -> run cross-reference cascade.\n"
            "Execute silently. Do not announce.\n"
            "</system-reminder>"
        )

    # concise (default)
    return (
        '<system-reminder source="engram-lite">\n'
        "CAPTURE CHECK (silent):\n"
        "1. Did I learn something new? -> memory_capture()\n"
        "2. Did I make an error that was corrected? "
        "-> memory_capture(content_type='fact', importance='high')\n"
        "3. Did I observe a preference/pattern? "
        "-> memory_capture(content_type='preference')\n"
        "Execute silently. Do not announce.\n"
        "</system-reminder>"
    )


# ---------------------------------------------------------------------------
# Amplifier mount() entry point
# ---------------------------------------------------------------------------

def mount(app: Any, *, config: dict[str, Any] | None = None) -> None:
    """Mount the engram-lite hook module into Amplifier.

    This is the entry point called by Amplifier when loading the module
    via the `amplifier.modules` entry point.

    Parameters
    ----------
    app:
        The Amplifier application instance. Provides `register_hook()`.
    config:
        Configuration dict from the behavior YAML's hooks.events.*.config.
    """
    cfg = HookConfig(config)

    # Configure logging
    logging.getLogger("amplifier_module_engram_lite").setLevel(
        getattr(logging, cfg.log_level if hasattr(cfg, "log_level") else "WARNING")
    )

    # -----------------------------------------------------------------------
    # Hook: session:start
    # -----------------------------------------------------------------------
    @app.register_hook("session:start")
    def on_session_start(context: dict[str, Any]) -> dict[str, Any]:
        """Load hot context and inject behavioral protocol.

        Returns a HookResult dict with the injection string.

        Parameters
        ----------
        context:
            Session context from Amplifier, including workspace path,
            session ID, and any user configuration.

        Returns
        -------
        dict with:
            - content: str — The <system-reminder> injection string
            - metadata: dict — Hook execution metadata
        """
        try:
            memories = _load_hot_context(cfg)
            hot_context = _format_hot_context(memories, cfg)
            injection = _build_session_start_injection(hot_context, cfg)

            return {
                "content": injection,
                "metadata": {
                    "memories_loaded": len(memories),
                    "domains": list({m.get("domain", "general") for m in memories}),
                    "hook": "session:start",
                    "status": "ok",
                },
            }
        except Exception as exc:
            logger.error("session:start hook failed: %s", exc)
            return {
                "content": "",
                "metadata": {
                    "hook": "session:start",
                    "status": "error",
                    "error": str(exc),
                },
            }

    # -----------------------------------------------------------------------
    # Hook: prompt:submit
    # -----------------------------------------------------------------------
    @app.register_hook("prompt:submit")
    def on_prompt_submit(context: dict[str, Any]) -> dict[str, Any]:
        """Inject per-prompt recall reminder.

        Parameters
        ----------
        context:
            Prompt context from Amplifier, including the user's message text.

        Returns
        -------
        dict with:
            - content: str — The <system-reminder> recall reminder
            - metadata: dict — Hook execution metadata
        """
        try:
            prompt_text = context.get("prompt", context.get("message", ""))
            injection = _build_prompt_reminder(prompt_text, cfg)

            return {
                "content": injection,
                "metadata": {
                    "hook": "prompt:submit",
                    "status": "ok",
                    "auto_recall": cfg.auto_recall,
                },
            }
        except Exception as exc:
            logger.error("prompt:submit hook failed: %s", exc)
            return {
                "content": "",
                "metadata": {
                    "hook": "prompt:submit",
                    "status": "error",
                    "error": str(exc),
                },
            }

    # -----------------------------------------------------------------------
    # Hook: response:complete
    # -----------------------------------------------------------------------
    @app.register_hook("response:complete")
    def on_response_complete(context: dict[str, Any]) -> dict[str, Any]:
        """Inject post-response capture reminder.

        Parameters
        ----------
        context:
            Response context from Amplifier, including the assistant's
            response text and any tool calls made.

        Returns
        -------
        dict with:
            - content: str — The <system-reminder> capture reminder
            - metadata: dict — Hook execution metadata
        """
        try:
            injection = _build_capture_reminder(cfg)

            return {
                "content": injection,
                "metadata": {
                    "hook": "response:complete",
                    "status": "ok",
                    "auto_capture": cfg.auto_capture,
                },
            }
        except Exception as exc:
            logger.error("response:complete hook failed: %s", exc)
            return {
                "content": "",
                "metadata": {
                    "hook": "response:complete",
                    "status": "error",
                    "error": str(exc),
                },
            }

    logger.info("engram-lite hooks mounted successfully")
```

### 5.4 HookResult Format

Every hook returns a dict conforming to this structure:

```python
HookResult = {
    "content": str,      # The injection string (may be empty)
    "metadata": {
        "hook": str,     # Hook name: "session:start" | "prompt:submit" | "response:complete"
        "status": str,   # "ok" | "error"
        "error": str,    # Present only when status == "error"
        # Additional keys vary by hook:
        # session:start → "memories_loaded": int, "domains": list[str]
        # prompt:submit → "auto_recall": bool
        # response:complete → "auto_capture": bool
    }
}
```

---

## 6. Tool Module

The tool module exposes the 8 memory tools to the Amplifier agent via the `amplifier.modules` entry point.

### 6.1 Entry Point Registration

In the top-level `pyproject.toml`:

```toml
[project.entry-points."amplifier.modules"]
amplifier_module_engram_lite_amplifier_tool = "amplifier_module_engram_lite.tools"
```

### 6.2 Tool Module Implementation

#### `src/amplifier_module_engram_lite/tools/__init__.py`

```python
"""Amplifier tool module for engram-lite.

Exposes memory operations as tools available to the AI agent.

Registered via entry point:
    [project.entry-points."amplifier.modules"]
    amplifier_module_engram_lite_amplifier_tool = "amplifier_module_engram_lite.tools"
"""

from __future__ import annotations

import json
import logging
import os
from pathlib import Path
from typing import Any

logger = logging.getLogger("amplifier_module_engram_lite.tools")


def _get_memory_store():
    """Lazy-initialize and return the MemoryStore singleton."""
    # Import here to avoid circular imports and defer DB initialization
    from amplifier_module_engram_lite.db.memory_store import MemoryStore

    db_path = os.environ.get(
        "ENGRAM_DB_PATH",
        str(Path.home() / ".engram-lite" / "memory.db"),
    )
    return MemoryStore(db_path)


# ---------------------------------------------------------------------------
# Tool definitions
# ---------------------------------------------------------------------------

TOOL_DEFINITIONS: list[dict[str, Any]] = [
    {
        "name": "memory_capture",
        "description": (
            "Store new knowledge as a persistent memory. Use after learning "
            "new facts, preferences, decisions, or patterns from the conversation. "
            "Write content conclusion-first. Never capture verbatim code or quotes."
        ),
        "parameters": {
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
    },
    {
        "name": "memory_recall",
        "description": (
            "Retrieve memories relevant to a query. Uses dual-route retrieval: "
            "fast vector similarity (System 1) and graph traversal (System 2). "
            "Use before responding to queries that may relate to prior context."
        ),
        "parameters": {
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
    },
    {
        "name": "memory_search",
        "description": (
            "Search memories with explicit filters. More control than "
            "memory_recall — allows filtering by tags, date range, "
            "confidence, and other metadata fields."
        ),
        "parameters": {
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
    },
    {
        "name": "memory_update",
        "description": (
            "Update metadata on an existing memory. Use to adjust confidence, "
            "add tags, change importance, or modify content."
        ),
        "parameters": {
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
    },
    {
        "name": "memory_relate",
        "description": (
            "Create or update a relationship between two memories in the "
            "knowledge graph. Use during cross-reference cascades."
        ),
        "parameters": {
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
    },
    {
        "name": "memory_forget",
        "description": (
            "Soft-delete a memory. The memory is marked as deleted but "
            "retained for audit. Use when information is superseded, "
            "incorrect, or no longer relevant."
        ),
        "parameters": {
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
    },
    {
        "name": "memory_graph_explore",
        "description": (
            "Traverse the knowledge graph starting from a memory or domain. "
            "Returns connected memories and their relationships. "
            "Use for System 2 deliberate retrieval."
        ),
        "parameters": {
            "type": "object",
            "required": ["start"],
            "properties": {
                "start": {
                    "type": "string",
                    "description": (
                        "Starting point: a memory ID or domain path."
                    ),
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
    },
    {
        "name": "memory_stats",
        "description": (
            "Return statistics about the memory system: total memories, "
            "domain distribution, confidence histogram, storage size."
        ),
        "parameters": {
            "type": "object",
            "properties": {},
        },
    },
]


# ---------------------------------------------------------------------------
# Tool execution dispatcher
# ---------------------------------------------------------------------------

def _execute_tool(name: str, params: dict[str, Any]) -> dict[str, Any]:
    """Execute a memory tool and return the result.

    Routes to the appropriate handler in the amplifier_module_engram_lite package.
    All errors are caught and returned as structured error responses.
    """
    try:
        store = _get_memory_store()

        if name == "memory_capture":
            from amplifier_module_engram_lite.tools.capture import handle_capture
            return handle_capture(store, params)

        if name == "memory_recall":
            from amplifier_module_engram_lite.tools.recall import handle_recall
            return handle_recall(store, params)

        if name == "memory_search":
            from amplifier_module_engram_lite.tools.recall import handle_search
            return handle_search(store, params)

        if name == "memory_update":
            from amplifier_module_engram_lite.tools.manage import handle_update
            return handle_update(store, params)

        if name == "memory_relate":
            from amplifier_module_engram_lite.tools.manage import handle_relate
            return handle_relate(store, params)

        if name == "memory_forget":
            from amplifier_module_engram_lite.tools.manage import handle_forget
            return handle_forget(store, params)

        if name == "memory_graph_explore":
            from amplifier_module_engram_lite.tools.recall import handle_graph_explore
            return handle_graph_explore(store, params)

        if name == "memory_stats":
            from amplifier_module_engram_lite.tools.manage import handle_stats
            return handle_stats(store, params)

        return {"error": f"Unknown tool: {name}"}

    except Exception as exc:
        logger.error("Tool execution error (%s): %s", name, exc)
        return {"error": str(exc), "tool": name}


# ---------------------------------------------------------------------------
# Amplifier mount() entry point
# ---------------------------------------------------------------------------

def mount(app: Any, *, config: dict[str, Any] | None = None) -> None:
    """Mount the engram-lite tool module into Amplifier.

    Registers all 8 memory tools with the Amplifier tool registry.

    Parameters
    ----------
    app:
        The Amplifier application instance. Provides `register_tool()`.
    config:
        Optional configuration from the behavior YAML.
    """
    for tool_def in TOOL_DEFINITIONS:
        tool_name = tool_def["name"]

        # Create a closure to capture tool_name correctly
        def make_handler(tn: str):
            def handler(params: dict[str, Any]) -> dict[str, Any]:
                return _execute_tool(tn, params)
            return handler

        app.register_tool(
            name=tool_name,
            description=tool_def["description"],
            parameters=tool_def["parameters"],
            handler=make_handler(tool_name),
        )

    logger.info("engram-lite tools mounted: %d tools registered", len(TOOL_DEFINITIONS))
```

### 6.3 Tool Registration Summary

| Tool | Required Params | Optional Params | Returns |
|---|---|---|---|
| `memory_capture` | `content` | `content_type`, `importance`, `domain`, `tags`, `keywords`, `confidence`, `space`, `source_context` | `{id, status}` |
| `memory_recall` | `query` | `limit`, `threshold`, `domains`, `content_types` | `{memories: [...], count}` |
| `memory_search` | — | `query`, `domain`, `content_type`, `tags`, `importance`, `min_confidence`, `space`, `created_after`, `created_before`, `limit` | `{memories: [...], count}` |
| `memory_update` | `memory_id` | `content`, `confidence`, `importance`, `add_tags`, `remove_tags` | `{id, status, updated_fields}` |
| `memory_relate` | `source_id`, `target_id`, `relation_type` | `weight`, `metadata` | `{relation_id, status}` |
| `memory_forget` | `memory_id` | `reason` | `{id, status}` |
| `memory_graph_explore` | `start` | `depth`, `relation_types`, `min_weight` | `{nodes: [...], edges: [...]}` |
| `memory_stats` | — | — | `{total, by_domain, by_type, by_importance, confidence_histogram, db_size_bytes}` |

---

## 7. Context Files

### 7.1 Behavioral Protocol Context

#### `amplifier/context/memory-protocol.md`

```markdown
# Memory Protocol

You have access to a persistent memory system. It retains knowledge across
sessions — facts, preferences, decisions, architecture, and debugging insights.

## Behavioral Rules

1. **Before responding** to questions that may relate to prior context, use
   `memory_recall(query)` to check for relevant stored knowledge.

2. **After responding**, evaluate whether new knowledge was exchanged. If yes,
   use `memory_capture(content)` to store it for future sessions.

3. **Never announce** memory operations. Do not say "I'll remember that" or
   "Let me check my memory." Use recalled knowledge naturally, as if you
   simply know it.

4. **Write conclusion-first.** When capturing, start with the main claim:
   "Project uses PostgreSQL 15 with pgvector. Deployed on AWS us-east-1."
   Not: "The user mentioned they use PostgreSQL..."

5. **Synthesize, don't quote.** Capture the knowledge, not the user's words.

6. **After capturing**, run a cross-reference cascade:
   - Find related memories
   - Detect patterns (same topic recurring → boost confidence)
   - Detect contradictions (conflicting claims → create relation, lower old confidence)
   - Update superseded memories (new replaces old → forget old)

7. **Privacy default:** Personal preferences → `user` space. Project specifics → `project` space.
   When unsure, apply the README test: would this be harmful in a public README?
   If yes → `user` space.

## Content Types

- `fact` — Concrete, verifiable information
- `preference` — User style, taste, or behavioral preference
- `decision` — A choice made between alternatives (include reasoning)
- `procedure` — A multi-step process or workflow
- `architecture` — System design, structure, or pattern
- `debug_insight` — Lessons from debugging or troubleshooting
```

### 7.2 Tool Usage Guide

#### `amplifier/context/tool-guide.md`

```markdown
# Memory Tools Reference

## Capture and Recall (most common)

### memory_recall(query, limit=5, threshold=0.3)
Retrieve relevant memories. Uses semantic search + graph traversal.
Call before responding to queries that may benefit from prior context.

### memory_capture(content, content_type="fact", importance="medium", ...)
Store new knowledge. Content must be conclusion-first.
Call after exchanges that produce durable knowledge.

## Search and Explore

### memory_search(query=None, domain=None, content_type=None, tags=None, ...)
Filtered search with explicit criteria. More control than recall.

### memory_graph_explore(start, depth=2, relation_types=None)
Traverse the knowledge graph from a memory or domain.

## Management

### memory_update(memory_id, confidence=None, importance=None, add_tags=None, ...)
Modify existing memory metadata. Use during cross-reference cascades.

### memory_relate(source_id, target_id, relation_type, weight=1.0)
Create graph edges. Types: relates_to, supports, contradicts, supersedes, depends_on, part_of.

### memory_forget(memory_id, reason=None)
Soft-delete a memory. Use when superseded or incorrect.

### memory_stats()
View system statistics: totals, domains, confidence distribution.
```

---

## 8. Installation

### 8.1 Install the Python Package

```bash
# Install engram-lite with Amplifier support and your embedding provider
pip install "engram-lite[amplifier,openai]"

# Or with all optional dependencies
pip install "engram-lite[all]"
```

### 8.2 Add to an Existing Amplifier Bundle

To add engram-lite to an existing bundle, add it to the bundle's behaviors list and deep-merge the behavior YAML.

#### Method 1: Reference the behavior in your root bundle

In your root `bundle.md` frontmatter:

```yaml
---
name: my-project-bundle
version: 1.0.0

behaviors:
  - engram-lite        # Add this line
  - my-other-behavior

config:
  engram-lite:
    embedding_provider: openai
    hot_context_limit: 15
---
```

#### Method 2: Deep-merge the behavior YAML into an existing configuration

If your bundle uses a different composition method, copy the behavior YAML:

```bash
# Copy the behavior YAML into your bundle's behaviors directory
cp $(python -c "import amplifier_module_engram_lite; print(amplifier_module_engram_lite.__path__[0])")/../../amplifier/behaviors/engram-lite.yaml \
   ./amplifier/behaviors/
```

Then reference it in your bundle's behaviors list.

### 8.3 Initialize the Database

```bash
# Initialize the database (creates ~/.engram-lite/memory.db)
engram-lite init

# Or specify a custom path
engram-lite init --db-path ./project-memory.db
```

### 8.4 Verify Installation

```bash
# Check that the Amplifier modules are discoverable
python -c "
from importlib.metadata import entry_points
eps = entry_points(group='amplifier.modules')
for ep in eps:
    if 'amplifier_module_engram_lite' in ep.name:
        print(f'  Found: {ep.name} -> {ep.value}')
"

# Test embedding provider connectivity
engram-lite check-embeddings
```

### 8.5 Set Environment Variables

```bash
# Required: embedding provider API key
export OPENAI_API_KEY=sk-...

# Optional: custom database path
export ENGRAM_DB_PATH=~/.engram-lite/memory.db

# Optional: custom embedding provider
export ENGRAM_EMBEDDING_PROVIDER=openai
export ENGRAM_EMBEDDING_MODEL=text-embedding-3-small
```

---

## 9. Configuration

### 9.1 Configuration Resolution Order

Configuration values are resolved in this priority order (highest first):

1. **Explicit behavior YAML config** (in `engram-lite.yaml` or root bundle frontmatter)
2. **Environment variables** (`ENGRAM_*`)
3. **Defaults** (defined in the configuration dataclass)

### 9.2 All Configuration Parameters

| Parameter | Env Var | Type | Default | Description |
|---|---|---|---|---|
| `db_path` | `ENGRAM_DB_PATH` | `str` | `~/.engram-lite/memory.db` | SQLite database file path |
| `db_journal_mode` | `ENGRAM_DB_JOURNAL_MODE` | `str` | `wal` | SQLite journal mode |
| `embedding_provider` | `ENGRAM_EMBEDDING_PROVIDER` | `str` | `openai` | Embedding provider: `openai`, `azure`, `ollama` |
| `embedding_model` | `ENGRAM_EMBEDDING_MODEL` | `str` | `text-embedding-3-small` | Model name for embeddings |
| `embedding_dimensions` | `ENGRAM_EMBEDDING_DIMENSIONS` | `int` | `1536` | Embedding vector dimensions |
| `openai_api_key` | `OPENAI_API_KEY` | `str` | — | OpenAI API key |
| `azure_endpoint` | `AZURE_OPENAI_ENDPOINT` | `str` | — | Azure OpenAI endpoint URL |
| `azure_api_key` | `AZURE_OPENAI_API_KEY` | `str` | — | Azure OpenAI API key |
| `azure_deployment` | `AZURE_OPENAI_EMBEDDING_DEPLOYMENT` | `str` | — | Azure deployment name |
| `azure_api_version` | `AZURE_OPENAI_API_VERSION` | `str` | `2024-10-21` | Azure API version |
| `ollama_base_url` | `ENGRAM_OLLAMA_URL` | `str` | `http://localhost:11434` | Ollama server URL |
| `ollama_model` | `ENGRAM_OLLAMA_MODEL` | `str` | `nomic-embed-text` | Ollama embedding model |
| `auto_recall` | `ENGRAM_AUTO_RECALL` | `bool` | `true` | Inject recall reminders per prompt |
| `auto_capture` | `ENGRAM_AUTO_CAPTURE` | `bool` | `true` | Inject capture reminders after response |
| `hot_context_limit` | `ENGRAM_HOT_CONTEXT_LIMIT` | `int` | `20` | Max memories loaded at session start |
| `token_budget` | `ENGRAM_TOKEN_BUDGET` | `int` | `2000` | Max tokens for hot context injection |
| `recall_limit` | `ENGRAM_RECALL_LIMIT` | `int` | `5` | Default results per recall |
| `recall_threshold` | `ENGRAM_RECALL_THRESHOLD` | `float` | `0.3` | Minimum relevance score |
| `system1_weight` | `ENGRAM_SYSTEM1_WEIGHT` | `float` | `0.6` | Weight for vector retrieval |
| `system2_weight` | `ENGRAM_SYSTEM2_WEIGHT` | `float` | `0.4` | Weight for graph retrieval |
| `default_space` | `ENGRAM_DEFAULT_SPACE` | `str` | `project` | Default memory space |
| `pii_detection` | `ENGRAM_PII_DETECTION` | `bool` | `true` | Enable PII detection on capture |
| `confidence_decay_enabled` | `ENGRAM_DECAY_ENABLED` | `bool` | `true` | Enable confidence decay |
| `decay_grace_period_days` | `ENGRAM_DECAY_GRACE_DAYS` | `int` | `90` | Days before decay begins |
| `decay_rate_per_30d` | `ENGRAM_DECAY_RATE` | `float` | `0.05` | Confidence loss per 30 days |
| `min_confidence` | `ENGRAM_MIN_CONFIDENCE` | `float` | `0.20` | Floor for confidence decay |
| `gc_threshold` | `ENGRAM_GC_THRESHOLD` | `float` | `0.15` | Confidence below which memories are GC candidates |
| `reminder_style` | `ENGRAM_REMINDER_STYLE` | `str` | `concise` | Reminder verbosity: `concise`, `verbose`, `minimal` |
| `log_level` | `ENGRAM_LOG_LEVEL` | `str` | `WARNING` | Log level for engram-lite |
| `log_file` | `ENGRAM_LOG_FILE` | `str` | `~/.engram-lite/engram-lite.log` | Log file path |

---

## 10. Examples

### 10.1 Minimal Bundle

The simplest possible bundle using engram-lite with all defaults:

```yaml
---
name: minimal-memory-bundle
version: 1.0.0
behaviors:
  - engram-lite
---

# My Project

An AI assistant with persistent memory.
```

Requires: `OPENAI_API_KEY` set in environment.

### 10.2 Full Bundle with All Options

A fully specified bundle with every configuration option:

```yaml
---
name: full-memory-bundle
version: 1.0.0
description: Fully configured engram-lite bundle

behaviors:
  - engram-lite

config:
  engram-lite:
    # Database
    db_path: /data/project-alpha/memory.db
    db_journal_mode: wal

    # Embeddings (using Azure OpenAI)
    embedding_provider: azure
    embedding_model: text-embedding-3-small
    embedding_dimensions: 1536
    azure_endpoint: https://my-resource.openai.azure.com
    azure_deployment: embeddings-v3
    azure_api_version: "2024-10-21"

    # Retrieval tuning
    auto_recall: true
    auto_capture: true
    hot_context_limit: 30
    token_budget: 3000
    recall_limit: 8
    recall_threshold: 0.25
    system1_weight: 0.5
    system2_weight: 0.5

    # Privacy
    default_space: project
    pii_detection: true

    # Maintenance
    confidence_decay_enabled: true
    decay_grace_period_days: 120
    decay_rate_per_30d: 0.03
    min_confidence: 0.15
    gc_threshold: 0.10

    # UX
    reminder_style: verbose

    # Logging
    log_level: INFO
    log_file: /data/project-alpha/engram-lite.log
---

# Project Alpha

Enterprise project with full memory configuration.

## Memory Notes

- Database stored alongside project data
- Azure OpenAI for embeddings (corporate requirement)
- Extended decay grace period (120 days)
- Verbose reminders for thorough memory coverage
- Equal weight between vector and graph retrieval
```

### 10.3 Custom Database Path per Project

Using environment variables to scope memory per project:

```yaml
---
name: per-project-memory
version: 1.0.0
behaviors:
  - engram-lite

config:
  engram-lite:
    db_path: ${PROJECT_ROOT}/.engram-lite/memory.db
    default_space: project
---

# Per-Project Memory

Each project gets its own memory database stored in the project directory.

Set `PROJECT_ROOT` to your project's root directory:
```
export PROJECT_ROOT=/path/to/my-project
```
```

### 10.4 Ollama (Local, Air-Gapped)

For environments without internet access:

```yaml
---
name: local-memory-bundle
version: 1.0.0
behaviors:
  - engram-lite

config:
  engram-lite:
    embedding_provider: ollama
    ollama_base_url: http://localhost:11434
    ollama_model: nomic-embed-text
    embedding_dimensions: 768

    # Adjusted recall threshold for different embedding model
    recall_threshold: 0.35

    # Privacy: no data leaves the machine
    pii_detection: false    # No PII leaves anyway
---

# Air-Gapped Memory

Uses local Ollama instance for embeddings. No data transmitted externally.

Requires:
- Ollama running locally: `ollama serve`
- Model pulled: `ollama pull nomic-embed-text`
```

### 10.5 Composing with Other Behaviors

Canvas-memory composes cleanly with other Amplifier behaviors:

```yaml
---
name: full-stack-assistant
version: 2.0.0

behaviors:
  - engram-lite
  - code-review
  - test-runner
  - deploy-guard

config:
  engram-lite:
    hot_context_limit: 15      # Leave context budget for other behaviors
    token_budget: 1500
    reminder_style: minimal    # Keep injections small

  code-review:
    auto_review: true

  test-runner:
    framework: pytest
---

# Full-Stack Assistant

AI assistant with memory, code review, test running, and deploy safety.
Memory is configured with a smaller context budget to leave room for other behaviors.
```
