"""
Manage tool — update, forget, relate, explore graph, stats, and index for stored memories.

Part of engram-lite (amplifier-module-engram-lite).
Covers Issues #12 and #13 — all remaining management tools.
"""

from __future__ import annotations

import json
import sqlite3
from datetime import UTC, datetime
from pathlib import Path

from amplifier_module_engram_lite.db import memory_md as mmd
from amplifier_module_engram_lite.db import memory_store as ms
from amplifier_module_engram_lite.db import vector_store as vs

# ── Valid relation types ──────────────────────────────────────────────────────

VALID_RELATIONS = {
    "relates-to",
    "supports",
    "contradicts",
    "supersedes",
    "exemplifies",
    "part-of",
    "caused-by",
    "decided-in",
    "applies-to",
}


# ── Public API ────────────────────────────────────────────────────────────────


def memory_update(
    conn: sqlite3.Connection,
    memory_id: str,
    *,
    content: str | None = None,
    summary: str | None = None,
    tags: list[str] | None = None,
    importance: str | None = None,
    confidence: float | None = None,
) -> dict:
    """
    Update fields on an existing memory.

    - If content changes: re-embeds and updates vector + FTS
    - If tags change: syncs memory_tags table
    - Returns {success, memory_id, changes_made: list[str]}
    """
    # 1. Fetch existing memory
    mem = ms.get_memory(conn, memory_id, track_access=False)
    if mem is None:
        return {"success": False, "error": "not found"}

    changes_made: list[str] = []
    data: dict = mem["data"]
    content_changed = False

    # 2. Content
    if content is not None and content != data.get("content"):
        data["content"] = content
        content_changed = True
        changes_made.append("content")

    # 3. Summary
    if summary is not None and summary != data.get("summary"):
        data["summary"] = summary
        changes_made.append("summary")

    # 4. Tags (even empty list = clear all)
    if tags is not None:
        conn.execute("DELETE FROM memory_tags WHERE memory_id = ?", (memory_id,))
        for tag in tags:
            conn.execute(
                "INSERT OR IGNORE INTO memory_tags (memory_id, tag) VALUES (?, ?)",
                (memory_id, tag),
            )
        data["tags"] = tags
        changes_made.append("tags")

    # 5. Importance / confidence — capture new values for the real columns
    new_importance: str = mem["importance"]
    new_confidence: float = mem["confidence"]

    if importance is not None and importance != mem["importance"]:
        new_importance = importance
        changes_made.append("importance")

    if confidence is not None and confidence != mem["confidence"]:
        new_confidence = confidence
        changes_made.append("confidence")

    # 6. Stamp modified_at whenever anything changed
    if changes_made:
        data["modified_at"] = datetime.now(UTC).isoformat()

    # 7. Write back the memories row (always safe — no-op when nothing changed)
    conn.execute(
        "UPDATE memories SET data = ?, importance = ?, confidence = ? WHERE id = ?",
        (json.dumps(data), new_importance, new_confidence, memory_id),
    )

    # 8. FTS re-sync if content changed (use latest summary/keywords from data)
    if content_changed:
        conn.execute("DELETE FROM memory_fts WHERE memory_id = ?", (memory_id,))
        conn.execute(
            "INSERT INTO memory_fts (memory_id, content, summary, keywords) VALUES (?, ?, ?, ?)",
            (
                memory_id,
                data["content"],
                data.get("summary", ""),
                " ".join(data.get("keywords", [])),
            ),
        )
        # Re-embed with updated content
        vs.insert_vector(conn, memory_id, vs.embed(data["content"][:512]))

    conn.commit()
    return {"success": True, "memory_id": memory_id, "changes_made": changes_made}


def memory_forget(
    conn: sqlite3.Connection,
    memory_id: str,
    reason: str | None = None,
) -> dict:
    """
    Hard-delete a memory from the DB, vector store, and MEMORY.md.

    CASCADE constraints handle memory_tags, memory_graph_nodes, memory_relations.
    Returns {success, memory_id}.
    """
    ms.delete_memory(conn, memory_id)  # clears FTS + CASCADE-deletes related rows
    vs.delete_vector(conn, memory_id)
    conn.commit()
    return {"success": True, "memory_id": memory_id}


def memory_relate(
    conn: sqlite3.Connection,
    from_id: str,
    to_id: str,
    relation_type: str,
    *,
    strength: float = 0.5,
) -> dict:
    """
    Create a typed directed edge between two memories.

    relation_type: relates-to | supports | contradicts | supersedes |
                   exemplifies | part-of | caused-by | decided-in | applies-to
    Returns {success, from_id, to_id, relation_type, strength}.
    """
    if relation_type not in VALID_RELATIONS:
        return {"success": False, "error": f"invalid relation_type: {relation_type}"}

    if not ms.get_memory(conn, from_id, track_access=False):
        return {"success": False, "error": f"from_id not found: {from_id}"}

    if not ms.get_memory(conn, to_id, track_access=False):
        return {"success": False, "error": f"to_id not found: {to_id}"}

    now = datetime.now(UTC).isoformat()
    conn.execute(
        "INSERT OR REPLACE INTO memory_relations "
        "(from_id, to_id, relation_type, strength, created_at) VALUES (?,?,?,?,?)",
        (from_id, to_id, relation_type, strength, now),
    )
    conn.commit()
    return {
        "success": True,
        "from_id": from_id,
        "to_id": to_id,
        "relation_type": relation_type,
        "strength": strength,
    }


