"""Vector store — sqlite-vec KNN + pure-Python fallback."""

from __future__ import annotations

import hashlib
import math
import random
import sqlite3
import struct

from .schema import DIMS

# ── Embedding ─────────────────────────────────────────────────────────────────


def fake_embed(text: str, dims: int = DIMS) -> list[float]:
    """
    Deterministic fake embedding — no API needed.
    Same text always → same vector. Good for demos.
    Uses SHA256 of lowercased text to seed RNG.
    """
    seed = int(hashlib.sha256(text.lower().strip().encode()).hexdigest(), 16) % (2**31)
    rng = random.Random(seed)
    vec = [rng.gauss(0, 1) for _ in range(dims)]
    mag = math.sqrt(sum(v * v for v in vec))
    return [v / mag for v in vec] if mag > 0 else vec


def cosine_similarity(a: list[float], b: list[float]) -> float:
    dot = sum(x * y for x, y in zip(a, b, strict=False))
    mag_a = math.sqrt(sum(x * x for x in a))
    mag_b = math.sqrt(sum(x * x for x in b))
    return dot / (mag_a * mag_b) if mag_a > 0 and mag_b > 0 else 0.0


def _pack(vec: list[float]) -> bytes:
    return struct.pack(f"{len(vec)}f", *vec)


# ── sqlite-vec operations ──────────────────────────────────────────────────────


def insert_vector(conn: sqlite3.Connection, memory_id: str, embedding: list[float]) -> None:
    try:
        conn.execute(
            "INSERT OR REPLACE INTO memory_vectors (memory_id, embedding) VALUES (?, ?)",
            (memory_id, _pack(embedding)),
        )
        conn.commit()
    except Exception:
        pass  # sqlite-vec not available, skip


def delete_vector(conn: sqlite3.Connection, memory_id: str) -> None:
    try:
        conn.execute("DELETE FROM memory_vectors WHERE memory_id = ?", (memory_id,))
        conn.commit()
    except Exception:
        pass


def knn_search(
    conn: sqlite3.Connection,
    query_vec: list[float],
    k: int = 5,
    domain: str | None = None,
    space: str | None = None,
) -> list[tuple[str, float]]:
    """KNN via sqlite-vec. Falls back to pure-Python cosine over all vectors."""
    try:
        # Try sqlite-vec path
        packed_query = _pack(query_vec)
        if domain or space:
            # Filtered KNN — need a JOIN
            where_parts = ["m.superseded_by IS NULL"]
            params: list = []
            if space:
                where_parts.append("m.space = ?")
                params.append(space)
            if domain:
                where_parts.append("m.domain LIKE ?")
                params.append(domain + "%")
            where = " AND ".join(where_parts)
            params_final = [packed_query] + params + [k]
            rows = conn.execute(
                f"""SELECT v.memory_id, vec_distance_cosine(v.embedding, ?) as dist
                    FROM memory_vectors v
                    JOIN memories m ON m.id = v.memory_id
                    WHERE {where}
                    ORDER BY dist LIMIT ?""",
                params_final,
            ).fetchall()
        else:
            rows = conn.execute(
                """SELECT memory_id, vec_distance_cosine(embedding, ?) as dist
                   FROM memory_vectors
                   ORDER BY dist LIMIT ?""",
                (_pack(query_vec), k),
            ).fetchall()
        # cosine distance = 1 - similarity; convert back
        return [(r[0], 1.0 - r[1]) for r in rows]
    except Exception:
        # Pure-Python fallback
        return _python_knn(conn, query_vec, k, domain, space)


def _python_knn(
    conn: sqlite3.Connection,
    query_vec: list[float],
    k: int,
    domain: str | None,
    space: str | None,
) -> list[tuple[str, float]]:
    """Fallback: load all memories, compute cosine in Python."""

    where = ["superseded_by IS NULL"]
    params: list = []
    if space:
        where.append("space = ?")
        params.append(space)
    if domain:
        where.append("domain LIKE ?")
        params.append(domain + "%")
    where_clause = " AND ".join(where)
    rows = conn.execute(
        f"SELECT id, json_extract(data, '$.content') as content FROM memories WHERE {where_clause}",
        params,
    ).fetchall()
    scored = []
    for row in rows:
        vec = fake_embed(row["content"] or "")
        sim = cosine_similarity(query_vec, vec)
        scored.append((row["id"], sim))
    scored.sort(key=lambda x: x[1], reverse=True)
    return scored[:k]
