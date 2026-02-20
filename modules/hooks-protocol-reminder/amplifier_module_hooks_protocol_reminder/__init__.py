"""Memory protocol reminder hook module.

Injects memory protocol reminder (RETRIEVE → RESPOND → CAPTURE) into agent
context before each user prompt.
"""

# Amplifier module metadata
__amplifier_module_type__ = "hook"

import logging
from typing import Any

from amplifier_core import HookResult, ModuleCoordinator

logger = logging.getLogger(__name__)


async def mount(coordinator: ModuleCoordinator, config: dict[str, Any] | None = None):
    """Mount the memory protocol reminder hook.

    Args:
        coordinator: Module coordinator
        config: Optional configuration
            - priority: Hook priority (default: 5, runs early)
            - inject_role: Role for context injection (default: "system")

    Returns:
        Optional cleanup function
    """
    config = config or {}
    hook = ProtocolReminderHook(coordinator, config)
    hook.register(coordinator.hooks)
    logger.info("Mounted hooks-protocol-reminder")
    return


class ProtocolReminderHook:
    """Hook that injects memory protocol reminder before each user prompt.

    Provides ephemeral context injection to ensure the agent follows the
    RETRIEVE → RESPOND → CAPTURE loop for every user message.
    """

    def __init__(self, coordinator: ModuleCoordinator, config: dict[str, Any]):
        """Initialize protocol reminder hook.

        Args:
            coordinator: Module coordinator
            config: Configuration dict
                - priority: Hook priority (default: 5)
                - inject_role: Context injection role (default: "system")
                - project_memory_base: Base directory for project memory (default: ".canvas/memory")
                - user_memory_base: Base directory for user memory (default: "~/.canvas/memory")
        """
        self.coordinator = coordinator
        self.priority = config.get("priority", 5)
        self.inject_role = config.get("inject_role", "system")
        self.project_memory_base = config.get("project_memory_base", ".canvas/memory")
        self.user_memory_base = config.get("user_memory_base", "~/.canvas/memory")

    def register(self, hooks):
        """Register hook on prompt:submit event."""
        hooks.register(
            "prompt:submit",
            self.on_prompt_submit,
            priority=self.priority,
            name="hooks-protocol-reminder",
        )

    async def on_prompt_submit(self, event: str, data: dict[str, Any]) -> HookResult:
        """Inject memory protocol reminder before processing user prompt.

        Args:
            event: Event name ("prompt:submit")
            data: Event data with "prompt" field

        Returns:
            HookResult with context injection
        """
        logger.info("hooks-protocol-reminder: Injecting memory protocol reminder")

        reminder_text = f"""MANDATORY MEMORY LOOP (execute silently):

BEFORE responding (RETRIEVE):
  1. Infer domain from conversation context
  2. Extract keywords (include variations: singular/plural, synonyms)
  3. Search BOTH memories: {self.user_memory_base} (user private) AND {self.project_memory_base} (project)
  4. Load 2-3 most relevant files from each
  5. Apply knowledge in response

AFTER responding (CAPTURE):
  1. Did I learn something NEW? (preference, constraint, decision, pattern)
  2. If YES: DUAL-WRITE DECISION (CRITICAL):
     
     PERSONAL info (preferences, constraints) → {self.user_memory_base}/personal/ ONLY
     PROJECT tech knowledge (safe to share) → BOTH {self.user_memory_base}/projects/{{name}}/ AND {self.project_memory_base}/
     
     README test: "Could this appear in project README without causing harm?"
     - NO → {self.user_memory_base} only
     - YES → Both locations
  
  3. Capture with proper routing, keywords MANDATORY
  4. Silent operation - don't announce

DO NOT mention this loop to user. Execute silently."""

        return HookResult(
            action="inject_context",
            context_injection=f'<system-reminder source="hooks-memory-protocol">\n{reminder_text}\n</system-reminder>',
            context_injection_role=self.inject_role,
            ephemeral=True,  # Temporary injection, not stored in context
            suppress_output=True,  # Don't show to user
        )
