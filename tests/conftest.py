"""Shared fixtures for engram-lite tests."""

import pytest

from amplifier_module_engram_lite.db.schema import get_memory_db


@pytest.fixture
def db():
    """In-memory SQLite DB with full schema including sqlite-vec."""
    conn, vec_ok = get_memory_db()
    yield conn, vec_ok
    conn.close()


@pytest.fixture
def conn(db):
    """Convenience: just the connection."""
    connection, _ = db
    return connection


@pytest.fixture
def seeded(conn, tmp_path):
    """DB with 4 pre-captured memories covering different types/domains."""
    from amplifier_module_engram_lite.tools.capture import memory_capture

    ids = {}
    ids["pref"] = memory_capture(
        conn,
        "I prefer TypeScript over JavaScript for all projects",
        content_type="preference",
        domain="personal/prefs",
        tags=["typescript", "javascript"],
        project_dir=tmp_path,
    )["memory_id"]
    ids["decision"] = memory_capture(
        conn,
        "Use SQLite for local-first apps — zero ops, portable, fast",
        content_type="decision",
        domain="professional/arch",
        importance="high",
        tags=["sqlite", "databases"],
        project_dir=tmp_path,
    )["memory_id"]
    ids["fact"] = memory_capture(
        conn,
        "HIPAA requires PHI encrypted at rest and in transit",
        content_type="fact",
        domain="professional/healthcare",
        importance="critical",
        tags=["hipaa", "compliance"],
        project_dir=tmp_path,
    )["memory_id"]
    ids["event"] = memory_capture(
        conn,
        "Shipped engram-lite Wave 4 — Amplifier + Claude Code integration",
        content_type="event",
        domain="projects/engram-lite",
        tags=["milestone"],
        project_dir=tmp_path,
    )["memory_id"]
    return ids
