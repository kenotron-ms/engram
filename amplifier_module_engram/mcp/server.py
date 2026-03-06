"""MCP server for engram-lite — exposes all 9 memory tools via Model Context Protocol."""

from __future__ import annotations

import os
from pathlib import Path
from typing import Annotated

from mcp.server.fastmcp import FastMCP

mcp = FastMCP(
    "engram-lite",
    instructions=(
        "Persistent vector memory that gives you continuity across conversations. "
        "Follow the RETRIEVE → RESPOND → CAPTURE protocol on every turn: "
        "(1) Call memory_recall before responding to anything involving prior context — "
        "preferences, past decisions, project history, people, recurring topics. "
        "(2) Respond naturally using what you retrieved. "
        "(3) After the turn, call memory_capture to retain anything new worth keeping. "
        "All memory operations are silent — never announce or explain them to the user."
    ),
)

# ── DB connection (lazy, per-process singleton) ───────────────────────────────

_conn = None


def _get_conn():
    global _conn
    if _conn is None:
        db_path = Path(os.environ.get("ENGRAM_USER_DB", "~/.engram/engram.db")).expanduser()
        from amplifier_module_engram.db.schema import get_db

        _conn, _ = get_db(db_path)
    return _conn


# ── Tools ─────────────────────────────────────────────────────────────────────


@mcp.tool()
def memory_capture(
    content: Annotated[
        str,
        "Fact, preference, decision, or observation to store. Write conclusion-first.",
    ],
    content_type: Annotated[
        str,
        "Type: fact | preference | event | skill | entity | relationship | decision | constraint",
    ] = "fact",
    domain: Annotated[
        str | None,
        "Slash-separated topic path, e.g. personal/prefs or professional/arch or projects/myapp",
    ] = None,
    space: Annotated[
        str, "Storage scope — user (cross-project), project (this repo only), local (this session)"
    ] = "user",
    importance: Annotated[
        str, "How critical this memory is: critical | high | medium | low"
    ] = "medium",
    tags: Annotated[list[str] | None, "Optional tags for filtering and grouping memories"] = None,
) -> dict:
    """Save new knowledge to persistent memory.

    Call silently after learning something worth retaining — a preference, decision, fact,
    constraint, or pattern. Never announce this call to the user.
    """
    from amplifier_module_engram.tools.capture import memory_capture as _capture

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
    query: Annotated[str, "Describe what you want to remember — semantic, not just keywords"],
    route: Annotated[
        str, "Search strategy: auto (recommended) | vector | graph | hybrid | keyword"
    ] = "auto",
    k: Annotated[int, "Maximum number of memories to return"] = 5,
    domain: Annotated[str | None, "Restrict results to this domain subtree"] = None,
    space: Annotated[str | None, "Restrict to space: user | project | local"] = None,
    include_detail: Annotated[bool, "Include full content in results (not just summary)"] = False,
) -> list[dict]:
    """Search persistent memory by semantic similarity.

    Call before responding to anything involving prior context — preferences, past decisions,
    project history, people, or recurring topics. Returns ranked memories with relevance scores.
    Use memory_search instead when you need exact keyword or ID lookup.
    """
    from amplifier_module_engram.tools.recall import memory_recall as _recall

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
    query: Annotated[str, "Exact keywords or phrases to find via full-text index"],
    domain: Annotated[str | None, "Restrict results to this domain subtree"] = None,
    limit: Annotated[int, "Maximum number of results to return"] = 10,
) -> list[dict]:
    """Fast keyword search across memory using BM25 full-text index.

    Prefer this over memory_recall when you have an exact term, name, ID, or specific phrase.
    Faster but purely lexical — use memory_recall for semantic or concept-based queries.
    """
    from amplifier_module_engram.tools.recall import memory_search as _search

    return _search(_get_conn(), query, domain=domain, limit=limit)


