"""System-2 retrieval: hierarchical graph traversal (Mnemis global selection)."""

from __future__ import annotations

import math
import sqlite3
from datetime import UTC, datetime

from amplifier_module_engram.db import memory_store as ms
from amplifier_module_engram.retrieval.types import RetrievalResult

IMPORTANCE_WEIGHTS = {"critical": 1.0, "high": 0.8, "medium": 0.5, "low": 0.2}
RECENCY_HALF_LIFE_DAYS = 90


def _recency_score(accessed_at: str | None) -> float:
    if not accessed_at:
        return 0.5
    try:
        dt = datetime.fromisoformat(accessed_at.replace("Z", "+00:00"))
        days = max(0, (datetime.now(UTC) - dt).days)
        return math.exp(-days * math.log(2) / RECENCY_HALF_LIFE_DAYS)
    except Exception:
        return 0.5


def _find_matching_nodes(conn: sqlite3.Connection, query: str, domain: str | None) -> list[str]:
    """Find graph node IDs whose label overlaps with query keywords."""
    import re

    words = [w.lower() for w in re.findall(r"[a-zA-Z]{4,}", query)]
    if not words:
        # Fallback: return all root-level nodes
        rows = conn.execute("SELECT id FROM graph_nodes WHERE parent IS NULL LIMIT 5").fetchall()
        return [r[0] for r in rows]

    matched_ids: set[str] = set()
    for word in words:
        rows = conn.execute(
            "SELECT id FROM graph_nodes WHERE label LIKE ?",
            (f"%{word}%",),
        ).fetchall()
        for r in rows:
            matched_ids.add(r[0])

    # If domain filter, also restrict
    if domain and matched_ids:
        filtered = conn.execute(
            f"SELECT id FROM graph_nodes WHERE label LIKE ? "
            f"AND id IN ({','.join('?' * len(matched_ids))})",
            (domain + "%", *matched_ids),
        ).fetchall()
        if filtered:
            matched_ids = {r[0] for r in filtered}

    return list(matched_ids)


def _collect_subtree_memories(conn: sqlite3.Connection, node_ids: list[str]) -> list[str]:
    """Use recursive CTE to collect all memory_ids under the given nodes."""
    if not node_ids:
        return []
    placeholders = ",".join("?" * len(node_ids))
    rows = conn.execute(
        f"""
        WITH RECURSIVE subtree(id) AS (
            SELECT id FROM graph_nodes WHERE id IN ({placeholders})
            UNION ALL
            SELECT g.id FROM graph_nodes g JOIN subtree s ON g.parent = s.id
        )
        SELECT DISTINCT mgn.memory_id
        FROM memory_graph_nodes mgn
        JOIN subtree st ON st.id = mgn.node_id
        """,
        node_ids,
    ).fetchall()
    return [r[0] for r in rows]


def system2_recall(
    conn: sqlite3.Connection,
    query: str,
    *,
    k: int = 10,
    domain: str | None = None,
    space: str | None = None,
    min_confidence: float = 0.0,
) -> list[RetrievalResult]:
    """
    Deliberate retrieval via hierarchical graph traversal.
    Best for broad queries: 'what do I know about X', 'everything about Y'.
    Returns up to k results scored by confidence × importance × recency.
    """
    # Find graph nodes matching query keywords
    node_ids = _find_matching_nodes(conn, query, domain)

    if not node_ids:
        return []

    # Collect all memories in matched subtrees
    memory_ids = _collect_subtree_memories(conn, node_ids)

    if not memory_ids:
        return []

    # Fetch, filter, and score
    results: list[RetrievalResult] = []
    for memory_id in memory_ids:
        mem = ms.get_memory(conn, memory_id, track_access=False)
        if not mem:
            continue
        if mem["confidence"] < min_confidence:
            continue
        if space and mem["space"] != space:
            continue

        d = mem["data"]
        imp_w = IMPORTANCE_WEIGHTS.get(mem["importance"], 0.5)
        rec = _recency_score(d.get("accessed_at"))
        score = mem["confidence"] * imp_w * rec

        results.append(
            RetrievalResult(
                memory_id=memory_id,
                summary=d.get("summary", ""),
                domain=mem["domain"],
                tags=d.get("tags", []),
                content_type=mem["content_type"],
                importance=mem["importance"],
                confidence=mem["confidence"],
                score=round(score, 4),
            )
        )

    results.sort(key=lambda r: r.score, reverse=True)
    return results[:k]
