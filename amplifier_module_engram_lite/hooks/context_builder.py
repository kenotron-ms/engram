"""Builds <system-reminder> injection strings for engram-lite hooks."""

from __future__ import annotations

from pathlib import Path

RECALL_NUDGE = (
    '<system-reminder source="engram-lite">\n'
    'Relevant prior context? → call memory_recall("your query") before responding.\n'
    "</system-reminder>"
)

CAPTURE_REMINDER = (
    '<system-reminder source="engram-lite">\n'
    "CAPTURE CHECK (silent — do not announce):\n"
    "  • New knowledge learned? → memory_capture(content, content_type=...)\n"
    '  • Correction to something wrong? → memory_capture(..., importance="high")\n'
    '  • Pattern seen 2nd+ time? → memory_capture(..., content_type="event")\n'
    "Execute silently. Never tell the user you are doing this.\n"
    "</system-reminder>"
)


def build_session_context(
    user_db: str = "~/.engram/engram.db",
    project_dir: Path | None = None,
) -> str:
    """Read MEMORY.md files and build the session-start injection string.

    Attempts to refresh the user-scope MEMORY.md from the database, then reads
    both user-scope and project-scope MEMORY.md files and assembles them into a
    single ``<system-reminder>`` block suitable for LLM context injection.

    Args:
        user_db: Path to the user-scope SQLite database (supports ``~`` expansion).
        project_dir: Optional project directory; defaults to ``Path.cwd()``.

    Returns:
        A ``<system-reminder>`` XML string containing MEMORY.md content, or a
        bootstrap message when the memory store is empty.
    """
    parts: list[str] = []

    # Refresh + read user-scope MEMORY.md
    user_path = Path("~/.engram/MEMORY.md").expanduser()
    if user_path.exists():
        try:
            from amplifier_module_engram_lite.db.schema import get_db

            conn, _ = get_db(Path(user_db).expanduser())
            from amplifier_module_engram_lite.db import memory_md as mmd

            mmd.refresh_now("user", conn=conn)
            content = user_path.read_text()
            if content.strip():
                parts.append(f"[MEMORY — user]\n{content}")
        except Exception:
            pass

    # Read project-scope MEMORY.md
    base = project_dir or Path.cwd()
    project_path = base / ".engram" / "MEMORY.md"
    if project_path.exists():
        try:
            content = project_path.read_text()
            if content.strip():
                parts.append(f"[MEMORY — project]\n{content}")
        except Exception:
            pass

    if not parts:
        return (
            '<system-reminder source="engram-lite">\n'
            "Memory store is empty. Use memory_capture() to start building your knowledge.\n"
            "Full memory tools: memory_capture | memory_recall | memory_search | memory_stats\n"
            "</system-reminder>"
        )

    body = "\n\n".join(parts)
    footer = "Full memory: memory_recall(query) | memory_search(query) | memory_graph_explore()"
    return f'<system-reminder source="engram-lite">\n{body}\n\n{footer}\n</system-reminder>'
