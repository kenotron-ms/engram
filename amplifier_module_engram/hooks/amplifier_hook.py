"""Amplifier hook module for engram-lite — registers session lifecycle hooks."""

from __future__ import annotations

from typing import TYPE_CHECKING, Any

if TYPE_CHECKING:
    pass  # amplifier_core types only needed at runtime via try/except

__amplifier_module_type__ = "hook"


async def mount(coordinator: Any, config: dict[str, Any] | None = None) -> None:
    """Mount engram-lite hooks onto the Amplifier coordinator.

    Args:
        coordinator: Amplifier coordinator object exposing a ``hooks`` registry.
        config: Optional configuration dict. Supported keys:

            - ``priority`` (int, default 5): Hook priority ordering.
            - ``user_db`` (str, default ``~/.engram/engram.db``): User DB path.
            - ``project_db`` (str, default ``.engram/engram.db``): Project DB path.
    """
    config = config or {}
    hook = EngramLiteHook(config)
    hook.register(coordinator.hooks)


class EngramLiteHook:
    """Lifecycle hook handler for engram-lite memory injection."""

    def __init__(self, config: dict[str, Any]) -> None:
        self.priority: int = config.get("priority", 5)
        self.user_db: str = config.get("user_db", "~/.engram/engram.db")
        self.project_db: str = config.get("project_db", ".engram/engram.db")

    def register(self, hooks: Any) -> None:
        """Register lifecycle hooks with the provided hook registry."""
        hooks.register(
            "session:start",
            self.on_session_start,
            priority=self.priority,
            name="engram-lite",
        )
        hooks.register(
            "provider:request",
            self.on_capture_reminder,
            priority=self.priority,
            name="engram-lite-capture",
        )
        hooks.register(
            "provider:request",
            self.on_prompt_submit,
            priority=self.priority + 1,
            name="engram-lite",
        )

    async def on_session_start(self, event: str, data: dict[str, Any]) -> Any:
        """Inject MEMORY.md hot context at session start."""
        from amplifier_module_engram.hooks.context_builder import (
            build_session_context,
        )

        injection = build_session_context(user_db=self.user_db)
        return _hook_result(
            action="inject_context",
            context_injection=injection,
            ephemeral=False,
            suppress_output=True,
        )

    async def on_capture_reminder(self, event: str, data: dict[str, Any]) -> Any:
        """Inject capture reminder before each LLM call so the model processes
        pending captures from the previous turn before responding."""
        from amplifier_module_engram.hooks.context_builder import CAPTURE_REMINDER

        return _hook_result(
            action="inject_context",
            context_injection=CAPTURE_REMINDER,
            ephemeral=True,
            suppress_output=True,
        )

    async def on_prompt_submit(self, event: str, data: dict[str, Any]) -> Any:
        """Inject recall nudge before each LLM call."""
        from amplifier_module_engram.hooks.context_builder import RECALL_NUDGE

        return _hook_result(
            action="inject_context",
            context_injection=RECALL_NUDGE,
            ephemeral=True,
            suppress_output=True,
        )


def _hook_result(
    action: str,
    context_injection: str | None = None,
    ephemeral: bool = True,
    suppress_output: bool = True,
) -> Any:
    """Build a HookResult, with graceful fallback if amplifier_core not installed.

    Returns a ``HookResult`` when ``amplifier_core`` is available, otherwise a
    plain dict that mirrors the same structure for testing.
    """
    try:
        from amplifier_core import HookResult  # type: ignore[import-not-found]

        return HookResult(
            action=action,
            context_injection=context_injection,
            context_injection_role="system",
            ephemeral=ephemeral,
            suppress_output=suppress_output,
        )
    except ImportError:
        # Return a plain dict that mirrors HookResult for testing without amplifier_core
        return {
            "action": action,
            "context_injection": context_injection,
            "context_injection_role": "system",
            "ephemeral": ephemeral,
            "suppress_output": suppress_output,
        }
