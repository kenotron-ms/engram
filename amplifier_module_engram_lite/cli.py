"""engram-lite CLI — init, status, refresh-now, rebuild-index."""

from __future__ import annotations

import sys
from pathlib import Path

import click

USER_DB = Path.home() / ".engram" / "engram.db"
PROJECT_DB = Path.cwd() / ".engram" / "engram.db"


def _get_conn(db_path: Path):
    """Open DB, return (conn, vec_ok) or (None, False) if DB doesn't exist."""
    if not db_path.exists():
        return None, False
    from amplifier_module_engram_lite.db.schema import get_db

    return get_db(db_path)


@click.group()
def main() -> None:
    """engram-lite — persistent vector memory for AI agents."""


@main.command()
@click.option("--project-name", default=None, help="Name for project MEMORY.md header.")
def init(project_name: str | None) -> None:
    """Create memory directories and blank MEMORY.md files."""
    from amplifier_module_engram_lite.db import memory_md as mmd
    from amplifier_module_engram_lite.db.schema import get_db

    # User scope
    user_path = mmd.initialize("user")
    click.echo(f"✓ {user_path}")

    # Project scope
    proj_name = project_name or Path.cwd().name
    proj_path = mmd.initialize("project", project_name=proj_name)
    click.echo(f"✓ {proj_path}")

    # Write .engram/.gitignore
    gitignore = Path.cwd() / ".engram" / ".gitignore"
    if not gitignore.exists():
        gitignore.write_text("engram.db\nengram.db-wal\nengram.db-shm\nMEMORY.local.md\n")
        click.echo(f"✓ {gitignore}")

    # Touch the user DB so it exists
    get_db(USER_DB)
    click.echo(f"✓ {USER_DB}")
    click.echo("Ready. Use memory_capture() to start building your memory store.")


@main.command()
def status() -> None:
    """Show memory statistics."""
    from amplifier_module_engram_lite.db import memory_store as ms

    total = 0
    for label, db_path in [("user", USER_DB), ("project", PROJECT_DB)]:
        conn, _ = _get_conn(db_path)
        if not conn:
            continue
        s = ms.stats(conn)
        if not s["total"]:
            continue
        total += s["total"]
        click.echo(f"\n[{label}] {db_path}")
        click.echo(f"  total: {s['total']}")
        if s["by_type"]:
            for t, n in sorted(s["by_type"].items()):
                click.echo(f"  {t:<15} {n}")
        if s["top_domains"]:
            click.echo("  top domains:")
            for dom, n in s["top_domains"]:
                click.echo(f"    {dom:<35} {n}")

    if not total:
        click.echo("No memories yet. Run: engram-lite init")


@main.command("refresh-now")
@click.argument("memory_md_path", type=click.Path())
def refresh_now(memory_md_path: str) -> None:
    """Refresh the ## Now section of a MEMORY.md file from the DB.

    Called by shell hooks: engram-lite refresh-now ~/.engram/MEMORY.md
    """
    from amplifier_module_engram_lite.db import memory_md as mmd

    path = Path(memory_md_path).expanduser()
    if not path.exists():
        # Silently succeed — hook calls this before checking file existence
        sys.exit(0)

    # Determine scope and project_dir from path
    if path == Path.home() / ".engram" / "MEMORY.md":
        scope = "user"
        project_dir = None
        db_path = USER_DB
    else:
        scope = "project" if "local" not in path.name else "local"
        project_dir = path.parent.parent  # .engram/ -> project root
        db_path = project_dir / ".engram" / "engram.db"

    conn, _ = _get_conn(db_path)
    mmd.refresh_now(scope, conn=conn, project_dir=project_dir)
    # Silently exit — shell hook captures stdout for injection


@main.command("rebuild-index")
@click.option(
    "--scope",
    type=click.Choice(["user", "project", "local", "all"]),
    default="all",
    show_default=True,
)
def rebuild_index(scope: str) -> None:
    """Regenerate MEMORY.md from the DB. Use after manual DB edits or corruption."""
    from amplifier_module_engram_lite.db import memory_md as mmd
    from amplifier_module_engram_lite.db import memory_store as ms

    scopes = ["user", "project", "local"] if scope == "all" else [scope]

    for s in scopes:
        db_path = USER_DB if s == "user" else PROJECT_DB
        conn, _ = _get_conn(db_path)
        if not conn:
            click.echo(f"  skip {s} — no DB at {db_path}")
            continue

        # Re-initialize blank MEMORY.md
        path = mmd.initialize(s)
        mem_list = ms.get_all(conn, space=s if s != "local" else None, limit=200)
        if not mem_list:
            click.echo(f"  {s}: 0 memories")
            continue

        # Rewrite entries
        from amplifier_module_engram_lite.db.memory_md import ENTRY_TYPE_MAP

        count = 0
        for mem in mem_list:
            d = mem["data"]
            entry_type = ENTRY_TYPE_MAP.get(mem["content_type"], "fact")
            summary = d.get("summary", d.get("content", ""))[:100]
            section = "## You" if s == "user" else f"## Project: {Path.cwd().name}"
            mmd.append_entry(s, entry_type, summary, section=section)
            count += 1

        mmd.refresh_now(s, conn=conn)
        click.echo(f"  ✓ {s}: rebuilt {count} entries → {path}")
