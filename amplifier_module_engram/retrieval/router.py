"""Query router: analyse query → dispatch to System-1, System-2, or hybrid."""

from __future__ import annotations

import sqlite3

from amplifier_module_engram.db import memory_store as ms
from amplifier_module_engram.db import vector_store as vs
from amplifier_module_engram.retrieval.system1 import rrf, system1_recall
from amplifier_module_engram.retrieval.system2 import system2_recall
from amplifier_module_engram.retrieval.types import RetrievalResult

BROAD_KEYWORDS = {
    "everything",
    "all",
    "any",
    "overview",
    "summary",
    "know",
    "knowing",
    "tell",
    "about",
    "regarding",
}


def _detect_route(query: str) -> str:
    """Heuristically determine the best retrieval route."""
    words = query.strip().split()

    # All-caps word = exact acronym → keyword-only
    if any(w.isupper() and len(w) >= 2 for w in words):
        return "keyword"

    # Broad / comprehensive query → hybrid (system2 dominant)
    lower_words = {w.lower().rstrip("?.,") for w in words}
    if lower_words & BROAD_KEYWORDS:
        return "hybrid"

    # Short, specific → vector only (System-1)
    if len(words) <= 4:
        return "vector"

    # Default: hybrid
    return "hybrid"


def route_query(
    conn: sqlite3.Connection,
    query: str,
    *,
    route: str = "auto",
    k: int = 5,
    domain: str | None = None,
    space: str | None = None,
    min_confidence: float = 0.0,
) -> list[RetrievalResult]:
    """
    Route a query to the appropriate retrieval system and return top-k results.

    Routes:
      auto    — detect best route from query shape (default)
      vector  — System-1 KNN only (fast, precise)
      graph   — System-2 graph traversal only (broad, structural)
      hybrid  — both System-1 and System-2, fused (max recall)
      keyword — BM25 FTS5 only (exact term matching)
    """
    if route == "auto":
        route = _detect_route(query)

    # ── Keyword-only ──────────────────────────────────────────────────────────
    if route == "keyword":
        fts_query = " OR ".join(f'"{w}"' for w in query.split() if len(w) > 2) or query
        try:
            hits = ms.fts_search(conn, fts_query, limit=k)
        except Exception:
            hits = []
        results = []
        for h in hits:
            d = h["data"]
            results.append(
                RetrievalResult(
                    memory_id=h["id"],
                    summary=d.get("summary", ""),
                    domain=h["domain"],
                    tags=d.get("tags", []),
                    content_type=h["content_type"],
                    importance=h["importance"],
                    confidence=h["confidence"],
                    score=1.0,
                )
            )
        return results[:k]

    # ── Embed query (needed for System-1 and hybrid) ──────────────────────────
    query_vec: list[float] | None = None
    if route in ("vector", "hybrid"):
        query_vec = vs.embed(query)

    # ── Vector-only (System-1) ────────────────────────────────────────────────
    if route == "vector":
        assert query_vec is not None
        return system1_recall(
            conn,
            query,
            query_vec,
            k=k,
            domain=domain,
            space=space,
            min_confidence=min_confidence,
        )

    # ── Graph-only (System-2) ─────────────────────────────────────────────────
    if route == "graph":
        return system2_recall(
            conn,
            query,
            k=k,
            domain=domain,
            space=space,
            min_confidence=min_confidence,
        )

    # ── Hybrid: both systems, fused with RRF ─────────────────────────────────
    if query_vec is None:
        query_vec = vs.embed(query)

    s1 = system1_recall(
        conn,
        query,
        query_vec,
        k=k * 2,
        domain=domain,
        space=space,
        min_confidence=min_confidence,
    )
    s2 = system2_recall(
        conn,
        query,
        k=k * 2,
        domain=domain,
        space=space,
        min_confidence=min_confidence,
    )

    # Fuse rankings
    s1_ids = [r.memory_id for r in s1]
    s2_ids = [r.memory_id for r in s2]
    fused_scores = rrf([s1_ids, s2_ids])

    # Build final results; prefer richer data from s1 if available
    s1_map = {r.memory_id: r for r in s1}
    s2_map = {r.memory_id: r for r in s2}
    seen: set[str] = set()
    final: list[RetrievalResult] = []

    for mid, score in sorted(fused_scores.items(), key=lambda x: x[1], reverse=True):
        if mid in seen:
            continue
        seen.add(mid)
        base = s1_map.get(mid) or s2_map.get(mid)
        if base:
            final.append(
                RetrievalResult(
                    memory_id=mid,
                    summary=base.summary,
                    domain=base.domain,
                    tags=base.tags,
                    content_type=base.content_type,
                    importance=base.importance,
                    confidence=base.confidence,
                    score=round(score, 4),
                )
            )
        if len(final) >= k:
            break

    return final
