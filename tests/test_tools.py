"""Tests for tools layer and retrieval engine."""

from amplifier_module_engram_lite.retrieval.router import _detect_route
from amplifier_module_engram_lite.tools.capture import memory_capture
from amplifier_module_engram_lite.tools.manage import (
    memory_forget,
    memory_graph_explore,
    memory_relate,
    memory_stats,
    memory_update,
)
from amplifier_module_engram_lite.tools.recall import memory_recall, memory_search


class TestCapture:
    def test_capture_returns_required_fields(self, conn, tmp_path):
        r = memory_capture(
            conn, "test fact", content_type="fact", domain="test/domain", project_dir=tmp_path
        )
        assert "memory_id" in r
        assert "summary" in r
        assert "memory_md_entry" in r
        assert r["memory_md_entry"].startswith("- [")

    def test_capture_inserts_into_db(self, conn, tmp_path):
        from amplifier_module_engram_lite.db import memory_store as ms

        r = memory_capture(
            conn, "stored content", content_type="fact", domain="d/test", project_dir=tmp_path
        )
        mem = ms.get_memory(conn, r["memory_id"], track_access=False)
        assert mem is not None
        assert mem["data"]["content"] == "stored content"

    def test_capture_populates_graph_nodes(self, conn, tmp_path):
        memory_capture(
            conn,
            "x",
            content_type="fact",
            domain="professional/arch/microservices",
            project_dir=tmp_path,
        )
        nodes = conn.execute("SELECT label FROM graph_nodes ORDER BY label").fetchall()
        labels = [n[0] for n in nodes]
        assert "professional" in labels
        assert "professional/arch" in labels
        assert "professional/arch/microservices" in labels

    def test_capture_infers_domain_when_missing(self, conn, tmp_path):
        r = memory_capture(
            conn, "I prefer dark mode", content_type="preference", project_dir=tmp_path
        )
        assert r["domain"] is not None
        assert len(r["domain"]) > 0

    def test_capture_tags_stored(self, conn, tmp_path):
        from amplifier_module_engram_lite.db import memory_store as ms

        r = memory_capture(
            conn,
            "content",
            content_type="fact",
            domain="d",
            tags=["alpha", "beta"],
            project_dir=tmp_path,
        )
        mem = ms.get_memory(conn, r["memory_id"], track_access=False)
        assert mem is not None
        assert set(mem["data"]["tags"]) == {"alpha", "beta"}


class TestRecall:
    def test_recall_returns_captured_memory(self, seeded, conn):
        results = memory_recall(conn, "typescript programming language", k=5)
        assert results
        summaries = [r["summary"].lower() for r in results]
        assert any("typescript" in s for s in summaries), f"TypeScript not in: {summaries}"

    def test_recall_respects_k(self, seeded, conn):
        results = memory_recall(conn, "knowledge", k=2)
        assert len(results) <= 2

    def test_recall_all_routes_work(self, seeded, conn):
        for route in ("auto", "vector", "graph", "hybrid", "keyword"):
            results = memory_recall(conn, "professional knowledge", route=route, k=3)
            assert isinstance(results, list), f"route={route} returned non-list"

    def test_search_finds_exact_term(self, seeded, conn):
        results = memory_search(conn, "HIPAA", limit=5)
        assert results
        assert any("hipaa" in r["summary"].lower() or "HIPAA" in str(r) for r in results)

    def test_recall_result_has_required_fields(self, seeded, conn):
        results = memory_recall(conn, "any query", k=1)
        if results:
            r = results[0]
            for field in ("memory_id", "summary", "domain", "tags", "score"):
                assert field in r, f"missing field: {field}"


