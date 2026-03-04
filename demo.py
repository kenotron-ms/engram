#!/usr/bin/env python3
"""
engram-lite interactive demo
─────────────────────────────
Play with persistent vector memory right in your terminal.
No API keys needed — uses deterministic fake embeddings.

Usage:
    .venv/bin/python demo.py            # fresh demo DB
    .venv/bin/python demo.py --db path  # use specific DB file
"""

from __future__ import annotations

import sys
from pathlib import Path

# Add repo root to path
sys.path.insert(0, str(Path(__file__).parent))

from datetime import UTC

from amplifier_module_engram_lite.db import memory_md as mmd
from amplifier_module_engram_lite.db import memory_store as ms
from amplifier_module_engram_lite.db import schema as sch
from amplifier_module_engram_lite.db import vector_store as vs

# ── Colours ───────────────────────────────────────────────────────────────────
R = "\033[0m"
BOLD = "\033[1m"
DIM = "\033[2m"
GREEN = "\033[32m"
CYAN = "\033[36m"
YELLOW = "\033[33m"
MAGENTA = "\033[35m"
BLUE = "\033[34m"
RED = "\033[31m"
GRAY = "\033[90m"


def c(text: str, *codes: str) -> str:
    return "".join(codes) + str(text) + R


def hr(char: str = "─", width: int = 60) -> str:
    return c(char * width, DIM)


# ── Helpers ───────────────────────────────────────────────────────────────────

IMPORTANCE_COLORS = {
    "critical": RED + BOLD,
    "high": YELLOW + BOLD,
    "medium": CYAN,
    "low": GRAY,
}
TYPE_ICONS = {
    "preference": "♥",
    "fact": "◆",
    "decision": "⊕",
    "skill": "★",
    "event": "⏱",
    "entity": "◉",
    "relationship": "⤷",
    "constraint": "⚑",
}


def print_memory(mem: dict, index: int | None = None, show_id: bool = True) -> None:
    d = mem["data"]
    imp_color = IMPORTANCE_COLORS.get(mem["importance"], "")
    icon = TYPE_ICONS.get(mem["content_type"], "·")
    prefix = f"{c(str(index), BOLD, CYAN)}. " if index is not None else ""
    print(f"\n{prefix}{c(icon + ' ' + mem['content_type'], imp_color)}  {c(mem['domain'], DIM)}")
    print(f"  {c(d.get('summary', d.get('content', '')[:80]), BOLD)}")
    if d.get("tags"):
        print(f"  {c('tags:', DIM)} {c(', '.join(d['tags']), CYAN)}")
    meta = (
        f"conf:{mem['confidence']:.0%}  imp:{mem['importance']}"
        f"  accessed:{d.get('access_count', 0)}x"
    )
    print(f"  {c(meta, GRAY)}")
    if show_id:
        print(f"  {c('id: ' + mem['id'], GRAY)}")


def print_memory_md(scope: str, project_dir: Path | None = None) -> None:
    text = mmd.read(scope, project_dir)
    if not text:
        print(c(f"  No {scope}-scope MEMORY.md found.", GRAY))
        return
    print(c(f"\n{'─' * 60}", DIM))
    if scope == "user":
        print(c(" ~/.engram/MEMORY.md", BOLD, CYAN))
    else:
        print(c(" .engram/MEMORY.md", BOLD, CYAN))
    print(c(f"{'─' * 60}", DIM))
    # Print just the body (skip frontmatter)
    _, body = mmd._parse_frontmatter(text)
    for line in body.strip().splitlines():
        if line.startswith("##"):
            print(c(line, BOLD, YELLOW))
        elif line.startswith("- ["):
            tag_end = line.index("]") + 1
            tag = line[: tag_end + 2]
            rest = line[tag_end + 2 :]
            print(f"  {c(tag, CYAN)}{rest}")
        elif line.startswith("→"):
            print(c(f"  {line}", GRAY))
        elif line.startswith("<!--"):
            pass  # skip comments
        else:
            print(line)
    print(c(f"{'─' * 60}", DIM))


