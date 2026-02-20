"""Memory tracker hook module.

Validates that knowledge was captured to the memory system after each
orchestration cycle completes.
"""

# Amplifier module metadata
__amplifier_module_type__ = "hook"

import logging
import subprocess
from pathlib import Path
from typing import Any

from amplifier_core import HookResult, ModuleCoordinator

logger = logging.getLogger(__name__)


async def mount(coordinator: ModuleCoordinator, config: dict[str, Any] | None = None):
    """Mount the memory tracker hook.

    Args:
        coordinator: Module coordinator
        config: Optional configuration
            - priority: Hook priority (default: 90, runs late)
            - project_dir: Project directory to check (default: current working directory)

    Returns:
        Optional cleanup function
    """
    config = config or {}
    hook = MemoryTrackerHook(coordinator, config)
    hook.register(coordinator.hooks)
    logger.info("Mounted hooks-memory-tracker")
    return


class MemoryTrackerHook:
    """Hook that validates memory capture after orchestration completes.

    Checks git status for new/modified files in .canvas/memory/ paths and
    provides feedback to the agent if knowledge was captured.
    """

    def __init__(self, coordinator: ModuleCoordinator, config: dict[str, Any]):
        """Initialize memory tracker hook.

        Args:
            coordinator: Module coordinator
            config: Configuration dict
                - priority: Hook priority (default: 90)
                - project_dir: Project directory (default: cwd)
                - project_memory_base: Base directory for project memory (default: ".canvas/memory")
                - user_memory_base: Base directory for user memory (default: "~/.canvas/memory")
        """
        self.coordinator = coordinator
        self.priority = config.get("priority", 90)
        self.project_dir = Path(config.get("project_dir", Path.cwd()))
        self.project_memory_base = config.get("project_memory_base", ".canvas/memory")
        self.user_memory_base = Path(
            config.get("user_memory_base", "~/.canvas/memory")
        ).expanduser()

    def register(self, hooks):
        """Register hook on execution:end event."""
        hooks.register(
            "execution:end",
            self.on_execution_end,
            priority=self.priority,
            name="hooks-memory-tracker",
        )

    async def on_execution_end(self, event: str, data: dict[str, Any]) -> HookResult:
        """Check for memory captures after orchestration completes.

        Args:
            event: Event name ("execution:end")
            data: Event data

        Returns:
            HookResult with optional user message
        """
        logger.info("hooks-memory-tracker: Checking for memory captures")

        # Check git status for changes in .canvas/memory/ paths
        memory_changes = self._check_memory_changes()

        if memory_changes:
            logger.info(
                f"hooks-memory-tracker: Knowledge captured - {len(memory_changes)} files modified"
            )
            return HookResult(
                action="continue",
                user_message="💾 Knowledge captured to memory system",
                user_message_level="info",
            )
        else:
            logger.debug("hooks-memory-tracker: No memory captures detected")
            return HookResult(action="continue")

    def _check_memory_changes(self) -> list[str]:
        """Check git status for modified/new files in memory paths.

        Returns:
            List of modified files in project memory paths
        """
        try:
            # Run git status --short in project directory
            result = subprocess.run(
                ["git", "status", "--short"],
                cwd=self.project_dir,
                capture_output=True,
                text=True,
                timeout=5,
            )

            if result.returncode != 0:
                logger.warning(f"git status failed: {result.stderr}")
                return []

            # Parse output for project memory changes
            memory_files = []
            for line in result.stdout.splitlines():
                if self.project_memory_base in line:
                    memory_files.append(line.strip())

            return memory_files

        except subprocess.TimeoutExpired:
            logger.error("git status timed out")
            return []
        except Exception as e:
            logger.error(f"Error checking git status: {e}", exc_info=True)
            return []
