"""MEMORY.md file manager — the hot-context surface for memory injection."""

from __future__ import annotations

import re
import sqlite3
from datetime import UTC, datetime
from pathlib import Path

MAX_ENTRIES_PER_SECTION = 60
MAX_NOW_ENTRIES = 10

IMPORTANCE_WEIGHTS = {"critical": 1.0, "high": 0.8, "medium": 0.5, "low": 0.2}

ENTRY_TYPE_MAP = {
    "preference": "pref",
    "constraint": "constraint",
    "decision": "decision",
    "skill": "skill",
    "entity": "person",
    "event": "event",
    "fact": "arch",
    "relationship": "pattern",
}

TEMPLATE_USER = """\
---
scope: user
updated: {now}
managed-by: engram-lite
entries: 0
---

# Memory

## You
<!-- Personal preferences, constraints, working style — added by memory_capture(). -->
→ No memories yet. Use: memory_capture("I prefer...", content_type="preference")

## Now
<!-- Current session focus — refreshed at session start. -->
→ Starting fresh.
"""

TEMPLATE_PROJECT = """\
---
scope: project
updated: {now}
managed-by: engram-lite
entries: 0
---

# Memory

## Project: {project}
<!-- Project decisions, architecture, status — added by memory_capture(space='project'). -->
→ No project memories yet.

## Now
<!-- Current session focus — refreshed at session start. -->
→ Starting fresh.
"""


def _now() -> str:
    return datetime.now(UTC).strftime("%Y-%m-%dT%H:%M:%SZ")


def get_path(scope: str, project_dir: Path | None = None) -> Path:
    if scope == "user":
        return Path.home() / ".engram" / "MEMORY.md"
    elif scope == "project":
        base = project_dir or Path.cwd()
        return base / ".engram" / "MEMORY.md"
    elif scope == "local":
        base = project_dir or Path.cwd()
        return base / ".engram" / "MEMORY.local.md"
    raise ValueError(f"Unknown scope: {scope}")


def initialize(scope: str, project_dir: Path | None = None, project_name: str = "project") -> Path:
    path = get_path(scope, project_dir)
    path.parent.mkdir(parents=True, exist_ok=True)
    if not path.exists():
        now = _now()
        if scope == "user":
            path.write_text(TEMPLATE_USER.format(now=now))
        else:
            path.write_text(TEMPLATE_PROJECT.format(now=now, project=project_name))
    return path


def read(scope: str, project_dir: Path | None = None) -> str | None:
    path = get_path(scope, project_dir)
    return path.read_text() if path.exists() else None


def _parse_frontmatter(text: str) -> tuple[dict, str]:
    """Split YAML frontmatter from body. Returns ({key:val}, body)."""
    if not text.startswith("---"):
        return {}, text
    end = text.index("\n---\n", 4) if "\n---\n" in text else -1
    if end == -1:
        return {}, text
    fm_block = text[4:end]
    body = text[end + 5 :]
    fm = {}
    for line in fm_block.splitlines():
        if ":" in line:
            k, _, v = line.partition(":")
            fm[k.strip()] = v.strip()
    return fm, body


def append_entry(
    scope: str,
    entry_type: str,
    text: str,
    section: str = "## You",
    project_dir: Path | None = None,
) -> str:
    """Add a new entry line to MEMORY.md. Returns the entry line."""
    path = get_path(scope, project_dir)
    if not path.exists():
        initialize(scope, project_dir)

    content = path.read_text()
    entry_line = f"- [{entry_type}] {text[:100]}"

    # Remove the "no memories yet" placeholder if present
    content = re.sub(r"→ No memories yet.*\n", "", content)
    content = re.sub(r"→ No project memories yet.*\n", "", content)

    # Find the section and append after it
    section_pattern = re.escape(section)
    if re.search(section_pattern, content):
        # Find insertion point: after the section header + comment
        insert_after = re.search(section_pattern + r".*?\n(?:<!--.*?-->\n)?", content, re.DOTALL)
        if insert_after:
            pos = insert_after.end()
            content = content[:pos] + entry_line + "\n" + content[pos:]
    else:
        content += f"\n{section}\n{entry_line}\n"

    # Update frontmatter entry count
    entry_count = len(re.findall(r"^- \[", content, re.MULTILINE))
    content = re.sub(r"entries: \d+", f"entries: {entry_count}", content)
    content = re.sub(r"updated: .*", f"updated: {_now()}", content)

    path.write_text(content)
    return entry_line


def refresh_now(
    scope: str,
    conn: sqlite3.Connection | None = None,
    project_dir: Path | None = None,
    custom_items: list[str] | None = None,
) -> None:
    """Refresh the ## Now section from recent events in DB."""
    path = get_path(scope, project_dir)
    if not path.exists():
        return

    content = path.read_text()

    # Build new Now lines
    now_lines = ["## Now", "<!-- Refreshed at session start. -->"]

    if custom_items:
        for item in custom_items[:MAX_NOW_ENTRIES]:
            now_lines.append(f"- [now] {item[:80]}")
    elif conn:
        rows = conn.execute(
            """SELECT json_extract(data, '$.summary') as summary
               FROM memories
               WHERE content_type = 'event'
               ORDER BY created_at DESC LIMIT 5"""
        ).fetchall()
        for row in rows:
            now_lines.append(f"- [event] {row[0][:80]}")

    now_lines.append('→ Recall anything: memory_recall("{query}")')

    new_now_block = "\n".join(now_lines)

    # Replace existing ## Now section
    if "## Now" in content:
        content = re.sub(
            r"## Now.*?(?=\n##|\Z)",
            new_now_block + "\n",
            content,
            flags=re.DOTALL,
        )
    else:
        content += "\n" + new_now_block + "\n"

    path.write_text(content)
