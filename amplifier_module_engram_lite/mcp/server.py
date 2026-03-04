"""MCP server for engram-lite — exposes all 9 memory tools via Model Context Protocol."""

from __future__ import annotations

import os
from pathlib import Path
from typing import Annotated

from mcp.server.fastmcp import FastMCP

mcp = FastMCP(
    "engram-lite",
    instructions=(
        "Persistent vector memory for AI agents. "
        "Use memory_capture after learning new information. "
        "Use memory_recall before responding to context-dependent queries. "
        "All operations are silent — never announce them to the user."
    ),
)

# ── DB connection (lazy, per-process singleton) ───────────────────────────────

_conn = None


def _get_conn():
    global _conn
    if _conn is None:
        db_path = Path(os.environ.get("ENGRAM_USER_DB", "~/.engram/engram.db")).expanduser()
        from amplifier_module_engram_lite.db.schema import get_db

        _conn, _ = get_db(db_path)
    return _conn


# ── Tools ─────────────────────────────────────────────────────────────────────


@mcp.tool()
def memory_capture(
    content: Annotated[str, "Knowledge to capture"],
    content_type: Annotated[
        str, "Type: fact|preference|event|skill|entity|relationship|decision|constraint"
    ] = "fact",
    domain: Annotated[str | None, "Domain path e.g. personal/prefs or professional/arch"] = None,
    space: Annotated[str, "Storage scope: user|project|local"] = "user",
    importance: Annotated[str, "Importance: critical|high|medium|low"] = "medium",
    tags: Annotated[list[str] | None, "Categorical tags"] = None,
) -> dict:
    """Capture new knowledge into persistent memory. Use silently after learning something new."""
    from amplifier_module_engram_lite.tools.capture import memory_capture as _capture

    return _capture(
        _get_conn(),
        content,
        content_type=content_type,
        domain=domain,
        space=space,
        importance=importance,
        tags=tags or [],
    )


@mcp.tool()
def memory_recall(
    query: Annotated[str, "What to recall"],
    route: Annotated[str, "Route: auto|vector|graph|hybrid|keyword"] = "auto",
    k: Annotated[int, "Max results"] = 5,
    domain: Annotated[str | None, "Restrict to domain subtree"] = None,
    space: Annotated[str | None, "Restrict to space: user|project|local"] = None,
    include_detail: Annotated[bool, "Include full content field"] = False,
) -> list[dict]:
    """Recall relevant memories by semantic query.

    Use before responding to context-dependent queries.
    """
    from amplifier_module_engram_lite.tools.recall import memory_recall as _recall

    return _recall(
        _get_conn(),
        query,
        route=route,
        k=k,
        domain=domain,
        space=space,
        include_detail=include_detail,
    )


@mcp.tool()
def memory_search(
    query: Annotated[str, "Keywords to search for"],
    domain: Annotated[str | None, "Restrict to domain subtree"] = None,
    limit: Annotated[int, "Max results"] = 10,
) -> list[dict]:
    """Quick BM25 keyword search. Faster than memory_recall for exact term lookup."""
    from amplifier_module_engram_lite.tools.recall import memory_search as _search

    return _search(_get_conn(), query, domain=domain, limit=limit)


@mcp.tool()
def memory_update(
    memory_id: Annotated[str, "ID of memory to update"],
    content: Annotated[str | None, "New content (triggers re-embedding)"] = None,
    summary: Annotated[str | None, "New summary"] = None,
    tags: Annotated[list[str] | None, "Replace tags (empty list = clear all)"] = None,
    importance: Annotated[str | None, "New importance level"] = None,
    confidence: Annotated[float | None, "New confidence 0.0-1.0"] = None,
) -> dict:
    """Update an existing memory's fields."""
    from amplifier_module_engram_lite.tools.manage import memory_update as _update

    return _update(
        _get_conn(),
        memory_id,
        content=content,
        summary=summary,
        tags=tags,
        importance=importance,
        confidence=confidence,
    )


@mcp.tool()
def memory_forget(
    memory_id: Annotated[str, "ID of memory to delete"],
    reason: Annotated[str | None, "Why this memory is being removed"] = None,
) -> dict:
    """Permanently delete a memory from storage."""
    from amplifier_module_engram_lite.tools.manage import memory_forget as _forget

    return _forget(_get_conn(), memory_id, reason=reason)


@mcp.tool()
def memory_relate(
    from_id: Annotated[str, "Source memory ID"],
    to_id: Annotated[str, "Target memory ID"],
    relation_type: Annotated[
        str,
        "Edge type: relates-to|supports|contradicts|supersedes|exemplifies"
        "|part-of|caused-by|decided-in|applies-to",
    ],
    strength: Annotated[float, "Edge strength 0.0-1.0"] = 0.5,
) -> dict:
    """Create a typed relationship edge between two memories."""
    from amplifier_module_engram_lite.tools.manage import memory_relate as _relate

    return _relate(_get_conn(), from_id, to_id, relation_type, strength=strength)


@mcp.tool()
def memory_graph_explore(
    query: Annotated[str | None, "Find nodes matching keywords"] = None,
    node_id: Annotated[str | None, "Start from specific node ID"] = None,
    depth: Annotated[int, "Traversal depth 1-4"] = 2,
) -> dict:
    """Explore the hierarchical domain graph of memory topics."""
    from amplifier_module_engram_lite.tools.manage import memory_graph_explore as _explore

    return _explore(_get_conn(), query=query, node_id=node_id, depth=depth)


@mcp.tool()
def memory_stats(
    space: Annotated[str | None, "Filter by space: user|project|local"] = None,
) -> dict:
    """Return statistics about the memory store."""
    from amplifier_module_engram_lite.tools.manage import memory_stats as _stats

    return _stats(_get_conn(), space=space)


@mcp.tool()
def memory_index(
    action: Annotated[str, "Action: read|write|status"] = "status",
    scope: Annotated[str, "Scope: user|project|local|all"] = "all",
    content: Annotated[
        str | None, "Full Markdown content to write (required for action='write')"
    ] = None,
) -> dict:
    """Read, write, check status of, or rebuild MEMORY.md hot-surface files.

    Use action='write' to update MEMORY.md with LLM-authored prose narrative content.
    """
    from amplifier_module_engram_lite.tools.manage import memory_index as _index

    return _index(_get_conn(), action=action, scope=scope, content=content)


# ── Entry point ───────────────────────────────────────────────────────────────


def main() -> None:
    mcp.run(transport="stdio")


if __name__ == "__main__":
    main()
