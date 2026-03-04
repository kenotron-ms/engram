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


def _traverse_relations(
    conn: sqlite3.Connection,
    start_id: str,
    depth: int,
) -> list[dict]:
    """BFS over memory_relations starting from a memory_id.

    Returns a flat list of node dicts, each with ``related`` listing outgoing
    and incoming edges.  The start node is always first (level=0).
    """
    visited: set[str] = set()
    queue: list[tuple[str, int]] = [(start_id, 0)]
    nodes: list[dict] = []

    while queue:
        mid, d = queue.pop(0)
        if mid in visited:
            continue
        visited.add(mid)

        mem = ms.get_memory(conn, mid, track_access=False)
        if not mem:
            continue

        out_edges = conn.execute(
            "SELECT to_id, relation_type, strength FROM memory_relations WHERE from_id = ?",
            (mid,),
        ).fetchall()
        in_edges = conn.execute(
            "SELECT from_id, relation_type, strength FROM memory_relations WHERE to_id = ?",
            (mid,),
        ).fetchall()

        related = [
            {"memory_id": r[0], "relation": r[1], "strength": r[2], "direction": "out"}
            for r in out_edges
        ] + [
            {"memory_id": r[0], "relation": r[1], "strength": r[2], "direction": "in"}
            for r in in_edges
        ]

        d_data = mem["data"]
        nodes.append(
            {
                "id": mid,
                "label": d_data.get("summary", mid)[:80],
                "level": d,
                "domain": mem["domain"],
                "content_type": mem["content_type"],
                "importance": mem["importance"],
                "related": related,
                "children": [],
            }
        )

        if d < depth:
            for r in out_edges:
                if r[0] not in visited:
                    queue.append((r[0], d + 1))
            for r in in_edges:
                if r[0] not in visited:
                    queue.append((r[0], d + 1))

    return nodes


def memory_graph_explore(
    conn: sqlite3.Connection,
    query: str | None = None,
    node_id: str | None = None,
    depth: int = 2,
) -> dict:
    """
    Explore the knowledge graph — both the domain hierarchy and memory relations.

    Modes:
    - node_id is a **memory_id** → BFS over memory_relations (who relates to whom)
    - node_id is a **graph_node_id** → domain hierarchy subtree (existing behaviour)
    - query matching domain labels → domain hierarchy subtree
    - query with no domain match → FTS search over memory content, then relations BFS
    - no args → top-level domain nodes

    Returns {nodes: [...]}.  Relation-traversal nodes carry a ``related`` list;
    domain-hierarchy nodes carry ``children`` and ``memory_count``.
    """

    def fetch_domain_subtree(nid: str, remaining_depth: int) -> dict:
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
                child_node = fetch_domain_subtree(child[0], remaining_depth - 1)
                if child_node:
                    node["children"].append(child_node)
        return node

    # ── node_id given ──────────────────────────────────────────────────────────
    if node_id:
        # Is it a memory_id?
        if ms.get_memory(conn, node_id, track_access=False):
            return {"nodes": _traverse_relations(conn, node_id, depth)}
        # Treat as a domain graph_node_id (original behaviour)
        node = fetch_domain_subtree(node_id, depth)
        return {"nodes": [node] if node else []}

    # ── query given ────────────────────────────────────────────────────────────
    if query:
        # 1. Try domain label match first
        domain_ids: list[str] = []
        seen: set[str] = set()
        for word in query.split():
            rows = conn.execute(
                "SELECT id FROM graph_nodes WHERE label LIKE ?",
                (f"%{word}%",),
            ).fetchall()
            for r in rows:
                if r[0] not in seen:
                    seen.add(r[0])
                    domain_ids.append(r[0])

        if domain_ids:
            nodes = [n for nid in domain_ids if (n := fetch_domain_subtree(nid, depth))]
            return {"nodes": nodes}

        # 2. No domain match — fall back to FTS over memory content + relations
        try:
            fts_rows = conn.execute(
                "SELECT memory_id FROM memory_fts WHERE memory_fts MATCH ? LIMIT 5",
                (query,),
            ).fetchall()
        except Exception:
            fts_rows = []

        if fts_rows:
            all_nodes: list[dict] = []
            seen_mids: set[str] = set()
            for row in fts_rows:
                mid = row[0]
                if mid not in seen_mids:
                    for node in _traverse_relations(conn, mid, min(depth, 1)):
                        if node["id"] not in seen_mids:
                            seen_mids.add(node["id"])
                            all_nodes.append(node)
            return {"nodes": all_nodes}

        return {"nodes": []}

    # ── no args — top-level domain nodes ──────────────────────────────────────
    rows = conn.execute("SELECT id FROM graph_nodes WHERE parent IS NULL LIMIT 5").fetchall()
    nodes = [n for r in rows if (n := fetch_domain_subtree(r[0], depth))]
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
    content: str | None = None,
) -> dict:
    """
    Read, write, or check status of MEMORY.md hot-surface files.

    action:
      read    — return current content of MEMORY.md file(s)
      write   — write content to MEMORY.md for the given scope
      status  — return path, exists, line_count, entry_count for each scope

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

    if action == "write":
        target = scopes[0] if scopes else "user"  # "all" not valid for write
        if target == "all":
            return {"success": False, "error": "scope='all' not valid for action='write'"}
        if not content:
            return {"success": False, "error": "content is required for action='write'"}
        path = mmd.get_path(target, project_dir)
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_text(content)
        return {"action": "write", "written": True, "scope": target, "path": str(path)}

    return {"success": False, "error": f"unknown action: {action}"}
