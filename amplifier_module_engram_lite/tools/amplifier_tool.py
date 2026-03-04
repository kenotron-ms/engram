"""Amplifier tool module — exposes all 9 memory tools via Amplifier Tool protocol."""

from __future__ import annotations

from pathlib import Path
from typing import Any

__amplifier_module_type__ = "tool"


async def mount(coordinator: Any, config: dict[str, Any] | None = None) -> None:
    """Mount all engram-lite tools onto the Amplifier coordinator.

    Opens the user-scope database connection and registers all 9 memory tools
    into ``coordinator.mount_points["tools"]``.

    Args:
        coordinator: Amplifier coordinator with a ``mount_points`` dict.
        config: Optional configuration dict. Supported keys:

            - ``user_db`` (str, default ``~/.engram/engram.db``): User DB path.
    """
    config = config or {}
    user_db = Path(config.get("user_db", "~/.engram/engram.db")).expanduser()

    from amplifier_module_engram_lite.db.schema import get_db

    conn, _ = get_db(user_db)

    tools = [
        MemoryCaptureTool(conn),
        MemoryRecallTool(conn),
        MemorySearchTool(conn),
        MemoryUpdateTool(conn),
        MemoryForgetTool(conn),
        MemoryRelateTool(conn),
        MemoryGraphExploreTool(conn),
        MemoryStatsTool(conn),
        MemoryIndexTool(conn),
    ]
    for tool in tools:
        coordinator.mount_points["tools"][tool.name] = tool


def _tool_result(success: bool, output: Any) -> Any:
    """Build a ToolResult, with graceful fallback if amplifier_core not installed.

    Returns a ``ToolResult`` when ``amplifier_core`` is available, otherwise a
    plain dict mirroring the same structure.
    """
    try:
        from amplifier_core import ToolResult  # type: ignore[import-not-found]

        return ToolResult(success=success, output=output)
    except ImportError:
        return {"success": success, "output": output}


class _BaseTool:
    """Shared base for all tool wrappers — holds the DB connection."""

    def __init__(self, conn: Any) -> None:
        self._conn = conn


# ---------------------------------------------------------------------------
# Individual tool wrappers
# ---------------------------------------------------------------------------


class MemoryCaptureTool(_BaseTool):
    @property
    def name(self) -> str:
        return "memory_capture"

    @property
    def description(self) -> str:
        return (
            "Capture new knowledge into persistent memory. Use silently after learning"
            " something new — preference, decision, fact, or pattern."
        )

    @property
    def input_schema(self) -> dict[str, Any]:
        return {
            "type": "object",
            "properties": {
                "content": {"type": "string", "description": "Knowledge to capture"},
                "content_type": {
                    "type": "string",
                    "enum": [
                        "fact",
                        "preference",
                        "event",
                        "skill",
                        "entity",
                        "relationship",
                        "decision",
                        "constraint",
                    ],
                    "default": "fact",
                },
                "domain": {
                    "type": "string",
                    "description": "Domain path e.g. personal/prefs or professional/arch",
                },
                "space": {
                    "type": "string",
                    "enum": ["user", "project", "local"],
                    "default": "user",
                },
                "importance": {
                    "type": "string",
                    "enum": ["critical", "high", "medium", "low"],
                    "default": "medium",
                },
                "tags": {"type": "array", "items": {"type": "string"}},
            },
            "required": ["content"],
        }

    async def execute(self, input: dict[str, Any]) -> Any:
        from amplifier_module_engram_lite.tools.capture import memory_capture

        result = memory_capture(
            self._conn,
            input["content"],
            content_type=input.get("content_type", "fact"),
            space=input.get("space", "user"),
            domain=input.get("domain"),
            importance=input.get("importance", "medium"),
            tags=input.get("tags"),
        )
        return _tool_result(True, result)


class MemoryRecallTool(_BaseTool):
    @property
    def name(self) -> str:
        return "memory_recall"

    @property
    def description(self) -> str:
        return (
            "Recall relevant memories by semantic query. Use before responding to"
            " queries that may relate to prior context."
        )

    @property
    def input_schema(self) -> dict[str, Any]:
        return {
            "type": "object",
            "properties": {
                "query": {"type": "string"},
                "route": {
                    "type": "string",
                    "enum": ["auto", "vector", "graph", "hybrid", "keyword"],
                    "default": "auto",
                },
                "k": {"type": "integer", "default": 5},
                "domain": {"type": "string"},
                "space": {
                    "type": "string",
                    "enum": ["user", "project", "local"],
                },
            },
            "required": ["query"],
        }

    async def execute(self, input: dict[str, Any]) -> Any:
        from amplifier_module_engram_lite.tools.recall import memory_recall

        result = memory_recall(
            self._conn,
            input["query"],
            route=input.get("route", "auto"),
            k=input.get("k", 5),
            domain=input.get("domain"),
            space=input.get("space"),
        )
        return _tool_result(True, result)


class MemorySearchTool(_BaseTool):
    @property
    def name(self) -> str:
        return "memory_search"

    @property
    def description(self) -> str:
        return "Quick keyword search via BM25. Faster than memory_recall for exact term lookup."

    @property
    def input_schema(self) -> dict[str, Any]:
        return {
            "type": "object",
            "properties": {
                "query": {"type": "string"},
                "domain": {"type": "string"},
                "limit": {"type": "integer", "default": 10},
            },
            "required": ["query"],
        }

    async def execute(self, input: dict[str, Any]) -> Any:
        from amplifier_module_engram_lite.tools.recall import memory_search

        return _tool_result(
            True,
            memory_search(
                self._conn,
                input["query"],
                domain=input.get("domain"),
                limit=input.get("limit", 10),
            ),
        )


