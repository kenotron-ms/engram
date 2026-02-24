"""Memory protocol hook - automatic retrieval + capture reminder.

Fires on prompt:submit (priority 5):
  1. Extracts keywords from the user's message
  2. Searches both user and project memory
  3. Injects matching files as context (RETRIEVE is now automatic)
  4. Appends a CAPTURE reminder for the agent
"""

__amplifier_module_type__ = "hook"

import logging
from pathlib import Path
from typing import Any

from amplifier_core import HookResult, ModuleCoordinator  # type: ignore[import-untyped]

from ._search import extract_keywords, search_memory

logger = logging.getLogger(__name__)


async def mount(coordinator: ModuleCoordinator, config: dict[str, Any] | None = None):
    """Mount the memory protocol hook."""
    config = config or {}
    hook = ProtocolReminderHook(coordinator, config)
    hook.register(coordinator.hooks)
    logger.info("Mounted hooks-protocol-reminder")


class ProtocolReminderHook:
    """Hook that auto-retrieves relevant memory and injects a capture reminder."""

    def __init__(self, coordinator: ModuleCoordinator, config: dict[str, Any]):
        self.coordinator = coordinator
        self.priority = config.get("priority", 5)
        self.inject_role = config.get("inject_role", "system")
        self.project_memory_base = config.get("project_memory_base", ".canvas/memory")
        self.user_memory_base = config.get("user_memory_base", "~/.canvas/memory")
        self.max_results = config.get("max_results", 3)

    def register(self, hooks: Any) -> None:
        hooks.register(
            "prompt:submit",
            self.on_prompt_submit,
            priority=self.priority,
            name="hooks-protocol-reminder",
        )

    async def on_prompt_submit(self, event: str, data: dict[str, Any]) -> HookResult:
        """Auto-retrieve relevant memory, then inject capture reminder."""
        prompt = data.get("prompt", "")
        keywords = extract_keywords(prompt)

        retrieved = self._retrieve(keywords) if keywords else []

        parts: list[str] = []

        if retrieved:
            parts.append("<retrieved-memory>")
            for item in retrieved:
                parts.append(f"\n<!-- {item['file']} -->\n{item['content']}")
            parts.append("\n</retrieved-memory>")
            logger.info(
                "hooks-protocol-reminder: Injected %d memory file(s)", len(retrieved)
            )
        else:
            logger.info("hooks-protocol-reminder: No relevant memory found")

        parts.append(
            f'<system-reminder source="hooks-memory-protocol">\n'
            f"AFTER responding (CAPTURE — execute silently):\n"
            f"  1. Did I learn something NEW? (preference, constraint, decision, pattern)\n"
            f"  2. If YES — dual-write decision:\n"
            f"     Personal info → {self.user_memory_base} ONLY\n"
            f"     Project knowledge (safe to share) → BOTH "
            f"{self.user_memory_base}/projects/{{name}}/ AND {self.project_memory_base}/\n"
            f"  3. Keywords MANDATORY. Do not announce.\n"
            f"</system-reminder>"
        )

        return HookResult(
            action="inject_context",
            context_injection="\n".join(parts),
            context_injection_role=self.inject_role,
            ephemeral=True,
            suppress_output=True,
        )

    def _retrieve(self, keywords: list[str]) -> list[dict[str, str]]:
        """Search user and project memory, return top results with file content."""
        results = []

        user_path = Path(self.user_memory_base).expanduser()
        if user_path.exists():
            results.extend(search_memory(keywords, user_path))

        project_path = Path(self.project_memory_base)
        if project_path.exists():
            results.extend(search_memory(keywords, project_path))

        # Deduplicate, cap at max_results
        seen: set[str] = set()
        unique = []
        for r in results:
            if r["file"] not in seen:
                seen.add(r["file"])
                unique.append(r)
            if len(unique) >= self.max_results:
                break

        # Read file contents
        output = []
        for match in unique:
            try:
                content = Path(match["file"]).read_text(encoding="utf-8")
                output.append({"file": match["file"], "content": content})
            except Exception as e:
                logger.warning("Could not read %s: %s", match["file"], e)

        return output
