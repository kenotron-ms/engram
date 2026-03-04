"""memory_capture — the primary write pipeline for engram-lite."""

from __future__ import annotations

import hashlib
import json
import re
import sqlite3
from datetime import UTC, datetime
from pathlib import Path

# ── Validation sets ──────────────────────────────────────────────────────────

VALID_TYPES = {
    "fact",
    "preference",
    "event",
    "skill",
    "entity",
    "relationship",
    "decision",
    "constraint",
}
VALID_SPACES = {"user", "project", "local"}
VALID_IMPORTANCE = {"critical", "high", "medium", "low"}


# ── Heuristic helpers ────────────────────────────────────────────────────────


def _make_summary(content: str) -> str:
    """First sentence, trimmed, max 200 chars."""
    sentence = re.split(r"[.!?\n]", content.strip())[0].strip()
    return sentence[:200] if sentence else content[:200]


def _infer_domain(conn: sqlite3.Connection, content_type: str, space: str) -> str:
    """Try to reuse the most common domain for this content_type+space combo."""
    row = conn.execute(
        "SELECT domain FROM memories WHERE content_type=? AND space=? "
        "GROUP BY domain ORDER BY COUNT(*) DESC LIMIT 1",
        (content_type, space),
    ).fetchone()
    if row:
        return row[0]
    defaults = {
        "preference": "personal/prefs",
        "constraint": "personal/constraints",
        "event": "personal/events",
        "skill": "professional/skills",
        "decision": "professional/arch",
        "entity": "people",
    }
    return defaults.get(content_type, "personal/general")


def _extract_keywords(content: str, tags: list[str]) -> list[str]:
    """Extract keywords from content plus tags, deduped, max 15."""
    words = re.findall(r"\b[a-zA-Z][a-zA-Z0-9\-]{3,}\b", content)
    stop = {
        "this",
        "that",
        "with",
        "from",
        "have",
        "will",
        "been",
        "were",
        "they",
        "their",
        "there",
        "when",
        "what",
        "which",
        "your",
        "about",
        "into",
        "also",
    }
    keywords = [w.lower() for w in words if w.lower() not in stop]
    seen: set[str] = set()
    result: list[str] = []
    for kw in (tags or []) + keywords:
        if kw not in seen:
            seen.add(kw)
            result.append(kw)
        if len(result) >= 15:
            break
    return result


def _upsert_graph_path(conn: sqlite3.Connection, domain: str) -> str:
    """Upsert graph_nodes for each segment of domain. Return leaf node id."""
    parts = [p for p in domain.strip("/").split("/") if p]
    if not parts:
        parts = ["general"]
    parent_id = None
    leaf_id = ""
    now = datetime.now(UTC).isoformat()
    for level, _ in enumerate(parts, 1):
        label = "/".join(parts[:level])
        node_id = hashlib.md5(label.encode()).hexdigest()[:16]
        data = json.dumps(
            {
                "level": level,
                "summary": None,
                "child_count": 0,
                "memory_count": 0,
                "updated_at": now,
            }
        )
        conn.execute(
            "INSERT OR IGNORE INTO graph_nodes (id, label, parent, data) VALUES (?,?,?,?)",
            (node_id, label, parent_id, data),
        )
        # Increment child_count on parent when adding a new child
        if parent_id:
            conn.execute(
                "UPDATE graph_nodes SET data = json_set(data, '$.child_count', "
                "json_extract(data, '$.child_count') + 1) WHERE id = ? "
                "AND NOT EXISTS (SELECT 1 FROM graph_nodes WHERE parent=? AND id=?)",
                (parent_id, parent_id, node_id),
            )
        parent_id = node_id
        leaf_id = node_id
    conn.commit()
    return leaf_id


# ── Public API ───────────────────────────────────────────────────────────────


def memory_capture(
    conn: sqlite3.Connection,
    content: str,
    *,
    content_type: str = "fact",
    space: str = "user",
    domain: str | None = None,
    importance: str = "medium",
    confidence: float = 0.7,
    tags: list[str] | None = None,
    project_dir: Path | None = None,
) -> dict:
    """
    Capture a memory: embed → insert DB → populate graph → update MEMORY.md.
    No LLM required — uses heuristics for summary and keyword extraction.

    Returns:
        {memory_id, summary, domain, tags, keywords_count, memory_md_entry}
    """
    # 1. Validate inputs
    if content_type not in VALID_TYPES:
        content_type = "fact"
    if space not in VALID_SPACES:
        space = "user"
    if importance not in VALID_IMPORTANCE:
        importance = "medium"

    # 2. Summary heuristic
    summary = _make_summary(content)

    # 3. Domain: infer if not provided
    if domain is None:
        domain = _infer_domain(conn, content_type, space)

    # 4. Keyword extraction
    keywords = _extract_keywords(content, tags or [])

    # 5. Embed the content
    from amplifier_module_engram_lite.db import vector_store as vs

    embedding = vs.embed(f"{content_type}: {summary}\n\n{content[:512]}")

    # 6. Insert into DB
    from amplifier_module_engram_lite.db import memory_store as ms

    memory_id = ms.insert_memory(
        conn,
        content=content,
        summary=summary,
        domain=domain,
        space=space,
        content_type=content_type,
        importance=importance,
        confidence=confidence,
        tags=tags or [],
        keywords=keywords,
    )
    vs.insert_vector(conn, memory_id, embedding)

    # 7. Populate graph nodes for the domain path
    leaf_id = _upsert_graph_path(conn, domain)
    conn.execute(
        "INSERT OR IGNORE INTO memory_graph_nodes (memory_id, node_id) VALUES (?,?)",
        (memory_id, leaf_id),
    )
    conn.execute(
        "UPDATE graph_nodes SET data = json_set(data, '$.memory_count', "
        "json_extract(data, '$.memory_count') + 1, '$.updated_at', ?) WHERE id = ?",
        (datetime.now(UTC).isoformat(), leaf_id),
    )
    conn.commit()

    # 8. Update MEMORY.md
    from amplifier_module_engram_lite.db import memory_md as mmd

    entry_type = mmd.ENTRY_TYPE_MAP.get(content_type, "fact")
    entry_line = mmd.append_entry(space, entry_type, summary, project_dir=project_dir)

    # 9. Return result
    return {
        "memory_id": memory_id,
        "summary": summary,
        "domain": domain,
        "tags": tags or [],
        "keywords_count": len(keywords),
        "memory_md_entry": entry_line,
    }