class MemoryUpdateTool(_BaseTool):
    @property
    def name(self) -> str:
        return "memory_update"

    @property
    def description(self) -> str:
        return "Update an existing memory's content, summary, tags, importance, or confidence."

    @property
    def input_schema(self) -> dict[str, Any]:
        return {
            "type": "object",
            "properties": {
                "memory_id": {"type": "string"},
                "content": {"type": "string"},
                "summary": {"type": "string"},
                "tags": {"type": "array", "items": {"type": "string"}},
                "importance": {
                    "type": "string",
                    "enum": ["critical", "high", "medium", "low"],
                },
                "confidence": {"type": "number", "minimum": 0, "maximum": 1},
            },
            "required": ["memory_id"],
        }

    async def execute(self, input: dict[str, Any]) -> Any:
        from amplifier_module_engram_lite.tools.manage import memory_update

        return _tool_result(
            True,
            memory_update(
                self._conn,
                input["memory_id"],
                content=input.get("content"),
                summary=input.get("summary"),
                tags=input.get("tags"),
                importance=input.get("importance"),
                confidence=input.get("confidence"),
            ),
        )


class MemoryForgetTool(_BaseTool):
    @property
    def name(self) -> str:
        return "memory_forget"

    @property
    def description(self) -> str:
        return "Permanently delete a memory. Use when information is wrong or no longer relevant."

    @property
    def input_schema(self) -> dict[str, Any]:
        return {
            "type": "object",
            "properties": {
                "memory_id": {"type": "string"},
                "reason": {"type": "string"},
            },
            "required": ["memory_id"],
        }

    async def execute(self, input: dict[str, Any]) -> Any:
        from amplifier_module_engram_lite.tools.manage import memory_forget

        return _tool_result(
            True,
            memory_forget(self._conn, input["memory_id"], reason=input.get("reason")),
        )


class MemoryRelateTool(_BaseTool):
    @property
    def name(self) -> str:
        return "memory_relate"

    @property
    def description(self) -> str:
        return "Create a typed edge between two memories in the knowledge graph."

    @property
    def input_schema(self) -> dict[str, Any]:
        return {
            "type": "object",
            "properties": {
                "from_id": {"type": "string"},
                "to_id": {"type": "string"},
                "relation_type": {
                    "type": "string",
                    "enum": [
                        "relates-to",
                        "supports",
                        "contradicts",
                        "supersedes",
                        "exemplifies",
                        "part-of",
                        "caused-by",
                        "decided-in",
                        "applies-to",
                    ],
                },
                "strength": {
                    "type": "number",
                    "minimum": 0,
                    "maximum": 1,
                    "default": 0.5,
                },
            },
            "required": ["from_id", "to_id", "relation_type"],
        }

    async def execute(self, input: dict[str, Any]) -> Any:
        from amplifier_module_engram_lite.tools.manage import memory_relate

        return _tool_result(
            True,
            memory_relate(
                self._conn,
                input["from_id"],
                input["to_id"],
                input["relation_type"],
                strength=input.get("strength", 0.5),
            ),
        )


class MemoryGraphExploreTool(_BaseTool):
    @property
    def name(self) -> str:
        return "memory_graph_explore"

    @property
    def description(self) -> str:
        return "Explore the hierarchical domain graph of memory topics."

    @property
    def input_schema(self) -> dict[str, Any]:
        return {
            "type": "object",
            "properties": {
                "query": {"type": "string"},
                "node_id": {"type": "string"},
                "depth": {"type": "integer", "default": 2, "minimum": 1, "maximum": 4},
            },
        }

    async def execute(self, input: dict[str, Any]) -> Any:
        from amplifier_module_engram_lite.tools.manage import memory_graph_explore

        return _tool_result(
            True,
            memory_graph_explore(
                self._conn,
                query=input.get("query"),
                node_id=input.get("node_id"),
                depth=input.get("depth", 2),
            ),
        )


class MemoryStatsTool(_BaseTool):
    @property
    def name(self) -> str:
        return "memory_stats"

    @property
    def description(self) -> str:
        return "Show statistics about the memory store (total, by type, by domain)."

    @property
    def input_schema(self) -> dict[str, Any]:
        return {
            "type": "object",
            "properties": {
                "space": {
                    "type": "string",
                    "enum": ["user", "project", "local"],
                },
            },
        }

    async def execute(self, input: dict[str, Any]) -> Any:
        from amplifier_module_engram_lite.tools.manage import memory_stats

        return _tool_result(True, memory_stats(self._conn, space=input.get("space")))


class MemoryIndexTool(_BaseTool):
    @property
    def name(self) -> str:
        return "memory_index"

    @property
    def description(self) -> str:
        return (
            "Read, write, check status of, or rebuild MEMORY.md hot-surface files. "
            "Use action='write' to update MEMORY.md with LLM-authored content."
        )

    @property
    def input_schema(self) -> dict[str, Any]:
        return {
            "type": "object",
            "properties": {
                "action": {
                    "type": "string",
                    "enum": ["read", "write", "status", "rebuild"],
                    "default": "read",
                },
                "scope": {
                    "type": "string",
                    "enum": ["user", "project", "local", "all"],
                    "default": "all",
                },
                "content": {
                    "type": "string",
                    "description": "Full Markdown content to write (required for action='write').",
                },
            },
        }

    async def execute(self, input: dict[str, Any]) -> Any:
        from amplifier_module_engram_lite.tools.manage import memory_index

        return _tool_result(
            True,
            memory_index(
                self._conn,
                action=input.get("action", "read"),
                scope=input.get("scope", "all"),
                content=input.get("content"),
            ),
        )

