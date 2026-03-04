"""Vector store — sentence-transformers embeddings + sqlite-vec KNN."""

from __future__ import annotations

import math
import sqlite3
import struct
from typing import Any

# ── Embedding model (lazy-loaded singleton) ───────────────────────────────────

MODEL_NAME = "all-MiniLM-L6-v2"  # 384 dims, ~80MB, fully local
_model: Any = None  # SentenceTransformer, typed as Any to avoid hard import


def _get_model() -> Any:
    global _model
    if _model is None:
        from sentence_transformers import (
            SentenceTransformer,  # type: ignore[import-untyped]  # noqa: PLC0415
        )

        _model = SentenceTransformer(MODEL_NAME)
    return _model


def embed(text: str) -> list[float]:
    """Encode text to a normalised 384-dim vector using all-MiniLM-L6-v2."""
    return _get_model().encode(text, normalize_embeddings=True).tolist()


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
        pass  # sqlite-vec not available


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
    """KNN via sqlite-vec. Falls back to pure-Python cosine if unavailable."""
    try:
        packed_query = _pack(query_vec)
        if domain or space:
            where_parts: list[str] = []
            params: list[Any] = [packed_query]
            if space:
                where_parts.append("m.space = ?")
                params.append(space)
            if domain:
                where_parts.append("m.domain LIKE ?")
                params.append(domain + "%")
            where = " AND ".join(where_parts)
            params.append(k)
            rows = conn.execute(
                f"""SELECT v.memory_id, vec_distance_cosine(v.embedding, ?) as dist
                    FROM memory_vectors v
                    JOIN memories m ON m.id = v.memory_id
                    {"WHERE " + where if where else ""}
                    ORDER BY dist LIMIT ?""",
                params,
            ).fetchall()
        else:
            rows = conn.execute(
                """SELECT memory_id, vec_distance_cosine(embedding, ?) as dist
                   FROM memory_vectors
                   ORDER BY dist LIMIT ?""",
                (packed_query, k),
            ).fetchall()
        # vec_distance_cosine returns cosine distance (0=identical, 2=opposite)
        # convert to similarity: 1 - dist gives range [-1, 1]
        return [(r[0], 1.0 - r[1]) for r in rows]
    except Exception:
        return _python_knn(conn, query_vec, k, domain, space)


def _python_knn(
    conn: sqlite3.Connection,
    query_vec: list[float],
    k: int,
    domain: str | None,
    space: str | None,
) -> list[tuple[str, float]]:
    """Fallback: re-embed all memories in Python and rank by cosine similarity."""
    where: list[str] = []
    params: list[Any] = []
    if space:
        where.append("space = ?")
        params.append(space)
    if domain:
        where.append("domain LIKE ?")
        params.append(domain + "%")
    clause = "WHERE " + " AND ".join(where) if where else ""
    rows = conn.execute(
        f"SELECT id, json_extract(data, '$.content') as content FROM memories {clause}",
        params,
    ).fetchall()
    scored = [
        (row["id"], cosine_similarity(query_vec, embed(row["content"] or ""))) for row in rows
    ]
    scored.sort(key=lambda x: x[1], reverse=True)
    return scored[:k]