class TestManage:
    def test_update_summary(self, conn, tmp_path):
        r = memory_capture(conn, "original", content_type="fact", domain="d", project_dir=tmp_path)
        u = memory_update(conn, r["memory_id"], summary="updated summary")
        assert u["success"]
        assert "summary" in u["changes_made"]

    def test_update_nonexistent(self, conn):
        u = memory_update(conn, "nonexistent-id", summary="x")
        assert not u["success"]

    def test_forget_removes_memory(self, conn, tmp_path):
        from amplifier_module_engram_lite.db import memory_store as ms

        r = memory_capture(conn, "to delete", content_type="fact", domain="d", project_dir=tmp_path)
        f = memory_forget(conn, r["memory_id"])
        assert f["success"]
        assert ms.get_memory(conn, r["memory_id"]) is None

    def test_relate_creates_edge(self, conn, tmp_path):
        r1 = memory_capture(conn, "A", content_type="fact", domain="d", project_dir=tmp_path)
        r2 = memory_capture(conn, "B", content_type="fact", domain="d", project_dir=tmp_path)
        rel = memory_relate(conn, r1["memory_id"], r2["memory_id"], "relates-to")
        assert rel["success"]
        edges = conn.execute("SELECT COUNT(*) FROM memory_relations").fetchone()[0]
        assert edges == 1

    def test_relate_invalid_type(self, conn, tmp_path):
        r1 = memory_capture(conn, "A", content_type="fact", domain="d", project_dir=tmp_path)
        r2 = memory_capture(conn, "B", content_type="fact", domain="d", project_dir=tmp_path)
        rel = memory_relate(conn, r1["memory_id"], r2["memory_id"], "invalid-type")
        assert not rel["success"]

    def test_graph_explore(self, seeded, conn):
        g = memory_graph_explore(conn, query="professional", depth=2)
        assert "nodes" in g
        assert len(g["nodes"]) > 0

    def test_stats_correct_counts(self, seeded, conn):
        s = memory_stats(conn)
        assert s["total"] == 4
        assert s["graph_node_count"] > 0


class TestRouter:
    def test_detect_route_keyword_for_acronym(self):
        assert _detect_route("HIPAA") == "keyword"
        assert _detect_route("SQL") == "keyword"

    def test_detect_route_vector_for_short_query(self):
        assert _detect_route("sqlite") == "vector"

    def test_detect_route_hybrid_for_broad(self):
        route = _detect_route("what do you know about everything")
        assert route == "hybrid"

    def test_system1_returns_results(self, seeded, conn):
        from amplifier_module_engram_lite.db import vector_store as vs
        from amplifier_module_engram_lite.retrieval.system1 import system1_recall

        qvec = vs.embed("typescript preferences")
        results = system1_recall(conn, "typescript preferences", qvec, k=3)
        assert results
        assert all(hasattr(r, "memory_id") for r in results)

    def test_system2_returns_results(self, seeded, conn):
        from amplifier_module_engram_lite.retrieval.system2 import system2_recall

        results = system2_recall(conn, "professional", k=5)
        assert isinstance(results, list)

    def test_rrf_scoring(self):
        from amplifier_module_engram_lite.retrieval.system1 import rrf

        scores = rrf([["a", "b", "c"], ["b", "c", "a"]])
        # "b" appears at rank 1 in list0 and rank 0 in list1 — should outscore "a" (rank 0, rank 2)
        assert scores["b"] > scores["a"]
        assert scores["c"] > 0


class TestContextBuilder:
    def test_recall_nudge_format(self):
        from amplifier_module_engram_lite.hooks.context_builder import RECALL_NUDGE

        assert '<system-reminder source="engram-lite">' in RECALL_NUDGE
        assert "</system-reminder>" in RECALL_NUDGE

    def test_capture_reminder_format(self):
        from amplifier_module_engram_lite.hooks.context_builder import CAPTURE_REMINDER

        assert "CAPTURE" in CAPTURE_REMINDER
        assert "silent" in CAPTURE_REMINDER.lower()

    def test_build_session_context_returns_string(self):
        from amplifier_module_engram_lite.hooks.context_builder import build_session_context

        ctx = build_session_context()
        assert isinstance(ctx, str)
        assert '<system-reminder source="engram-lite">' in ctx
        assert "</system-reminder>" in ctx
