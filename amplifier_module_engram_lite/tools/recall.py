"""
Recall tool — retrieve relevant memories via dual-route search.

Part of engram-lite (amplifier-module-engram-lite).
See docs/ for full specifications.
"""

from __future__ import annotations

import sqlite3

from amplifier_module_engram_lite.db import memory_store as ms


def memory_recall(
    conn: sqlite3.Connection,
    query: str,
    *,
    route: str = "auto",
    k: int = 5,
    domain: str | None = None,
    space: str | None = None,
    min_confidence: float = 0.0,
    include_detail: bool = False,
) -> list[dict]:
    """
    Recall memories by semantic query using the dual-route retrieval engine.

    Routes:
      auto    — detect best route from query shape (default)
      vector  — KNN similarity only (fast, precise)
      graph   — hierarchical graph traversal (broad, comprehensive)
      hybrid  — both routes fused (max recall)
      keyword — BM25 full-text search only (exact terms)

    Returns list of dicts with keys:
      memory_id, summary, domain, tags, content_type,
      importance, confidence, score, [content, detail if include_detail]
    """
    from amplifier_module_engram_lite.retrieval.router import route_query

    results = route_query(
        conn,
        query,
        route=route,
        k=k,
        domain=domain,
        space=space,
        min_confidence=min_confidence,
    )

    out = []
    for r in results:
        entry: dict = {
            "memory_id": r.memory_id,
            "summary": r.summary,
            "domain": r.domain,
            "tags": r.tags,
            "content_type": r.content_type,
            "importance": r.importance,
            "confidence": r.confidence,
            "score": r.score,
        }
        if include_detail:
            mem = ms.get_memory(conn, r.memory_id, track_access=True)
            if mem:
                d = mem["data"]
                entry["content"] = d.get("content", "")
                entry["detail"] = d.get("detail")
        out.append(entry)

    return out


def memory_search(
    conn: sqlite3.Connection,
    query: str,
    *,
    domain: str | None = None,
    limit: int = 10,
    filters: dict | None = None,
) -> list[dict]:
    """
    Quick keyword search using BM25 (FTS5). Simpler and faster than memory_recall.
    Best for: exact term lookup, known keywords, tag-based filtering.

    Returns list of dicts with keys: memory_id, summary, domain, tags, content_type,
    importance, confidence.
    """
    # Build FTS5 query safely
    terms = [w for w in query.split() if len(w) > 1]
    fts_query = " OR ".join(f'"{t}"' for t in terms) if terms else query

    try:
        hits = ms.fts_search(conn, fts_query, limit=limit)
    except Exception:
        # FTS5 query syntax error — fall back to raw query
        try:
            hits = ms.fts_search(conn, query, limit=limit)
        except Exception:
            return []

    # Apply post-filters
    if domain:
        hits = [h for h in hits if h["domain"].startswith(domain)]

    filters = filters or {}
    if "content_type" in filters:
        hits = [h for h in hits if h["content_type"] == filters["content_type"]]
    if "space" in filters:
        hits = [h for h in hits if h["space"] == filters["space"]]
    if "importance" in filters:
        hits = [h for h in hits if h["importance"] == filters["importance"]]

    return [
        {
            "memory_id": h["id"],
            "summary": h["data"].get("summary", ""),
            "domain": h["domain"],
            "tags": h["data"].get("tags", []),
            "content_type": h["content_type"],
            "importance": h["importance"],
            "confidence": h["confidence"],
        }
        for h in hits
    ]