@mcp.tool()
def memory_update(
    memory_id: Annotated[str, "ID of the memory to update (from recall/search results)"],
    content: Annotated[
        str | None, "Updated content — triggers re-embedding and semantic index refresh"
    ] = None,
    summary: Annotated[str | None, "Updated one-line summary shown in search results"] = None,
    tags: Annotated[list[str] | None, "Replace all tags (pass empty list to clear)"] = None,
    importance: Annotated[str | None, "Updated importance: critical | high | medium | low"] = None,
    confidence: Annotated[float | None, "Updated confidence score 0.0–1.0"] = None,
) -> dict:
    """Correct or refine an existing memory.

    Use when information was right but needs updating — a preference has changed, a decision
    was revised, or details need clarifying. Updating content triggers re-embedding so
    semantic search stays accurate. Use memory_forget for memories that are simply wrong.
    """
    from amplifier_module_engram.tools.manage import memory_update as _update

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
    memory_id: Annotated[str, "ID of the memory to permanently remove"],
    reason: Annotated[str | None, "Why it's being removed, e.g. 'outdated' or 'incorrect'"] = None,
) -> dict:
    """Permanently delete a memory. Use when information is wrong or no longer relevant.

    For memories that are merely outdated or need corrections, prefer memory_update.
    Deletion is irreversible — provide a reason to keep the action auditable.
    """
    from amplifier_module_engram.tools.manage import memory_forget as _forget

    return _forget(_get_conn(), memory_id, reason=reason)


@mcp.tool()
def memory_relate(
    from_id: Annotated[str, "Source memory ID (the 'from' end of the relationship)"],
    to_id: Annotated[str, "Target memory ID (the 'to' end of the relationship)"],
    relation_type: Annotated[
        str,
        "Relationship type: relates-to | supports | contradicts | supersedes | exemplifies"
        " | part-of | caused-by | decided-in | applies-to",
    ],
    strength: Annotated[float, "Relationship strength from 0.0 (weak) to 1.0 (strong)"] = 0.5,
) -> dict:
    """Link two memories with a typed relationship in the knowledge graph.

    Builds structured connections between related facts — e.g. link a decision to the
    context that caused it, a constraint to the project it applies to, or a fact that
    supersedes an older one. Enables graph-based navigation via memory_graph_explore.
    """
    from amplifier_module_engram.tools.manage import memory_relate as _relate

    return _relate(_get_conn(), from_id, to_id, relation_type, strength=strength)


@mcp.tool()
def memory_graph_explore(
    query: Annotated[str | None, "Keywords to find matching domain nodes (optional)"] = None,
    node_id: Annotated[str | None, "Start traversal from this specific node (optional)"] = None,
    depth: Annotated[int, "How many hops to traverse outward (1–4)"] = 2,
) -> dict:
    """Traverse the hierarchical domain graph to discover memory structure.

    Use to see what domains exist, navigate outward from a known node, or find
    clusters of related memories. Complements memory_recall for structured exploration
    when you want to understand the shape of what's stored rather than retrieve content.
    """
    from amplifier_module_engram.tools.manage import memory_graph_explore as _explore

    return _explore(_get_conn(), query=query, node_id=node_id, depth=depth)


@mcp.tool()
def memory_stats(
    space: Annotated[str | None, "Scope to inspect: user | project | local (omit for all)"] = None,
) -> dict:
    """Show statistics about the memory store.

    Returns total memory count, breakdown by type and domain, and per-space usage.
    Useful for understanding what has been captured and where memory is concentrated.
    """
    from amplifier_module_engram.tools.manage import memory_stats as _stats

    return _stats(_get_conn(), space=space)


@mcp.tool()
def memory_index(
    action: Annotated[
        str, "What to do: read (get content) | write (update content) | status (check paths)"
    ] = "status",
    scope: Annotated[str, "Which MEMORY.md files: user | project | local | all"] = "all",
    content: Annotated[
        str | None, "Full Markdown content to write (required when action='write')"
    ] = None,
) -> dict:
    """Manage the MEMORY.md hot-surface file — always-visible session context.

    MEMORY.md is a short prose narrative injected at every session start so key context
    is immediately available without a search. Read it to inspect the current narrative,
    write it to update with fresh LLM-authored prose, or check status to see file paths.
    Use action='write' with a complete rewrite — do not append or patch.
    """
    from amplifier_module_engram.tools.manage import memory_index as _index

    return _index(_get_conn(), action=action, scope=scope, content=content)


# ── Entry point ───────────────────────────────────────────────────────────────


def main() -> None:
    mcp.run(transport="stdio")


if __name__ == "__main__":
    main()
