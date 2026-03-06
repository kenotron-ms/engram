"""MEMORY.md file manager — the hot-context surface for memory injection."""

from __future__ import annotations

from pathlib import Path


TEMPLATE_USER = """\
No memories yet. Use memory_capture() to start building your knowledge store.

---
More on: (nothing recorded yet)
→ memory_recall("topic") to surface it
"""

TEMPLATE_PROJECT = """\
No project memories yet. Use memory_capture(space="project") to start.

---
More on: (nothing recorded yet)
→ memory_recall("topic") to surface it
"""


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
        if scope == "user":
            path.write_text(TEMPLATE_USER)
        else:
            path.write_text(TEMPLATE_PROJECT)
    return path


def read(scope: str, project_dir: Path | None = None) -> str | None:
    path = get_path(scope, project_dir)
    return path.read_text() if path.exists() else None
