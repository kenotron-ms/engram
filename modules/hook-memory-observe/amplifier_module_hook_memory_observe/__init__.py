"""Hook module: triggers engram observe at session end to extract memories."""
from __future__ import annotations

import os
import subprocess


async def mount(coordinator, config: dict):
    """Register the memory observe hook with the coordinator."""
    try:
        from amplifier_core import HookResult
    except ImportError:
        HookResult = None

    async def handle_execution_end(event):
        """Call engram observe <transcript_path> in background after session ends."""
        transcript_path = None
        if hasattr(event, "context") and isinstance(event.context, dict):
            transcript_path = event.context.get("transcript_path")

        if not transcript_path:
            if HookResult is not None:
                return HookResult(action="noop")
            return None

        api_key = os.environ.get("ANTHROPIC_API_KEY", "")
        cmd = ["engram", "observe", transcript_path]
        if api_key:
            cmd.extend(["--api-key", api_key])

        try:
            subprocess.Popen(
                cmd,
                stdout=subprocess.DEVNULL,
                stderr=subprocess.DEVNULL,
            )
        except FileNotFoundError:
            pass

        if HookResult is not None:
            return HookResult(action="noop")
        return None

    coordinator.hooks.register(
        "execution:end",
        handle_execution_end,
        priority=config.get("priority", 90),
    )