def memory_graph_explore(
    conn: sqlite3.Connection,
    query: str | None = None,
    node_id: str | None = None,
    depth: int = 2,
) -> dict:
    """
    Explore the hierarchical graph of memory domains.

    - If node_id: start from that specific node
    - If query: find matching nodes by keyword
    - Else: start from top-level root nodes
    - Returns {nodes: [{id, label, level, summary, memory_count, children: [...]}]}
    """

    def fetch_subtree(nid: str, remaining_depth: int) -> dict:
        row = conn.execute(
            "SELECT id, label, parent, data FROM graph_nodes WHERE id = ?", (nid,)
        ).fetchone()
        if not row:
            return {}
        d = json.loads(row["data"])
        node: dict = {
            "id": row["id"],
            "label": row["label"],
            "level": d.get("level", 0),
            "summary": d.get("summary"),
            "memory_count": d.get("memory_count", 0),
            "children": [],
        }
        if remaining_depth > 0:
            children = conn.execute(
                "SELECT id FROM graph_nodes WHERE parent = ?", (nid,)
            ).fetchall()
            for child in children:
                child_node = fetch_subtree(child[0], remaining_depth - 1)
                if child_node:
                    node["children"].append(child_node)
        return node

    # Determine root node IDs
    root_ids: list[str] = []

    if node_id:
        root_ids = [node_id]
    elif query:
        seen: set[str] = set()
        for word in query.split():
            rows = conn.execute(
                "SELECT id FROM graph_nodes WHERE label LIKE ?",
                (f"%{word}%",),
            ).fetchall()
            for r in rows:
                if r[0] not in seen:
                    seen.add(r[0])
                    root_ids.append(r[0])
    else:
        rows = conn.execute("SELECT id FROM graph_nodes WHERE parent IS NULL LIMIT 5").fetchall()
        root_ids = [r[0] for r in rows]

    nodes = [n for nid in root_ids if (n := fetch_subtree(nid, depth))]
    return {"nodes": nodes}


def memory_stats(
    conn: sqlite3.Connection,
    space: str | None = None,
) -> dict:
    """
    Return aggregated statistics about the memory store.

    Returns {total, by_type, by_space, top_domains, graph_node_count, oldest, newest}.
    When space is provided, also includes space_filter, space_total, space_by_type,
    and space_top_domains.
    """
    result = ms.stats(conn)

    # Graph node count
    result["graph_node_count"] = conn.execute("SELECT COUNT(*) FROM graph_nodes").fetchone()[0]

    # Oldest / newest memories
    row = conn.execute("SELECT MIN(created_at), MAX(created_at) FROM memories").fetchone()
    result["oldest"] = row[0]
    result["newest"] = row[1]

    # Optional space-scoped overlay
    if space:
        result["space_filter"] = space
        result["space_total"] = conn.execute(
            "SELECT COUNT(*) FROM memories WHERE space = ?", (space,)
        ).fetchone()[0]
        result["space_by_type"] = {
            r[0]: r[1]
            for r in conn.execute(
                "SELECT content_type, COUNT(*) FROM memories WHERE space = ? GROUP BY content_type",
                (space,),
            ).fetchall()
        }
        top = conn.execute(
            "SELECT domain, COUNT(*) as n FROM memories "
            "WHERE space = ? GROUP BY domain ORDER BY n DESC LIMIT 5",
            (space,),
        ).fetchall()
        result["space_top_domains"] = [(r[0], r[1]) for r in top]

    return result


def memory_index(
    conn: sqlite3.Connection,
    action: str = "read",
    scope: str = "all",
    project_dir: Path | None = None,
) -> dict:
    """
    Read, get status of, or trigger rebuild of MEMORY.md hot-surface files.

    action:
      read    — return current content of MEMORY.md file(s)
      status  — return path, exists, line_count, entry_count for each scope
      rebuild — regenerate MEMORY.md from DB (deferred: use CLI)

    scope: user | project | local | all
    """
    scopes = ["user", "project", "local"] if scope == "all" else [scope]

    if action == "read":
        files = []
        for s in scopes:
            content = mmd.read(s, project_dir)
            files.append(
                {
                    "scope": s,
                    "path": str(mmd.get_path(s, project_dir)),
                    "exists": content is not None,
                    "content": content,
                }
            )
        return {"action": "read", "files": files}

    if action == "status":
        files = []
        for s in scopes:
            path = mmd.get_path(s, project_dir)
            if path.exists():
                lines = path.read_text().splitlines()
                entry_count = sum(1 for line in lines if line.startswith("- ["))
                files.append(
                    {
                        "scope": s,
                        "path": str(path),
                        "exists": True,
                        "line_count": len(lines),
                        "entry_count": entry_count,
                    }
                )
            else:
                files.append(
                    {
                        "scope": s,
                        "path": str(path),
                        "exists": False,
                        "line_count": 0,
                        "entry_count": 0,
                    }
                )
        return {"action": "status", "files": files}

    if action == "rebuild":
        return {"action": "rebuild", "message": "Use CLI: engram-lite rebuild-index"}

    return {"success": False, "error": f"unknown action: {action}"}
