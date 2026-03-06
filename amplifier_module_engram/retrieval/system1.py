"""System-1 retrieval: vector KNN + BM25 full-text search, fused with RRF."""

from __future__ import annotations

import sqlite3

from amplifier_module_engram.db import memory_store as ms
from amplifier_module_engram.db import vector_store as vs
from amplifier_module_engram.retrieval.types import RetrievalResult

IMPORTANCE_WEIGHTS = {"critical": 1.0, "high": 0.8, "medium": 0.5, "low": 0.2}


def rrf(rankings: list[list[str]], k: int = 60) -> dict[str, float]:
    """Reciprocal Rank Fusion. Returns {memory_id: score}, higher = more relevant."""
    scores: dict[str, float] = {}
    for ranking in rankings:
        for rank, memory_id in enumerate(ranking):
            scores[memory_id] = scores.get(memory_id, 0.0) + 1.0 / (k + rank + 1)
    return scores


def system1_recall(
    conn: sqlite3.Connection,
    query: str,
    query_vec: list[float],
    *,
    k: int = 10,
    domain: str | None = None,
    space: str | None = None,
    min_confidence: float = 0.0,
) -> list[RetrievalResult]:
    """
    Fast retrieval: KNN vector search + BM25 keyword search, fused with RRF.
    Returns up to k results sorted by fused score descending.
    """
    fetch_k = max(k * 3, 20)  # fetch more candidates than needed before re-ranking

    # ── Leg 1: vector KNN ────────────────────────────────────────────────────
    knn_hits = vs.knn_search(conn, query_vec, k=fetch_k, domain=domain, space=space)
    knn_ranking = [mid for mid, _ in knn_hits]

    # ── Leg 2: BM25 via FTS5 ─────────────────────────────────────────────────
    # Build FTS5 query: quote each token to avoid syntax errors
    fts_query = " OR ".join(f'"{w}"' for w in query.split() if len(w) > 2) or query
    try:
        bm25_hits = ms.fts_search(conn, fts_query, limit=fetch_k)
    except Exception:
        bm25_hits = []

    # Apply domain/space filter to BM25 results (FTS5 doesn't do this natively)
    if domain:
        bm25_hits = [h for h in bm25_hits if h["domain"].startswith(domain)]
    if space:
        bm25_hits = [h for h in bm25_hits if h["space"] == space]

    bm25_ranking = [h["id"] for h in bm25_hits]

    # ── RRF fusion ────────────────────────────────────────────────────────────
    fused = rrf([knn_ranking, bm25_ranking])

    # ── Fetch, filter, and sort ───────────────────────────────────────────────
    results: list[RetrievalResult] = []
    seen: set[str] = set()

    for memory_id in sorted(fused, key=lambda m: fused[m], reverse=True):
        if memory_id in seen:
            continue
        seen.add(memory_id)

        mem = ms.get_memory(conn, memory_id, track_access=False)
        if not mem:
            continue
        if mem["confidence"] < min_confidence:
            continue

        d = mem["data"]
        results.append(
            RetrievalResult(
                memory_id=memory_id,
                summary=d.get("summary", ""),
                domain=mem["domain"],
                tags=d.get("tags", []),
                content_type=mem["content_type"],
                importance=mem["importance"],
                confidence=mem["confidence"],
                score=round(fused[memory_id], 4),
            )
        )
        if len(results) >= k:
            break

    return results
