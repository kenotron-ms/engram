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
    "Worth keeping from this turn? Run silently:\n"
    "  1. memory_capture(content, content_type, domain)   → vector DB\n"
    '  2. memory_index(action="read", scope="user")         → get MEMORY.md\n'
    "  3. Add or update the right section                  → plain markdown\n"
    '  4. memory_index(action="write", scope="user", content=<updated>)\n'
    "Sections: ## Preferences  ## Architecture  ## Stack  ## Debugging  ## Constraints\n"
    "(or whatever fits — you choose the structure)\n"
    "Write naturally. No [type] tags. Keep MEMORY.md under 200 lines.\n"
    "Execute silently. Never tell the user.\n"
    "</system-reminder>"
)


def build_session_context(
    user_db: str = "~/.engram/engram.db",
    project_dir: Path | None = None,
) -> str:
    """Read MEMORY.md files and build the session-start injection string.

    Reads user-scope and project-scope MEMORY.md files and assembles them into a
    single ``<system-reminder>`` block suitable for LLM context injection.

    MEMORY.md is LLM-authored free-form markdown — we read it as-is without
    modifying it. The agent updates it directly via memory_index(action="write").

    Args:
        user_db: Path to the user-scope SQLite database (supports ``~`` expansion).
        project_dir: Optional project directory; defaults to ``Path.cwd()``.

    Returns:
        A ``<system-reminder>`` XML string containing MEMORY.md content, or a
        bootstrap message when no MEMORY.md files exist yet.
    """
    parts: list[str] = []

    # Read user-scope MEMORY.md (LLM-authored — do not modify)
    user_path = Path("~/.engram/MEMORY.md").expanduser()
    if user_path.exists():
        try:
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
            "No MEMORY.md yet. Start building yours:\n"
            "  memory_capture(content, ...) → stores to vector DB\n"
            '  memory_index(action="write", scope="user", content="# Memory\\n...") → hot surface\n'
            "</system-reminder>"
        )

    body = "\n\n".join(parts)
    footer = "Full memory: memory_recall(query) | memory_search(query) | memory_graph_explore()"
    return f'<system-reminder source="engram-lite">\n{body}\n\n{footer}\n</system-reminder>'
