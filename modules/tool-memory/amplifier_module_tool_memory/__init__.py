"""Tool module: exposes memory_search, memory_load, memory_status via engram CLI."""
from __future__ import annotations

import subprocess


def _run_engram(args: list[str], timeout: int = 10) -> str:
    """Run an engram CLI command and return stdout or error message."""
    try:
        result = subprocess.run(
            ["engram", *args],
            capture_output=True,
            text=True,
            timeout=timeout,
        )
        if result.returncode == 0:
            return result.stdout
        return f"engram error: {result.stderr.strip()}"
    except FileNotFoundError:
        return "engram binary not found. Install engram first."
    except subprocess.TimeoutExpired:
        return f"engram timed out after {timeout}s"


async def mount(coordinator, config: dict):
    """Register memory tools with the coordinator."""

    async def memory_search(query: str, limit: int = 10) -> str:
        """Search personal memory vault semantically.

        Args:
            query: Natural language search query
            limit: Maximum number of results to return (default: 10)
        """
        return _run_engram(["search", query, "--limit", str(limit)], timeout=config.get("timeout", 10))

    async def memory_load(format: str = "context") -> str:  # noqa: A002
        """Load context from personal memory vault.

        Args:
            format: Output format -- 'context' (default), 'facts', or 'summary'
        """
        return _run_engram(["load", f"--format={format}"], timeout=config.get("timeout", 5))

    async def memory_status() -> str:
        """Get status of personal memory vault, search index, and sync backend."""
        return _run_engram(["status"], timeout=config.get("timeout", 5))

    coordinator.tools.register(memory_search)
    coordinator.tools.register(memory_load)
    coordinator.tools.register(memory_status)
