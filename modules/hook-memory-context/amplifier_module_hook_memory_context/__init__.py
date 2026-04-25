"""Hook module: injects engram memory context at session start via prompt:submit."""
from __future__ import annotations

import subprocess


async def mount(coordinator, config: dict):
    """Register the memory context hook with the coordinator."""
    try:
        from amplifier_core import HookResult
    except ImportError:
        HookResult = None

    async def handle_prompt_submit(event):
        """Call engram load --format=context and inject result as system reminder."""
        try:
            result = subprocess.run(
                ["engram", "load", "--format=context"],
                capture_output=True,
                text=True,
                timeout=config.get("timeout", 5),
            )
            if result.returncode == 0 and result.stdout.strip():
                if HookResult is not None:
                    return HookResult(
                        action="inject_context",
                        content=result.stdout.strip(),
                        ephemeral=True,
                        suppress_output=True,
                    )
        except (subprocess.TimeoutExpired, FileNotFoundError):
            pass
        if HookResult is not None:
            return HookResult(action="noop")

    coordinator.hooks.register(
        "prompt:submit",
        handle_prompt_submit,
        priority=config.get("priority", 5),
    )
