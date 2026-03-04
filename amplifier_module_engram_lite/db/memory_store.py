"""Memory CRUD — JSON-first. data column holds all non-indexed fields."""

from __future__ import annotations

import json
import sqlite3
import uuid
from datetime import UTC, datetime


def _now() -> str:
    return datetime.now(UTC).isoformat()


def _id() -> str:
    return str(uuid.uuid4()).replace("-", "")[:16]


def insert_memory(
    conn: sqlite3.Connection,
    *,
    content: str,
    summary: str,
    domain: str,
    space: str = "user",
    content_type: str = "fact",
    importance: str = "medium",
    confidence: float = 0.7,
    tags: list[str] | None = None,
    keywords: list[str] | None = None,
    detail: str | None = None,
    project: str | None = None,
    source_session: str | None = None,
) -> str:
    """Insert a memory. Returns memory_id."""
    memory_id = _id()
    now = _now()
    data = {
        "content": content,
        "summary": summary,
        "tags": tags or [],
        "keywords": keywords or [],
        "modified_at": now,
        "accessed_at": now,
        "access_count": 0,
        "expires_at": None,
        "visibility": "private",
    }
    if detail:
        data["detail"] = detail
    if project:
        data["project"] = project
    if source_session:
        data["source_session"] = source_session

    conn.execute(
        """INSERT INTO memories
           (id, space, domain, content_type, importance, confidence, created_at, data)
           VALUES (?, ?, ?, ?, ?, ?, ?, ?)""",
        (memory_id, space, domain, content_type, importance, confidence, now, json.dumps(data)),
    )
    # Sync tags
    for tag in tags or []:
        conn.execute(
            "INSERT OR IGNORE INTO memory_tags (memory_id, tag) VALUES (?, ?)",
            (memory_id, tag),
        )
    # Sync FTS
    conn.execute(
        "INSERT INTO memory_fts (memory_id, content, summary, keywords) VALUES (?, ?, ?, ?)",
        (memory_id, content, summary, " ".join(keywords or [])),
    )
    conn.commit()
    return memory_id


def get_memory(conn: sqlite3.Connection, memory_id: str, track_access: bool = True) -> dict | None:
    """Fetch a memory by ID. Returns dict or None."""
    row = conn.execute(
        "SELECT * FROM memories WHERE id = ? AND superseded_by IS NULL", (memory_id,)
    ).fetchone()
    if not row:
        return None
    result = dict(row)
    result["data"] = json.loads(result["data"])
    if track_access:
        d = result["data"]
        d["access_count"] = d.get("access_count", 0) + 1
        d["accessed_at"] = _now()
        conn.execute(
            "UPDATE memories SET data = ? WHERE id = ?",
            (json.dumps(d), memory_id),
        )
        conn.commit()
    return result


def soft_delete(conn: sqlite3.Connection, memory_id: str) -> bool:
    cur = conn.execute(
        "UPDATE memories SET superseded_by = '__deleted__' WHERE id = ?", (memory_id,)
    )
    conn.commit()
    return cur.rowcount > 0


def get_all(
    conn: sqlite3.Connection,
    space: str | None = None,
    domain: str | None = None,
    content_type: str | None = None,
    limit: int = 50,
) -> list[dict]:
    """List active memories with optional filters."""
    where = ["superseded_by IS NULL"]
    params: list = []
    if space:
        where.append("space = ?")
        params.append(space)
    if domain:
        where.append("domain LIKE ?")
        params.append(domain + "%")
    if content_type:
        where.append("content_type = ?")
        params.append(content_type)
    params.append(limit)
    rows = conn.execute(
        f"SELECT * FROM memories WHERE {' AND '.join(where)} ORDER BY created_at DESC LIMIT ?",
        params,
    ).fetchall()
    results = []
    for row in rows:
        r = dict(row)
        r["data"] = json.loads(r["data"])
        results.append(r)
    return results


def fts_search(conn: sqlite3.Connection, query: str, limit: int = 5) -> list[dict]:
    """BM25 full-text search. Returns list of memory dicts."""
    rows = conn.execute(
        """SELECT m.*, fts.rank
           FROM memory_fts fts
           JOIN memories m ON m.id = fts.memory_id
           WHERE memory_fts MATCH ? AND m.superseded_by IS NULL
           ORDER BY fts.rank
           LIMIT ?""",
        (query, limit),
    ).fetchall()
    results = []
    for row in rows:
        r = dict(row)
        r.pop("rank", None)
        r["data"] = json.loads(r["data"])
        results.append(r)
    return results


def stats(conn: sqlite3.Connection) -> dict:
    total = conn.execute("SELECT COUNT(*) FROM memories WHERE superseded_by IS NULL").fetchone()[0]
    by_type = {
        row[0]: row[1]
        for row in conn.execute(
            "SELECT content_type, COUNT(*) FROM memories"
            " WHERE superseded_by IS NULL GROUP BY content_type"
        ).fetchall()
    }
    by_space = {
        row[0]: row[1]
        for row in conn.execute(
            "SELECT space, COUNT(*) FROM memories WHERE superseded_by IS NULL GROUP BY space"
        ).fetchall()
    }
    by_domain = conn.execute(
        "SELECT domain, COUNT(*) as n FROM memories WHERE superseded_by IS NULL"
        " GROUP BY domain ORDER BY n DESC LIMIT 5"
    ).fetchall()
    return {
        "total": total,
        "by_type": by_type,
        "by_space": by_space,
        "top_domains": [(r[0], r[1]) for r in by_domain],
    }
