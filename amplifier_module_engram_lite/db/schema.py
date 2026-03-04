"""SQLite schema — JSON-first design. Real columns only for indexed fields."""

from __future__ import annotations

import sqlite3
from pathlib import Path

SCHEMA_VERSION = 1
DIMS = 384  # all-MiniLM-L6-v2 output dimensions

DDL = """
PRAGMA journal_mode=WAL;
PRAGMA foreign_keys=ON;

CREATE TABLE IF NOT EXISTS memories (
    id            TEXT PRIMARY KEY,
    space         TEXT NOT NULL CHECK (space IN ('user','project','local')),
    domain        TEXT NOT NULL,
    content_type  TEXT NOT NULL CHECK (content_type IN (
                    'fact','preference','event','skill',
                    'entity','relationship','decision','constraint')),
    importance    TEXT NOT NULL CHECK (importance IN ('critical','high','medium','low')),
    confidence    REAL NOT NULL CHECK (confidence BETWEEN 0.0 AND 1.0),
    created_at    TEXT NOT NULL,
    data          TEXT NOT NULL CHECK (json_valid(data))
);

CREATE INDEX IF NOT EXISTS idx_mem_space        ON memories(space);
CREATE INDEX IF NOT EXISTS idx_mem_domain       ON memories(domain);
CREATE INDEX IF NOT EXISTS idx_mem_space_domain ON memories(space, domain);
CREATE INDEX IF NOT EXISTS idx_mem_content_type ON memories(content_type);
CREATE INDEX IF NOT EXISTS idx_mem_importance   ON memories(importance);
CREATE INDEX IF NOT EXISTS idx_mem_confidence   ON memories(confidence);
CREATE INDEX IF NOT EXISTS idx_mem_created_at   ON memories(created_at DESC);

CREATE TABLE IF NOT EXISTS memory_tags (
    memory_id TEXT NOT NULL REFERENCES memories(id) ON DELETE CASCADE,
    tag       TEXT NOT NULL,
    PRIMARY KEY (memory_id, tag)
);
CREATE INDEX IF NOT EXISTS idx_tags_tag ON memory_tags(tag);

CREATE VIRTUAL TABLE IF NOT EXISTS memory_fts USING fts5(
    memory_id UNINDEXED,
    content,
    summary,
    keywords,
    tokenize = 'porter unicode61'
);

CREATE TABLE IF NOT EXISTS memory_relations (
    from_id       TEXT NOT NULL REFERENCES memories(id) ON DELETE CASCADE,
    to_id         TEXT NOT NULL REFERENCES memories(id) ON DELETE CASCADE,
    relation_type TEXT NOT NULL,
    strength      REAL NOT NULL DEFAULT 0.5,
    created_at    TEXT NOT NULL,
    PRIMARY KEY (from_id, to_id, relation_type)
);

CREATE TABLE IF NOT EXISTS graph_nodes (
    id     TEXT PRIMARY KEY,
    label  TEXT NOT NULL UNIQUE,
    parent TEXT REFERENCES graph_nodes(id),
    data   TEXT NOT NULL CHECK (json_valid(data))
);
CREATE INDEX IF NOT EXISTS idx_graph_parent ON graph_nodes(parent);

CREATE TABLE IF NOT EXISTS memory_graph_nodes (
    memory_id TEXT NOT NULL REFERENCES memories(id) ON DELETE CASCADE,
    node_id   TEXT NOT NULL REFERENCES graph_nodes(id) ON DELETE CASCADE,
    PRIMARY KEY (memory_id, node_id)
);
CREATE INDEX IF NOT EXISTS idx_mgn_node ON memory_graph_nodes(node_id);

CREATE TABLE IF NOT EXISTS capture_log (
    id          TEXT PRIMARY KEY,
    memory_id   TEXT REFERENCES memories(id),
    captured_at TEXT NOT NULL,
    data        TEXT NOT NULL CHECK (json_valid(data))
);
"""


def load_vec(conn: sqlite3.Connection) -> bool:
    """Load sqlite-vec extension. Returns True on success."""
    try:
        import importlib

        sqlite_vec = importlib.import_module("sqlite_vec")
        conn.enable_load_extension(True)
        sqlite_vec.load(conn)
        conn.enable_load_extension(False)
        return True
    except Exception:
        return False


def ensure_schema(conn: sqlite3.Connection, with_vec: bool = True) -> bool:
    """Create all tables. Returns True if sqlite-vec is available."""
    conn.row_factory = sqlite3.Row
    conn.executescript(DDL)
    vec_ok = False
    if with_vec:
        vec_ok = load_vec(conn)
        if vec_ok:
            conn.execute(f"""
                CREATE VIRTUAL TABLE IF NOT EXISTS memory_vectors
                USING vec0(memory_id TEXT PRIMARY KEY, embedding FLOAT[{DIMS}])
            """)
            conn.commit()
    conn.execute(f"PRAGMA user_version = {SCHEMA_VERSION}")
    conn.commit()
    return vec_ok


def get_db(path: str | Path) -> tuple[sqlite3.Connection, bool]:
    """Open (or create) a DB. Returns (conn, vec_available)."""
    path = Path(path).expanduser()
    path.parent.mkdir(parents=True, exist_ok=True)
    conn = sqlite3.connect(str(path), check_same_thread=False)
    conn.row_factory = sqlite3.Row
    vec_ok = ensure_schema(conn)
    return conn, vec_ok


def get_memory_db() -> tuple[sqlite3.Connection, bool]:
    """In-memory DB for tests."""
    conn = sqlite3.connect(":memory:")
    conn.row_factory = sqlite3.Row
    vec_ok = ensure_schema(conn)
    return conn, vec_ok
