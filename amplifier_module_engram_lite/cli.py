"""engram-lite CLI — init, status."""

from __future__ import annotations

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