HELP = f"""
{c("engram-lite demo commands", BOLD, CYAN)}
{hr()}
  {c("capture", BOLD)} <text>           — store a new memory
    {c("--type", DIM)} <type>           types: fact preference decision skill event entity constraint
    {c("--domain", DIM)} <domain>       e.g. personal/prefs  professional/arch  projects/myapp
    {c("--importance", DIM)} <level>    critical high medium low
    {c("--tags", DIM)} tag1,tag2        comma-separated tags

  {c("recall", BOLD)} <query>           — semantic search (vector KNN)
  {c("search", BOLD)} <query>           — keyword search (BM25)
  {c("forget", BOLD)} <id>             — soft-delete a memory
  {c("show", BOLD)}                    — list all memories
  {c("memory", BOLD)}                  — show current MEMORY.md
  {c("stats", BOLD)}                   — show memory statistics
  {c("seed", BOLD)}                    — load example memories to play with
  {c("clear", BOLD)}                   — wipe all memories (fresh start)
  {c("help", BOLD)}                    — show this help
  {c("quit", BOLD)} / {c("exit", BOLD)}              — exit

{c("Examples:", BOLD)}
  {c('capture "I prefer Python over Go for backend services" --type preference --domain personal/prefs', CYAN)}
  {c('capture "Use SQLite for local-first apps" --type decision --domain professional/arch --importance high', CYAN)}
  {c('recall "programming language preferences"', CYAN)}
  {c('search "SQLite"', CYAN)}
"""

SEED_MEMORIES = [
    (
        "I always prefer inductive writing — state the conclusion first, then the evidence",
        "preference",
        "personal/prefs",
        "medium",
        ["writing", "communication"],
    ),
    (
        "Use SQLite for any project that doesn't need concurrent writes from multiple machines",
        "decision",
        "professional/arch",
        "high",
        ["sqlite", "architecture", "databases"],
    ),
    (
        "Avoid Docker in local dev — prefer native tools and venv for Python",
        "constraint",
        "personal/workflow",
        "medium",
        ["docker", "devtools", "python"],
    ),
    (
        "In TypeScript, always use strict mode and explicit return types on exported functions",
        "preference",
        "professional/engineering",
        "medium",
        ["typescript", "coding-style"],
    ),
    (
        "engram-lite uses JSON-first SQLite schema: real columns only for indexed fields",
        "fact",
        "projects/engram-lite",
        "high",
        ["sqlite", "schema", "json", "engram-lite"],
    ),
    (
        "HIPAA requires PHI to be encrypted at rest and in transit"
        " — applies to all healthcare data",
        "fact",
        "professional/healthcare",
        "critical",
        ["hipaa", "compliance", "phi", "encryption"],
    ),
    (
        "Presented the engram-lite architecture design to the team",
        "event",
        "projects/engram-lite",
        "medium",
        ["design", "architecture"],
    ),
    (
        "Mnemis paper: dual-route retrieval (System-1 similarity + System-2 graph traversal)"
        " achieves SOTA on LoCoMo",
        "fact",
        "professional/research",
        "high",
        ["mnemis", "rag", "retrieval", "memory-systems"],
    ),
]


# ── Main REPL ──────────────────────────────────────────────────────────────────


def parse_args_from_line(line: str) -> tuple[str, dict]:
    """Parse a command line into (text, {--type: val, ...})."""
    parts = line.split(" --")
    text = parts[0].strip()
    flags: dict[str, str] = {}
    for part in parts[1:]:
        if " " in part:
            k, _, v = part.partition(" ")
            flags[k.strip()] = v.strip()
        else:
            flags[part.strip()] = "true"
    return text, flags


