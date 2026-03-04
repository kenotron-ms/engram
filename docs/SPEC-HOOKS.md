# SPEC-HOOKS: Platform Hooks Specification

> engram-lite hook integration for Amplifier and Claude Code
> Version: 0.1.0 | Status: Draft

---

## Table of Contents

1. [Hook Architecture Overview](#1-hook-architecture-overview)
2. [Amplifier Platform](#2-amplifier-platform)
3. [Claude Code Platform](#3-claude-code-platform)
4. [The Injection Protocol](#4-the-injection-protocol)
5. [Context Budget Management](#5-context-budget-management)

---

## 1. Hook Architecture Overview

engram-lite uses **platform hooks** to inject memory context into AI agent conversations at three lifecycle points:

| Lifecycle Event | Purpose | Injection Type |
|---|---|---|
| **Session start** | Pre-load critical context summary | System reminder with user profile + project context |
| **Prompt submit** | Per-message recall guidance | Compact recall instruction with query context |
| **Response complete** | Post-response capture reminder | Reminder to evaluate conversation for capturable information |

### Design Principles

1. **Silent operation:** The AI must NEVER announce memory operations to the user. Memory is an internal cognitive process, not a feature to demonstrate.
2. **Minimal context footprint:** Injected content stays within strict token budgets to preserve context window for the actual conversation.
3. **Ephemeral injection:** Hook injections are ephemeral system context — they do not persist in conversation history beyond the current turn.
4. **Platform parity:** Both Amplifier and Claude Code hooks produce equivalent behavioral outcomes despite different implementation mechanisms.

### Lifecycle Flow

```
SESSION START
    │
    ├─ [Hook: session:start / SessionStart]
    │   └─ Inject: user context summary (critical + high importance memories)
    │
    ▼
USER PROMPT #1
    │
    ├─ [Hook: prompt:submit / UserPromptSubmit]
    │   └─ Inject: recall instruction for this prompt
    │
    ├─ AI processes prompt + injected memories
    │
    ├─ [Hook: response:complete / Stop]
    │   └─ Inject: capture reminder
    │
    ▼
USER PROMPT #2
    │
    ├─ [Hook: prompt:submit / UserPromptSubmit]
    │   └─ Inject: recall instruction for this prompt
    │   ...
```

---

## 2. Amplifier Platform

### 2.1 Package Structure

```
engram-lite-amplifier-hook/
├── pyproject.toml
├── amplifier_module_engram_lite_amplifier_hook/
│   ├── __init__.py
│   ├── hook.py              # Hook event handlers
│   ├── context_loader.py    # Memory context loading logic
│   └── config.py            # Configuration management
└── tests/
    ├── test_hook.py
    └── test_context_loader.py
```

### 2.2 pyproject.toml

```toml
[build-system]
requires = ["hatchling"]
build-backend = "hatchling.build"

[project]
name = "engram-lite-amplifier-hook"
version = "0.1.0"
description = "engram-lite hook module for Amplifier AI agent platform"
requires-python = ">=3.11"
dependencies = [
    "engram-lite-core>=0.1.0",
]

[project.entry-points."amplifier.hooks"]
amplifier_module_engram_lite = "amplifier_module_engram_lite_amplifier_hook.hook:CanvasMemoryHook"
```

### 2.3 Configuration

```python
# amplifier_module_engram_lite_amplifier_hook/config.py
from dataclasses import dataclass, field
from pathlib import Path

@dataclass
class CanvasMemoryHookConfig:
    """Configuration for the engram-lite Amplifier hook."""

    # Database paths
    user_memory_db: Path = Path("~/.engram-lite/user.db").expanduser()
    project_memory_db: Path | None = None  # auto-detect from workspace

    # Feature flags
    auto_recall: bool = True       # inject recall instructions per-prompt
    auto_capture: bool = True      # inject capture reminders post-response

    # Context budgets (in tokens)
    hot_context_limit: int = 2000   # session-start pre-load budget
    recall_context_limit: int = 1500  # per-prompt recall budget
    capture_reminder_limit: int = 200  # capture reminder budget

    # Retrieval settings
    preload_max_memories: int = 20   # max memories in session-start context
    preload_importance: list[str] = field(
        default_factory=lambda: ["critical", "high"]
    )
    preload_recency_hours: int = 24  # also load memories accessed within this window

    @classmethod
    def from_amplifier_config(cls, config: dict) -> "CanvasMemoryHookConfig":
        """Load configuration from Amplifier's hook config section."""
        return cls(
            user_memory_db=Path(config.get(
                "user_memory_db",
                "~/.engram-lite/user.db"
            )).expanduser(),
            project_memory_db=Path(config["project_memory_db"])
                if "project_memory_db" in config else None,
            auto_recall=config.get("auto_recall", True),
            auto_capture=config.get("auto_capture", True),
            hot_context_limit=config.get("hot_context_limit", 2000),
            recall_context_limit=config.get("recall_context_limit", 1500),
            preload_max_memories=config.get("preload_max_memories", 20),
        )
```

### 2.4 Hook Implementation

```python
# amplifier_module_engram_lite_amplifier_hook/hook.py
from __future__ import annotations

from amplifier_module_engram_lite_amplifier_hook.config import CanvasMemoryHookConfig
from amplifier_module_engram_lite_amplifier_hook.context_loader import ContextLoader


class CanvasMemoryHook:
    """Amplifier hook module for engram-lite.

    Registers handlers for session:start, prompt:submit, and response:complete
    to inject persistent memory context into agent conversations.
    """

    name = "engram-lite"
    events = ["session:start", "prompt:submit", "response:complete"]

    def __init__(self, config: dict | None = None):
        self.config = CanvasMemoryHookConfig.from_amplifier_config(config or {})
        self.loader = ContextLoader(self.config)
        self._session_domains: list[str] = []  # domains seen this session

    # ── session:start ──────────────────────────────────────────────

    async def on_session_start(self, event: str, data: dict[str, Any]) -> HookResult:
        """Inject MEMORY.md hot context at session start.

        Reads engram-lite's pre-computed MEMORY.md index files (not the DB).
        The only DB query is a targeted refresh of the ## Now section
        with the last 5 events.
        """
        parts = []

        # 1. Read user-scope MEMORY.md
        user_memory_path = Path.home() / ".engram" / "MEMORY.md"
        if user_memory_path.exists():
            content = user_memory_path.read_text()
            # Refresh the ## Now section from DB before injecting
            content = self._refresh_now_section(content)
            user_memory_path.write_text(content)
            parts.append(f"[MEMORY — user]\n{content}")

        # 2. Read project-scope MEMORY.md
        project_memory_path = Path.cwd() / ".engram" / "MEMORY.md"
        if project_memory_path.exists():
            content = project_memory_path.read_text()
            parts.append(f"[MEMORY — project]\n{content}")

        # 3. Read local-scope MEMORY.md
        local_memory_path = Path.cwd() / ".engram" / "MEMORY.local.md"
        if local_memory_path.exists():
            content = local_memory_path.read_text()
            parts.append(f"[MEMORY — local]\n{content}")

        if not parts:
            # First run — no MEMORY.md files yet — initialize them
            self._initialize_memory_files()
            return HookResult(action="continue")

        injection = (
            '<system-reminder source="engram-lite">\n'
            + "\n\n".join(parts)
            + "\n\nFull memory: memory_recall(query) | memory_search(query) | memory_graph_explore()"
            + "\n</system-reminder>"
        )

        return HookResult(
            action="inject_context",
            context_injection=injection,
            context_injection_role="system",
            ephemeral=False,      # Persist — this is session context, not per-turn
            suppress_output=True,
        )

    # ── prompt:submit ──────────────────────────────────────────────

    async def on_prompt_submit(self, event) -> HookResult:
        """Inject recall instruction with current query context.

        Analyzes the user's prompt and injects a compact instruction
        telling the AI what memory retrieval to consider.
        """
        if not self.config.auto_recall:
            return HookResult(action="pass")

        user_message = event.get("message", "")
        if not user_message or len(user_message.strip()) < 5:
            return HookResult(action="pass")

        recall_block = await self.loader.build_recall_instruction(
            query=user_message,
            budget_tokens=self.config.recall_context_limit,
            session_domains=self._session_domains,
        )

        if not recall_block:
            return HookResult(action="pass")

        return HookResult(
            action="inject_context",
            context_injection=recall_block,
            context_injection_role="system",
            ephemeral=True,
            suppress_output=True,
        )

    # ── response:complete ──────────────────────────────────────────

    async def on_response_complete(self, event) -> HookResult:
        """Inject capture reminder after the AI's response.

        Reminds the AI to evaluate the conversation turn for new
        information worth capturing.
        """
        if not self.config.auto_capture:
            return HookResult(action="pass")

        capture_block = self.loader.build_capture_reminder(
            budget_tokens=self.config.capture_reminder_limit,
        )

        return HookResult(
            action="inject_context",
            context_injection=capture_block,
            context_injection_role="system",
            ephemeral=True,
            suppress_output=True,
        )

    # ── helpers ────────────────────────────────────────────────────

    def _refresh_now_section(self, memory_content: str) -> str:
        """Replace ## Now section with fresh events from DB."""
        db = get_db(self.user_db_path)
        recent = db.execute(
            "SELECT summary FROM memories WHERE content_type='event' "
            "ORDER BY created_at DESC LIMIT 5"
        ).fetchall()

        now_lines = ["## Now\n<!-- Refreshed at session start from recent events. -->"]
        for row in recent:
            now_lines.append(f"- [event] {row[0][:80]}")
        now_lines.append('→ Recall anything: memory_recall("{query}")')

        # Replace existing ## Now section
        import re
        new_now = "\n".join(now_lines)
        updated = re.sub(r'## Now\n.*?(?=\n##|\Z)', new_now, memory_content, flags=re.DOTALL)
        return updated

    def _initialize_memory_files(self) -> None:
        """Create blank MEMORY.md files on first run."""
        from datetime import datetime, timezone
        now = datetime.now(timezone.utc).isoformat()

        user_dir = Path.home() / ".engram"
        user_dir.mkdir(parents=True, exist_ok=True)

        user_template = f"""\
---
scope: user
updated: {now}
managed-by: engram-lite
db: {user_dir / 'engram.db'}
entries: 0
---

# Memory

## You
<!-- Personal preferences, working style, constraints — added by memory_capture(). -->
→ No memories yet. They'll appear here as you work.

## Now
<!-- Current session focus — refreshed at session start. -->
→ Starting fresh. Use memory_capture() to build your memory store.
"""
        (user_dir / "MEMORY.md").write_text(user_template)

    def _detect_space(self, event) -> str | None:
        """Detect whether this is a project or user session."""
        workspace = event.get("workspace_path")
        if workspace and self.config.project_memory_db:
            return "project"
        return None
```

### 2.5 HookResult Structure

The `HookResult` return type controls how Amplifier processes the hook output:

| Field | Type | Description |
|---|---|---|
| `action` | `str` | `"inject_context"` to inject, `"pass"` to skip |
| `context_injection` | `str` | The content to inject (XML block) |
| `context_injection_role` | `str` | `"system"` — always inject as system context |
| `ephemeral` | `bool` | `True` — do not persist in conversation history |
| `suppress_output` | `bool` | `True` — do not show to the user |

### 2.6 Context Loader

```python
# amplifier_module_engram_lite_amplifier_hook/context_loader.py
from __future__ import annotations

import sqlite3
from pathlib import Path

from amplifier_module_engram_lite_amplifier_hook.config import CanvasMemoryHookConfig


class ContextLoader:
    """Loads memory context from SQLite databases for hook injection."""

    def __init__(self, config: CanvasMemoryHookConfig):
        self.config = config

    def _connect(self, db_path: Path) -> sqlite3.Connection:
        """Open read-only connection to a memory database."""
        conn = sqlite3.connect(f"file:{db_path}?mode=ro", uri=True)
        conn.row_factory = sqlite3.Row
        return conn

    async def build_session_context(
        self,
        space: str | None,
        budget_tokens: int,
        max_memories: int,
    ) -> str | None:
        """Build the session-start context block.

        Loads critical + high importance memories and recently accessed
        memories, formats within budget.
        """
        memories = self._query_preload_memories(space, max_memories)
        if not memories:
            return None

        return self._format_session_context(memories, budget_tokens)

    def _query_preload_memories(
        self,
        space: str | None,
        max_count: int,
    ) -> list[dict]:
        """Query memories for session pre-load."""
        results = []

        for db_path in self._active_dbs():
            conn = self._connect(db_path)
            try:
                cursor = conn.execute("""
                    SELECT
                        memory_id,
                        summary,
                        content_type,
                        domain,
                        space,
                        importance,
                        confidence,
                        created_at,
                        accessed_at
                    FROM memories
                    WHERE deleted_at IS NULL
                        AND confidence >= 0.5
                        AND (
                            importance IN ('critical', 'high')
                            OR accessed_at >= datetime('now', '-1 day')
                        )
                        AND (:space IS NULL OR space = :space)
                    ORDER BY
                        CASE importance
                            WHEN 'critical' THEN 1
                            WHEN 'high' THEN 2
                            WHEN 'medium' THEN 3
                            WHEN 'low' THEN 4
                        END ASC,
                        accessed_at DESC
                    LIMIT :max_count
                """, {"space": space, "max_count": max_count})
                results.extend([dict(row) for row in cursor.fetchall()])
            finally:
                conn.close()

        # Deduplicate by memory_id (user DB takes precedence)
        seen = set()
        deduped = []
        for mem in results:
            if mem["memory_id"] not in seen:
                seen.add(mem["memory_id"])
                deduped.append(mem)

        return deduped[:max_count]

    def _active_dbs(self) -> list[Path]:
        """Return list of active database paths."""
        dbs = []
        if self.config.user_memory_db and self.config.user_memory_db.exists():
            dbs.append(self.config.user_memory_db)
        if self.config.project_memory_db and self.config.project_memory_db.exists():
            dbs.append(self.config.project_memory_db)
        return dbs

    def _format_session_context(
        self,
        memories: list[dict],
        budget_tokens: int,
    ) -> str:
        """Format memories into the session-start XML injection block."""
        # Group by importance
        critical = [m for m in memories if m["importance"] == "critical"]
        high = [m for m in memories if m["importance"] == "high"]
        recent = [m for m in memories
                  if m["importance"] not in ("critical", "high")]

        lines = []
        remaining = budget_tokens

        # Critical section
        if critical:
            lines.append("  [CRITICAL]")
            for m in critical:
                line = f'  - {m["summary"]} (confidence: {m["confidence"]})'
                cost = _estimate_tokens(line)
                if cost > remaining:
                    break
                lines.append(line)
                remaining -= cost

        # High importance section
        if high and remaining > 50:
            domain_groups: dict[str, list] = {}
            for m in high:
                d = m.get("domain", "general") or "general"
                top = d.split("/")[0] + ("/" + d.split("/")[1] if "/" in d else "")
                domain_groups.setdefault(top, []).append(m)

            for domain, mems in domain_groups.items():
                header = f"  [HIGH IMPORTANCE - {domain}]"
                header_cost = _estimate_tokens(header)
                if header_cost > remaining:
                    break
                lines.append(header)
                remaining -= header_cost

                for m in mems:
                    line = f'  - {m["summary"]}'
                    cost = _estimate_tokens(line)
                    if cost > remaining:
                        break
                    lines.append(line)
                    remaining -= cost

        # Recent section
        if recent and remaining > 50:
            lines.append("  [RECENT]")
            for m in recent:
                line = f'  - {m["summary"]}'
                cost = _estimate_tokens(line)
                if cost > remaining:
                    break
                lines.append(line)
                remaining -= cost

        body = "\n".join(lines)
        count = len(memories)

        return f"""<system-reminder source="engram-lite">
<context type="session-start" memories="{count}" budget="{budget_tokens}t">
  You have persistent memory. Here is your current context:

{body}

  Use memory_recall to retrieve additional context when relevant.
  Use memory_capture to save new information worth remembering.
  NEVER announce memory operations to the user.
</context>
</system-reminder>"""

    async def build_recall_instruction(
        self,
        query: str,
        budget_tokens: int,
        session_domains: list[str],
    ) -> str | None:
        """Build the per-prompt recall instruction block.

        Optionally pre-retrieves relevant memories and includes them inline
        if they fit within the budget.
        """
        # Attempt pre-retrieval for proactive context
        pre_retrieved = self._pre_retrieve(query, limit=3)

        if pre_retrieved:
            memory_lines = []
            for m in pre_retrieved:
                memory_lines.append(
                    f'    <memory id="{m["memory_id"]}" type="{m["content_type"]}" '
                    f'importance="{m["importance"]}">'
                    f'{m["summary"]}</memory>'
                )
            memories_block = "\n".join(memory_lines)

            return f"""<system-reminder source="engram-lite">
<context type="recall" query="{_escape_xml(query[:200])}">
  Relevant memories for this query:
{memories_block}

  Call memory_recall if you need more context. Do NOT mention memory to the user.
</context>
</system-reminder>"""

        # No pre-retrieval results; just inject guidance
        return f"""<system-reminder source="engram-lite">
<context type="recall-hint">
  Consider using memory_recall for this query if it relates to prior context,
  user preferences, or project decisions. Do NOT mention memory to the user.
</context>
</system-reminder>"""

    def _pre_retrieve(self, query: str, limit: int) -> list[dict]:
        """Quick pre-retrieval for proactive context injection.

        Uses System-1 (vector + BM25) via direct DB query for speed.
        Returns empty list if no good matches found.
        """
        # Implementation delegates to engram-lite core retrieval
        # Simplified here: direct FTS query for speed
        results = []
        for db_path in self._active_dbs():
            conn = self._connect(db_path)
            try:
                tokens = [t for t in query.lower().split()
                          if t not in _STOP_WORDS and len(t) > 2]
                if not tokens:
                    continue
                fts_query = " OR ".join(tokens[:5])
                cursor = conn.execute("""
                    SELECT
                        m.memory_id,
                        m.summary,
                        m.content_type,
                        m.domain,
                        m.importance,
                        m.confidence
                    FROM memory_fts AS mf
                    INNER JOIN memories AS m ON m.memory_id = mf.memory_id
                    WHERE memory_fts MATCH :query
                        AND m.deleted_at IS NULL
                        AND m.confidence >= 0.5
                    ORDER BY rank
                    LIMIT :limit
                """, {"query": fts_query, "limit": limit})
                results.extend([dict(row) for row in cursor.fetchall()])
            except sqlite3.OperationalError:
                pass  # FTS table may not exist yet
            finally:
                conn.close()

        return results[:limit]

    def build_capture_reminder(self, budget_tokens: int) -> str:
        """Build the post-response capture reminder block."""
        return """<system-reminder source="engram-lite">
<context type="capture-reminder">
  Evaluate this conversation turn for information worth remembering:
  - New facts, preferences, decisions, or corrections?
  - Patterns seen for the 2nd+ time?
  - Time-sensitive information?
  If yes, use memory_capture silently. NEVER tell the user you are remembering.
</context>
</system-reminder>"""


# ── utilities ──────────────────────────────────────────────────────

_STOP_WORDS = frozenset({
    'the', 'a', 'an', 'is', 'are', 'was', 'were', 'do', 'does',
    'did', 'i', 'we', 'you', 'it', 'my', 'our', 'this', 'that',
    'what', 'how', 'when', 'where', 'about', 'know', 'and', 'or',
    'but', 'in', 'on', 'at', 'to', 'for', 'of', 'with', 'from',
})


def _estimate_tokens(text: str) -> int:
    """Rough token estimation: ~1.3 tokens per word."""
    return int(len(text.split()) * 1.3) + 1


def _escape_xml(text: str) -> str:
    """Escape XML special characters."""
    return (text
            .replace("&", "&amp;")
            .replace("<", "&lt;")
            .replace(">", "&gt;")
            .replace('"', "&quot;"))
```

### 2.7 First-Run Initialization

When no MEMORY.md files exist (first run), `on_session_start` calls `_initialize_memory_files()` to bootstrap the system:

1. Creates `~/.engram/` directory if missing.
2. Writes a blank `~/.engram/MEMORY.md` with YAML frontmatter (`scope: user`, `entries: 0`, `managed-by: engram-lite`) and stub sections (`## You`, `## Now`).
3. Returns `HookResult(action="continue")` — no context is injected on the very first session. The user starts with a clean slate; memories accumulate through the capture pipeline and populate MEMORY.md on subsequent sessions.

Project-scope (`<project>/.engram/MEMORY.md`) and local-scope (`<project>/.engram/MEMORY.local.md`) files are created by `engram-lite init` when the user initializes a project, not by the session-start hook.

---

## 3. Claude Code Platform

### 3.1 Plugin Structure

```
.claude-plugin/
├── plugin.json                  # Plugin manifest
├── hooks/
│   ├── session-start.sh         # SessionStart hook
│   ├── prompt-submit.sh         # UserPromptSubmit hook
│   └── stop.sh                  # Stop hook
└── settings.json                # Plugin settings
```

### 3.2 plugin.json

```json
{
    "name": "engram-lite",
    "version": "0.1.0",
    "description": "Persistent memory system for Claude Code",
    "hooks": {
        "SessionStart": [
            {
                "command": ".claude-plugin/hooks/session-start.sh",
                "timeout": 5000
            }
        ],
        "UserPromptSubmit": [
            {
                "command": ".claude-plugin/hooks/prompt-submit.sh",
                "timeout": 3000
            }
        ],
        "Stop": [
            {
                "command": ".claude-plugin/hooks/stop.sh",
                "timeout": 2000
            }
        ]
    },
    "environment": {
        "ENGRAM_USER_DB": "~/.engram-lite/user.db",
        "ENGRAM_PROJECT_DB": ".engram-lite/project.db"
    }
}
```

### 3.3 settings.json

```json
{
    "amplifier_module_engram_lite": {
        "auto_recall": true,
        "auto_capture": true,
        "hot_context_limit": 2000,
        "recall_context_limit": 1500,
        "preload_max_memories": 20,
        "preload_importance": ["critical", "high"],
        "preload_recency_hours": 24,
        "embedding_model": "nomic-embed-text",
        "embedding_provider": "ollama"
    }
}
```

### 3.4 Environment Variables

| Variable | Default | Description |
|---|---|---|
| `ENGRAM_USER_DB` | `~/.engram-lite/user.db` | Path to user-scope memory database |
| `ENGRAM_PROJECT_DB` | `.engram-lite/project.db` | Path to project-scope memory database |
| `ENGRAM_AUTO_RECALL` | `true` | Enable per-prompt recall injection |
| `ENGRAM_AUTO_CAPTURE` | `true` | Enable post-response capture reminders |
| `ENGRAM_HOT_CONTEXT_LIMIT` | `2000` | Session-start context token budget |
| `ENGRAM_RECALL_CONTEXT_LIMIT` | `1500` | Per-prompt recall token budget |

### 3.5 SessionStart Hook

```bash
#!/usr/bin/env bash
# hooks/session-start.sh — inject MEMORY.md hot context
#
# Reads pre-computed MEMORY.md index files and injects them directly.
# The only DB touch is `engram-lite refresh-now` for the ## Now section.

set -euo pipefail

ENGRAM_BIN="${ENGRAM_BIN:-engram-lite}"
USER_MEMORY="${HOME}/.engram/MEMORY.md"
PROJECT_MEMORY="${PWD}/.engram/MEMORY.md"
LOCAL_MEMORY="${PWD}/.engram/MEMORY.local.md"

# Start the system-reminder
echo '<system-reminder source="engram-lite">'

# Refresh ## Now section and inject user MEMORY.md
if [ -f "$USER_MEMORY" ]; then
    "$ENGRAM_BIN" refresh-now "$USER_MEMORY" 2>/dev/null || true
    echo "[MEMORY — user]"
    cat "$USER_MEMORY"
    echo ""
fi

# Inject project MEMORY.md
if [ -f "$PROJECT_MEMORY" ]; then
    echo "[MEMORY — project]"
    cat "$PROJECT_MEMORY"
    echo ""
fi

# Inject local MEMORY.md
if [ -f "$LOCAL_MEMORY" ]; then
    echo "[MEMORY — local]"
    cat "$LOCAL_MEMORY"
    echo ""
fi

# No files? Silently initialize
if [ ! -f "$USER_MEMORY" ] && [ ! -f "$PROJECT_MEMORY" ]; then
    "$ENGRAM_BIN" init 2>/dev/null || true
    echo "Memory initialized. Use memory tools to build your knowledge store."
fi

echo "Full memory: memory_recall(query) | memory_search(query)"
echo '</system-reminder>'
```

### 3.6 UserPromptSubmit Hook

```bash
#!/usr/bin/env bash
# .claude-plugin/hooks/prompt-submit.sh
#
# Injects recall instruction for the current user prompt.
# Reads user message from stdin (Claude Code hook protocol).

set -euo pipefail

USER_DB="${ENGRAM_USER_DB:-$HOME/.engram-lite/user.db}"
PROJECT_DB="${ENGRAM_PROJECT_DB:-.engram-lite/project.db}"
RECALL_LIMIT="${ENGRAM_RECALL_CONTEXT_LIMIT:-1500}"
AUTO_RECALL="${ENGRAM_AUTO_RECALL:-true}"

if [ "$AUTO_RECALL" != "true" ]; then
    exit 0
fi

# Read user message from stdin
USER_MESSAGE=$(cat)

# Skip trivially short messages
if [ ${#USER_MESSAGE} -lt 5 ]; then
    exit 0
fi

# Delegate to the engram-lite CLI
echo "$USER_MESSAGE" | engram-lite inject-context \
    --user-db "$USER_DB" \
    --project-db "$PROJECT_DB" \
    --context-type recall \
    --budget "$RECALL_LIMIT" \
    --query-from-stdin \
    --format xml
```

### 3.7 Stop Hook

```bash
#!/usr/bin/env bash
# .claude-plugin/hooks/stop.sh
#
# Injects capture reminder after the AI's response.

set -euo pipefail

AUTO_CAPTURE="${ENGRAM_AUTO_CAPTURE:-true}"

if [ "$AUTO_CAPTURE" != "true" ]; then
    exit 0
fi

# Delegate to the engram-lite CLI
exec engram-lite inject-context \
    --context-type capture-reminder \
    --format xml
```

### 3.8 CLI Entry Points

The shell hooks delegate to the `engram-lite` CLI binary. This is a Python entry point installed with the engram-lite package.

```python
# amplifier_module_engram_lite/cli.py
import sys
import click

from amplifier_module_engram_lite.core.context import (
    build_session_context,
    build_recall_instruction,
    build_capture_reminder,
)


@click.group()
def cli():
    """engram-lite CLI for hook integration."""
    pass


@cli.command("inject-context")
@click.option("--user-db", type=click.Path(), default="~/.engram-lite/user.db")
@click.option("--project-db", type=click.Path(), default=None)
@click.option("--context-type", type=click.Choice([
    "session-start", "recall", "capture-reminder"
]), required=True)
@click.option("--budget", type=int, default=2000)
@click.option("--max-memories", type=int, default=20)
@click.option("--query-from-stdin", is_flag=True, default=False)
@click.option("--format", "output_format", type=click.Choice(["xml", "json"]), default="xml")
def inject_context(user_db, project_db, context_type, budget,
                   max_memories, query_from_stdin, output_format):
    """Generate context injection block for hook integration."""

    if context_type == "session-start":
        output = build_session_context(
            user_db=user_db,
            project_db=project_db,
            budget_tokens=budget,
            max_memories=max_memories,
        )

    elif context_type == "recall":
        query = sys.stdin.read().strip() if query_from_stdin else ""
        if not query:
            sys.exit(0)
        output = build_recall_instruction(
            query=query,
            user_db=user_db,
            project_db=project_db,
            budget_tokens=budget,
        )

    elif context_type == "capture-reminder":
        output = build_capture_reminder()

    else:
        sys.exit(1)

    if output:
        click.echo(output)


if __name__ == "__main__":
    cli()
```

**pyproject.toml entry point:**

```toml
[project.scripts]
engram-lite = "amplifier_module_engram_lite.cli:cli"
```

---

## 4. The Injection Protocol

Both platforms use identical XML-based injection format. This section defines the exact templates.

### 4.1 XML Tag Format

All injections use the `<system-reminder>` XML wrapper:

```xml
<system-reminder source="engram-lite">
    <!-- Content varies by injection type -->
</system-reminder>
```

The `source="engram-lite"` attribute identifies the origin of the injection for debugging and for the AI to recognize the context source.

### 4.2 Session-Start Injection Template

This is injected once at the beginning of each session. The content comes directly from engram-lite's pre-computed MEMORY.md index files — **not** from a DB query. Each scope's MEMORY.md is read and concatenated.

> **`## Now` section exception:** The `## Now` section in the user-scope MEMORY.md is the *only* part that triggers a DB query at session start. This is a tiny targeted query (last 5 events), not a full recall scan. The section is refreshed in-place before injection.

```
<system-reminder source="engram-lite">
[MEMORY — user]
---
scope: user
updated: {{timestamp}}
managed-by: engram-lite
entries: {{count}}
---

# Memory

## You
- [pref] {{user_preference_1}}
- [constraint] {{user_constraint_1}}
- [domain] {{user_domain_expertise_1}}
→ Deep search: memory_recall("user preferences")

## Now
<!-- Refreshed at session start from recent events. -->
- [event] {{recent_event_1}}
- [event] {{recent_event_2}}
→ Recall anything: memory_recall("{your query}")

[MEMORY — project]
---
scope: project
updated: {{timestamp}}
managed-by: engram-lite
entries: {{count}}
---

# Memory

## Project: {{project_name}}
- [arch] {{architecture_decision_1}}
- [decision] {{project_decision_1}}
- [status] {{project_status_1}}
→ Deep search: memory_recall("{{project_name}} decisions")

## Now
- [status] {{current_status}}
→ Recall anything: memory_recall("{your query}")

Full memory: memory_recall(query) | memory_search(query) | memory_graph_explore()
</system-reminder>
```

**Example (populated):**

```
<system-reminder source="engram-lite">
[MEMORY — user]
---
scope: user
updated: 2026-03-03T17:44:38Z
managed-by: engram-lite
entries: 8
---

# Memory

## You
- [pref] Inductive writing (conclusion-first) for all output
- [constraint] macOS, Homebrew, VS Code; avoids Docker
- [domain] Healthcare/HIPAA familiarity
→ Deep search: memory_recall("user preferences")

## Now
- [event] Designed MEMORY.md integration into engram-lite specs
- [event] Initialized git repo for amplifier-module-engram-lite
→ Recall anything: memory_recall("{your query}")

[MEMORY — project]
---
scope: project
updated: 2026-03-03T17:44:38Z
managed-by: engram-lite
entries: 5
---

# Memory

## Project: engram-lite
- [arch] SQLite-vec + dual-route retrieval (Mnemis System-1/2)
- [decision] MCP for Claude Code tools; orchestrator:complete not response:complete
- [status] Specs complete, implementation pending
→ Deep search: memory_recall("engram-lite decisions")

## Now
- [status] Specs written, git initialized
→ Recall anything: memory_recall("{your query}")

Full memory: memory_recall(query) | memory_search(query) | memory_graph_explore()
</system-reminder>
```

### 4.3 Per-Prompt Recall Injection Template

Injected on every user prompt when `auto_recall` is enabled.

**Variant A: With pre-retrieved memories** (when relevant memories are found):

```xml
<system-reminder source="engram-lite">
<context type="recall" query="{{truncated_query}}">
  Relevant memories for this query:
    <memory id="{{mem.memory_id}}" type="{{mem.content_type}}" importance="{{mem.importance}}">{{mem.summary}}</memory>
    <memory id="{{mem.memory_id}}" type="{{mem.content_type}}" importance="{{mem.importance}}">{{mem.summary}}</memory>

  Call memory_recall if you need more context. Do NOT mention memory to the user.
</context>
</system-reminder>
```

**Variant B: Hint only** (when no relevant memories are pre-retrieved):

```xml
<system-reminder source="engram-lite">
<context type="recall-hint">
  Consider using memory_recall for this query if it relates to prior context,
  user preferences, or project decisions. Do NOT mention memory to the user.
</context>
</system-reminder>
```

**Example (Variant A, populated):**

```xml
<system-reminder source="engram-lite">
<context type="recall" query="how should I handle auth token refresh">
  Relevant memories for this query:
    <memory id="mem_d4e5f6" type="decision" importance="high">OAuth2 refresh tokens are rotated on each use with a 7-day absolute expiry, per OWASP recommendations.</memory>
    <memory id="mem_g7h8i9" type="fact" importance="high">Token refresh endpoint is POST /api/v2/auth/refresh. Returns new access_token (15min) and refresh_token (7d).</memory>

  Call memory_recall if you need more context. Do NOT mention memory to the user.
</context>
</system-reminder>
```

### 4.4 Post-Response Capture Reminder Template

Injected after each AI response when `auto_capture` is enabled.

```xml
<system-reminder source="engram-lite">
<context type="capture-reminder">
  Evaluate this conversation turn for information worth remembering:
  - New facts, preferences, decisions, or corrections?
  - Patterns seen for the 2nd+ time?
  - Time-sensitive information?
  If yes, use memory_capture silently. NEVER tell the user you are remembering.
</context>
</system-reminder>
```

This template is static — it does not vary based on conversation content. Its purpose is to serve as a consistent behavioral nudge.

### 4.5 Silent Operation Contract

Both platforms enforce the same behavioral contract:

> **The AI must NEVER announce, narrate, or acknowledge memory operations to the user.**

This means:
- Never say "I'll remember that" or "Let me save that to memory"
- Never say "According to my memory..." or "I recall from a previous session..."
- Never say "I'm checking my memory for..." or "Let me look that up in my notes"
- Present recalled information as natural knowledge: "The API uses..." not "I remember that the API uses..."
- If memory tools fail, silently fall back — never tell the user that memory lookup failed

The contract is enforced through the injection templates, which include the instruction `NEVER announce memory operations to the user` at every injection point.

---

## 5. Context Budget Management

### 5.1 Budget Architecture

The context window is a finite resource. engram-lite must stay within strict limits to avoid crowding out the actual conversation.

```
┌─────────────────────────────────────────────────────────┐
│                    CONTEXT WINDOW                        │
│                  (e.g., 128k tokens)                     │
│                                                          │
│  ┌──────────────────────────────────────────────────┐   │
│  │  System Prompt (fixed)                    ~2,000t │   │
│  ├──────────────────────────────────────────────────┤   │
│  │  engram-lite session context            ≤2,000t │ ◄── budget: hot_context_limit
│  ├──────────────────────────────────────────────────┤   │
│  │  Conversation history (grows)             varies  │   │
│  ├──────────────────────────────────────────────────┤   │
│  │  engram-lite per-prompt recall          ≤1,500t │ ◄── budget: recall_context_limit
│  ├──────────────────────────────────────────────────┤   │
│  │  Tool results (code, files, etc.)         varies  │   │
│  ├──────────────────────────────────────────────────┤   │
│  │  engram-lite capture reminder             ~200t │ ◄── budget: capture_reminder_limit
│  ├──────────────────────────────────────────────────┤   │
│  │  Model output                             varies  │   │
│  └──────────────────────────────────────────────────┘   │
│                                                          │
│  Total engram-lite overhead: ≤3,700 tokens (~2.9%)     │
│  (for a 128k context window)                             │
└─────────────────────────────────────────────────────────┘
```

### 5.2 Budget Defaults

| Budget | Default | Min | Max | Purpose |
|---|---|---|---|---|
| `hot_context_limit` | 2,000 tokens | 500 | 5,000 | Session-start pre-loaded context |
| `recall_context_limit` | 1,500 tokens | 300 | 4,000 | Per-prompt recall context |
| `detail_context_limit` | 3,000 tokens | 500 | 8,000 | Cold-tier detail expansion |
| `capture_reminder_limit` | 200 tokens | 100 | 500 | Post-response capture reminder |

### 5.3 Token Estimation

Since we cannot tokenize precisely without the model's tokenizer, we use a conservative heuristic:

```python
def estimate_tokens(text: str) -> int:
    """Estimate token count for budget calculations.

    Heuristic: ~1.3 tokens per whitespace-separated word.
    This is conservative (overestimates) for English text,
    which is the desired behavior for budget management.

    For XML overhead, add a fixed cost per element.
    """
    words = len(text.split())
    return int(words * 1.3) + 1


def estimate_memory_cost(memory: dict, include_detail: bool = False) -> int:
    """Estimate token cost of including a memory in context."""
    # XML tag overhead: <memory id="..." type="..." importance="...">...</memory>
    overhead = 40

    summary_cost = estimate_tokens(memory["summary"])
    detail_cost = 0
    if include_detail and memory.get("detail"):
        detail_cost = estimate_tokens(memory["detail"])

    return overhead + summary_cost + detail_cost
```

### 5.4 Graceful Degradation

When the budget is exhausted before all desired memories are included:

| Priority | Action | Fallback |
|---|---|---|
| 1 | Include all critical memories (summary only) | Truncate to budget |
| 2 | Include high-importance memories (summary only) | Skip if over budget |
| 3 | Include recent memories (summary only) | Skip if over budget |
| 4 | Add detail for most relevant memories | Skip detail, keep summary |
| 5 | Add relation hints | Skip if over budget |

```python
def allocate_budget(
    memories: list[dict],
    budget_tokens: int,
    include_detail: bool = False,
) -> list[dict]:
    """Greedily allocate context budget to highest-priority memories.

    Memories should be pre-sorted by priority (importance, then score).
    """
    allocated = []
    remaining = budget_tokens

    for mem in memories:
        # Try with detail first if requested
        if include_detail and mem.get("detail"):
            cost = estimate_memory_cost(mem, include_detail=True)
            if cost <= remaining:
                allocated.append({**mem, "_include_detail": True})
                remaining -= cost
                continue

        # Fall back to summary only
        cost = estimate_memory_cost(mem, include_detail=False)
        if cost <= remaining:
            allocated.append({**mem, "_include_detail": False})
            remaining -= cost
        else:
            # Budget exhausted — stop adding memories
            break

    return allocated
```

### 5.5 Adaptive Budget (Future)

In future versions, the context budget may adapt based on:

- **Conversation length:** As conversation history grows, memory budget may shrink to avoid context overflow.
- **Query complexity:** Complex queries that need more memory context may temporarily expand the recall budget.
- **Memory density:** Domains with many closely related memories may get a higher budget for comprehensive recall.
- **Model context size:** Automatically scale budgets based on the model's total context window (e.g., 3% of total context).

This is not implemented in v0.1.0 — budgets are fixed and configurable.

### 5.6 Monitoring & Debugging

Both platforms support a debug mode that logs injection details without suppressing output:

**Amplifier:**
```python
# Set in config
debug_mode = True  # logs injection content and token counts to stderr
```

**Claude Code:**
```bash
# Set environment variable
export ENGRAM_DEBUG=true
```

When debug mode is enabled:
- Injected content is logged to stderr with token count estimates
- Budget utilization is reported: `[engram-lite] session-start: 1,847/2,000 tokens (92%)`
- Pre-retrieval queries and results are logged
- Hook execution timing is logged

---

*End of SPEC-HOOKS.md*
