"""Tests for DB layer: schema, memory_store, vector_store, memory_md."""

import sqlite3

import pytest

from amplifier_module_engram_lite.db import memory_md as mmd
from amplifier_module_engram_lite.db import memory_store as ms
from amplifier_module_engram_lite.db import vector_store as vs


class TestSchema:
    def test_schema_creates_all_tables(self, conn):
        tables = {
            r[0]
            for r in conn.execute("SELECT name FROM sqlite_master WHERE type='table'").fetchall()
        }
        required = {
            "memories",
            "memory_tags",
            "memory_relations",
            "graph_nodes",
            "memory_graph_nodes",
            "capture_log",
        }
        assert required.issubset(tables), f"missing: {required - tables}"

    def test_schema_creates_fts5(self, conn):
        row = conn.execute(
            "SELECT name FROM sqlite_master WHERE type='table' AND name='memory_fts'"
        ).fetchone()
        assert row is not None

    def test_schema_version(self, conn):
        from amplifier_module_engram_lite.db.schema import SCHEMA_VERSION

        ver = conn.execute("PRAGMA user_version").fetchone()[0]
        assert ver == SCHEMA_VERSION

    def test_schema_idempotent(self, db):
        """ensure_schema is safe to call twice."""
        conn, _ = db
        from amplifier_module_engram_lite.db.schema import ensure_schema

        ensure_schema(conn)  # second call — must not raise
        count = conn.execute("SELECT COUNT(*) FROM memories").fetchone()[0]
        assert count == 0  # still empty

    def test_data_json_check_constraint(self, conn):
        """CHECK(json_valid(data)) rejects invalid JSON."""
        with pytest.raises(sqlite3.IntegrityError):
            conn.execute(
                "INSERT INTO memories"
                " (id,space,domain,content_type,importance,confidence,created_at,data)"
                " VALUES ('x','user','d','fact','medium',0.7,'2026-01-01','not json')"
            )


class TestMemoryStore:
    def test_insert_and_get(self, conn):
        mid = ms.insert_memory(
            conn,
            content="test content",
            summary="test summary",
            domain="personal/test",
            space="user",
            content_type="fact",
            importance="medium",
            confidence=0.7,
            tags=["test"],
            keywords=["test"],
        )
        mem = ms.get_memory(conn, mid, track_access=False)
        assert mem is not None
        assert mem["data"]["content"] == "test content"
        assert mem["domain"] == "personal/test"

    def test_get_tracks_access(self, conn):
        mid = ms.insert_memory(
            conn,
            content="x",
            summary="x",
            domain="d",
            space="user",
            content_type="fact",
            importance="medium",
            confidence=0.7,
        )
        ms.get_memory(conn, mid, track_access=True)
        ms.get_memory(conn, mid, track_access=True)
        mem = ms.get_memory(conn, mid, track_access=False)
        assert mem is not None
        assert mem["data"]["access_count"] == 2

    def test_get_nonexistent(self, conn):
        assert ms.get_memory(conn, "doesnotexist") is None

    def test_delete(self, conn):
        mid = ms.insert_memory(
            conn,
            content="to delete",
            summary="bye",
            domain="d",
            space="user",
            content_type="fact",
            importance="low",
            confidence=0.5,
        )
        assert ms.delete_memory(conn, mid) is True
        assert ms.get_memory(conn, mid) is None

    def test_delete_cascades_tags(self, conn):
        mid = ms.insert_memory(
            conn,
            content="x",
            summary="x",
            domain="d",
            space="user",
            content_type="fact",
            importance="medium",
            confidence=0.7,
            tags=["a", "b"],
        )
        ms.delete_memory(conn, mid)
        tags = conn.execute("SELECT tag FROM memory_tags WHERE memory_id=?", (mid,)).fetchall()
        assert tags == []

    def test_fts_search(self, conn):
        ms.insert_memory(
            conn,
            content="unique xylophone zebra",
            summary="unique xylophone zebra",
            domain="d",
            space="user",
            content_type="fact",
            importance="medium",
            confidence=0.7,
            keywords=["xylophone"],
        )
        results = ms.fts_search(conn, '"xylophone"', limit=5)
        assert any("xylophone" in r["data"].get("content", "") for r in results)

    def test_stats(self, seeded, conn):
        s = ms.stats(conn)
        assert s["total"] == 4
        assert "preference" in s["by_type"]
        assert "decision" in s["by_type"]


class TestVectorStore:
    def test_embed_returns_correct_dims(self):
        import math

        vec = vs.embed("hello world")
        from amplifier_module_engram_lite.db.schema import DIMS

        assert len(vec) == DIMS
        # normalised: magnitude ≈ 1
        mag = math.sqrt(sum(v * v for v in vec))
        assert abs(mag - 1.0) < 0.01

    def test_insert_and_knn(self, db):
        conn, vec_ok = db
        if not vec_ok:
            pytest.skip("sqlite-vec not available")
        mid = ms.insert_memory(
            conn,
            content="TypeScript",
            summary="TypeScript",
            domain="d",
            space="user",
            content_type="fact",
            importance="medium",
            confidence=0.7,
        )
        vec = vs.embed("TypeScript")
        vs.insert_vector(conn, mid, vec)
        results = vs.knn_search(conn, vec, k=1)
        assert results
        assert results[0][0] == mid
        assert results[0][1] > 0.99  # same vector = similarity ~1.0

    def test_delete_vector(self, db):
        conn, vec_ok = db
        if not vec_ok:
            pytest.skip("sqlite-vec not available")
        mid = ms.insert_memory(
            conn,
            content="x",
            summary="x",
            domain="d",
            space="user",
            content_type="fact",
            importance="medium",
            confidence=0.7,
        )
        vs.insert_vector(conn, mid, vs.embed("x"))
        vs.delete_vector(conn, mid)
        results = vs.knn_search(conn, vs.embed("x"), k=5)
        assert all(r[0] != mid for r in results)


class TestMemoryMD:
    def test_initialize_creates_file(self, tmp_path):
        path = mmd.initialize("project", project_dir=tmp_path, project_name="test")
        assert path.exists()
        text = path.read_text()
        assert "## Project: test" in text
        assert "## Now" in text

    def test_append_entry(self, tmp_path):
        path = mmd.initialize("project", project_dir=tmp_path, project_name="test")
        entry = mmd.append_entry("project", "pref", "TypeScript preferred", project_dir=tmp_path)
        assert entry.startswith("- [pref]")
        text = path.read_text()
        assert "TypeScript preferred" in text

    def test_entry_count_updates_frontmatter(self, tmp_path):
        mmd.initialize("project", project_dir=tmp_path, project_name="test")
        mmd.append_entry("project", "fact", "fact one", project_dir=tmp_path)
        mmd.append_entry("project", "fact", "fact two", project_dir=tmp_path)
        path = mmd.get_path("project", tmp_path)
        text = path.read_text()
        assert "entries: 2" in text