def run(db_path: str = "~/.engram/demo.db") -> None:
    print(
        f"\n{c('engram-lite', BOLD, CYAN)} {c('memory demo', BOLD)}"
        f"  {c('(type help for commands)', DIM)}"
    )
    print(c(f"DB: {db_path}", GRAY))

    conn, vec_ok = sch.get_db(db_path)
    project_dir = Path.cwd()

    if vec_ok:
        print(c("✓ sqlite-vec loaded — real KNN search active", GREEN))
    else:
        print(c("⚠ sqlite-vec not available — using pure-Python cosine fallback", YELLOW))

    # Initialize MEMORY.md files
    mmd.initialize("user")
    mmd.initialize("project", project_dir, "demo")

    print()

    while True:
        try:
            line = input(c("engram", BOLD, MAGENTA) + c("> ", BOLD)).strip()
        except (EOFError, KeyboardInterrupt):
            print(c("\nBye!", DIM))
            break

        if not line:
            continue

        cmd, *rest_parts = line.split(None, 1)
        rest = rest_parts[0] if rest_parts else ""

        # ── quit ──────────────────────────────────────────────────────────────
        if cmd in ("quit", "exit", "q"):
            print(c("Bye!", DIM))
            break

        # ── help ──────────────────────────────────────────────────────────────
        elif cmd == "help":
            print(HELP)

        # ── capture ───────────────────────────────────────────────────────────
        elif cmd == "capture":
            if not rest:
                print(
                    c(
                        "  Usage: capture <text>"
                        " [--type TYPE] [--domain DOMAIN]"
                        " [--importance LEVEL] [--tags t1,t2]",
                        GRAY,
                    )
                )
                continue
            text, flags = parse_args_from_line(rest)
            content_type = flags.get("type", "fact")
            domain = flags.get("domain", "personal/general")
            importance = flags.get("importance", "medium")
            tags = [t.strip() for t in flags.get("tags", "").split(",") if t.strip()]

            # Generate a summary (first sentence or truncate)
            summary = text.split(".")[0].strip()
            if len(summary) > 120:
                summary = summary[:120] + "…"
            if not summary:
                summary = text[:120]

            # Extract simple keywords (unique words > 4 chars)
            keywords = list({w.lower() for w in text.split() if len(w) > 4})[:10]
            keywords = sorted(set(keywords + tags))

            memory_id = ms.insert_memory(
                conn,
                content=text,
                summary=summary,
                domain=domain,
                space="user",
                content_type=content_type,
                importance=importance,
                tags=tags,
                keywords=keywords,
            )

            # Store vector
            emb = vs.fake_embed(text)
            vs.insert_vector(conn, memory_id, emb)

            # Update MEMORY.md
            entry_type = mmd.ENTRY_TYPE_MAP.get(content_type, "fact")
            entry = mmd.append_entry("user", entry_type, summary)

            print(
                f"\n  {c('✓ Captured', GREEN, BOLD)}  {c(f'[{entry_type}] {summary[:60]}', BOLD)}"
            )
            print(f"  {c('id:', GRAY)} {c(memory_id, DIM)}")
            print(
                f"  {c('domain:', GRAY)} {domain}"
                f"  {c('type:', GRAY)} {content_type}"
                f"  {c('importance:', GRAY)} {importance}"
            )
            if tags:
                print(f"  {c('tags:', GRAY)} {', '.join(tags)}")
            print(f"  {c('MEMORY.md:', GRAY)} {c(entry, CYAN)}")
            print()

        # ── recall ────────────────────────────────────────────────────────────
        elif cmd == "recall":
            if not rest:
                print(c("  Usage: recall <query>", GRAY))
                continue
            query = rest.strip()
            query_vec = vs.fake_embed(query)
            results = vs.knn_search(conn, query_vec, k=5)

            if not results:
                print(c("  No memories found.", GRAY))
                continue

            print(
                f"\n  {c('Semantic recall:', BOLD, CYAN)} {c(query, BOLD)}"
                f"  {c(f'({len(results)} results)', GRAY)}"
            )
            for i, (mem_id, score) in enumerate(results, 1):
                mem = ms.get_memory(conn, mem_id, track_access=False)
                if mem:
                    d = mem["data"]
                    icon = TYPE_ICONS.get(mem["content_type"], "·")
                    bar_len = int(score * 20)
                    bar = c("█" * bar_len, GREEN) + c("░" * (20 - bar_len), GRAY)
                    print(f"\n  {c(str(i), BOLD)} {bar} {c(f'{score:.2f}', BOLD, GREEN)}")
                    print(f"    {icon} {c(d.get('summary', ''), BOLD)}  {c(mem['domain'], DIM)}")
                    if d.get("tags"):
                        print(f"    {c(' '.join('#' + t for t in d['tags']), CYAN)}")
                    print(f"    {c('id: ' + mem_id, GRAY)}")
            print()

        # ── search ────────────────────────────────────────────────────────────
        elif cmd == "search":
            if not rest:
                print(c("  Usage: search <keywords>", GRAY))
                continue
            results = ms.fts_search(conn, rest.strip(), limit=5)
            if not results:
                print(c("  No results.", GRAY))
                continue
            print(
                f"\n  {c('BM25 keyword search:', BOLD, CYAN)} {c(rest, BOLD)}"
                f"  {c(f'({len(results)} results)', GRAY)}"
            )
            for i, mem in enumerate(results, 1):
                print_memory(mem, index=i)
            print()

        # ── forget ────────────────────────────────────────────────────────────
        elif cmd == "forget":
            if not rest:
                print(c("  Usage: forget <memory-id>", GRAY))
                continue
            ok = ms.delete_memory(conn, rest.strip())
            vs.delete_vector(conn, rest.strip())
            if ok:
                print(
                    c(
                        f"  ✓ Memory {rest.strip()} forgotten (soft-deleted, stays in DB).",
                        GREEN,
                    )
                )
            else:
                print(c(f"  Memory not found: {rest.strip()}", RED))

        # ── show ──────────────────────────────────────────────────────────────
        elif cmd == "show":
            memories = ms.get_all(conn, limit=20)
            if not memories:
                print(c("  No memories yet. Try: seed", GRAY))
                continue
            print(f"\n  {c(f'All memories ({len(memories)}):', BOLD, CYAN)}")
            for i, mem in enumerate(memories, 1):
                print_memory(mem, index=i)
            print()

        # ── memory ────────────────────────────────────────────────────────────
        elif cmd == "memory":
            print_memory_md("user")
            if (project_dir / ".engram" / "MEMORY.md").exists():
                print_memory_md("project", project_dir)

        # ── stats ─────────────────────────────────────────────────────────────
        elif cmd == "stats":
            s = ms.stats(conn)
            print(f"\n  {c('Memory statistics', BOLD, CYAN)}")
            print(f"  {c('Total:', GRAY)} {c(s['total'], BOLD, GREEN)} memories")
            if s["by_type"]:
                print(f"\n  {c('By type:', GRAY)}")
                for t, n in sorted(s["by_type"].items()):
                    icon = TYPE_ICONS.get(t, "·")
                    print(f"    {icon} {t:<15} {c(n, BOLD)}")
            if s["by_space"]:
                print(f"\n  {c('By space:', GRAY)}")
                for sp, n in s["by_space"].items():
                    print(f"    {sp:<12} {c(n, BOLD)}")
            if s["top_domains"]:
                print(f"\n  {c('Top domains:', GRAY)}")
                for dom, n in s["top_domains"]:
                    print(f"    {dom:<35} {c(n, BOLD)}")
            print()

        # ── seed ──────────────────────────────────────────────────────────────
        elif cmd == "seed":
            print(c(f"\n  Seeding {len(SEED_MEMORIES)} example memories…", CYAN))
            for content, ctype, domain, importance, tags in SEED_MEMORIES:
                summary = content.split(".")[0].strip()
                if len(summary) > 120:
                    summary = summary[:120] + "…"
                keywords = sorted({w.lower() for w in content.split() if len(w) > 4} | set(tags))
                mid = ms.insert_memory(
                    conn,
                    content=content,
                    summary=summary,
                    domain=domain,
                    space="user",
                    content_type=ctype,
                    importance=importance,
                    tags=tags,
                    keywords=keywords[:15],
                )
                emb = vs.fake_embed(content)
                vs.insert_vector(conn, mid, emb)
                entry_type = mmd.ENTRY_TYPE_MAP.get(ctype, "fact")
                mmd.append_entry("user", entry_type, summary)
                icon = TYPE_ICONS.get(ctype, "·")
                print(f"  {c('✓', GREEN)} {icon} {c(summary[:55], BOLD)}  {c(domain, GRAY)}")
            print(c('\n  Done! Try: recall "what are my coding preferences"', CYAN))
            print(c("         or: memory", CYAN))
            print()

        # ── clear ─────────────────────────────────────────────────────────────
        elif cmd == "clear":
            conn.execute("DELETE FROM memory_fts")
            conn.execute("DELETE FROM memory_tags")
            conn.execute("DELETE FROM capture_log")
            try:
                conn.execute("DELETE FROM memory_vectors")
            except Exception:
                pass
            conn.execute("DELETE FROM memories")
            conn.commit()
            # Reset MEMORY.md
            mmd.initialize("user")
            path = mmd.get_path("user")
            from datetime import datetime

            path.write_text(
                mmd.TEMPLATE_USER.format(now=datetime.now(UTC).strftime("%Y-%m-%dT%H:%M:%SZ"))
            )
            print(c("  ✓ All memories cleared. Fresh start!", GREEN))

        else:
            print(c(f"  Unknown command: {cmd}. Type 'help'.", RED))


if __name__ == "__main__":
    import argparse

    p = argparse.ArgumentParser(description="engram-lite interactive demo")
    p.add_argument(
        "--db",
        default="~/.engram/demo.db",
        help="DB path (default: ~/.engram/demo.db)",
    )
    args = p.parse_args()
    run(args.db)
