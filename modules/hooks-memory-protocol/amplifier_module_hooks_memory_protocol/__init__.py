"""Memory protocol enforcement hook module.

Enforces the RETRIEVE → RESPOND → CAPTURE loop through:
1. Pre-request reminders (before each LLM call)
2. Trigger detection (when user provides new knowledge)
3. Post-execution validation (check if capture occurred)
"""

# Amplifier module metadata
__amplifier_module_type__ = "hook"

import logging
from typing import Any

from amplifier_core import HookResult, ModuleCoordinator

logger = logging.getLogger(__name__)

# Default capture triggers (signals that new knowledge needs capturing)
DEFAULT_TRIGGERS = [
    "i prefer",
    "remember that",
    "my style",
    "don't have access",
    "can't use",
    "constraint",
    "requirement",
    "we decided",
    "let's use",
    "chosen approach",
    "going forward",
    "always do",
    "never do",
]


async def mount(coordinator: ModuleCoordinator, config: dict[str, Any] | None = None):
    """Mount the memory protocol enforcement hook.
    
    Args:
        coordinator: Module coordinator
        config: Optional configuration
            - inject_pre_request: Inject reminder before each LLM call (default: True)
            - inject_validation: Inject reminder if capture missing (default: True)
            - capture_triggers: List of trigger phrases (default: DEFAULT_TRIGGERS)
            - memory_path_pattern: Path pattern that counts as capture (default: "/.canvas/memory/")
            - validation_turns: Turns to check for capture (default: 1)
            - priority: Hook priority (default: 5)
    
    Returns:
        Optional cleanup function
    """
    config = config or {}
    hook = MemoryProtocolHook(coordinator, config)
    hook.register(coordinator.hooks)
    logger.info("Mounted hooks-memory-protocol")
    return


class MemoryProtocolHook:
    """Hook that enforces RETRIEVE → RESPOND → CAPTURE memory protocols.
    
    Three-phase enforcement:
    1. Pre-request: Inject protocol reminder before each LLM call
    2. Input analysis: Detect when user provides new knowledge
    3. Post-execution: Validate capture occurred when needed
    """
    
    def __init__(self, coordinator: ModuleCoordinator, config: dict[str, Any]):
        """Initialize memory protocol hook.
        
        Args:
            coordinator: Module coordinator for state management
            config: Configuration dict
        """
        self.coordinator = coordinator
        
        # Configuration
        self.inject_pre_request = config.get("inject_pre_request", True)
        self.inject_validation = config.get("inject_validation", True)
        self.capture_triggers = config.get("capture_triggers", DEFAULT_TRIGGERS)
        self.memory_path_pattern = config.get("memory_path_pattern", "/.canvas/memory/")
        self.validation_turns = config.get("validation_turns", 1)
        self.priority = config.get("priority", 5)
        
        # State (stored in coordinator for persistence)
        self._state_key_prefix = "hooks_memory_protocol"
    
    def register(self, hooks):
        """Register hook handlers for lifecycle events."""
        if self.inject_pre_request:
            hooks.register(
                "provider:request",
                self.on_provider_request,
                priority=self.priority,
                name="hooks-memory-protocol-reminder"
            )
        
        hooks.register(
            "prompt:submit",
            self.on_prompt_submit,
            priority=self.priority,
            name="hooks-memory-protocol-detector"
        )
        
        if self.inject_validation:
            hooks.register(
                "execution:end",
                self.on_execution_end,
                priority=self.priority,
                name="hooks-memory-protocol-validator"
            )
    
    async def on_provider_request(self, event: str, data: dict[str, Any]) -> HookResult:
        """Inject protocol reminder before each LLM request.
        
        Args:
            event: Event name (provider:request)
            data: Event data
            
        Returns:
            HookResult with ephemeral context injection
        """
        reminder = """MANDATORY MEMORY LOOP (execute silently):

BEFORE responding (RETRIEVE):
  1. Infer domain from conversation context
  2. Extract keywords (include variations: singular/plural, synonyms, acronyms)
  3. Search memory: python scripts/canvas-memory-search.py --keyword "term" --domain "domain/"
  4. Load 2-3 most relevant files
  5. Apply knowledge in your response

AFTER responding (CAPTURE):
  1. Quick judgment: Did I learn something NEW?
     - User preference, constraint, decision, pattern (2nd+ occurrence)
  2. If YES: Capture immediately
     - Domain routing: projects/ professional/ personal/
     - Size routing: ≤500 words hot, >500 cold + hot ref
     - Keywords: natural variations MANDATORY
     - Inductive writing: conclusion first
     - Silent: don't announce
  3. If NO: Done

DO NOT announce this loop to user. Execute silently. User sees only your informed response."""
        
        return HookResult(
            action="inject_context",
            context_injection=f'<system-reminder source="hooks-memory-protocol">\n{reminder}\n</system-reminder>',
            context_injection_role="system",
            ephemeral=True,
            suppress_output=True
        )
    
    async def on_prompt_submit(self, event: str, data: dict[str, Any]) -> HookResult:
        """Analyze user prompt for capture triggers.
        
        Args:
            event: Event name (prompt:submit)
            data: Event data with "prompt" field
            
        Returns:
            HookResult(action="continue") - sets flag if triggers detected
        """
        prompt = data.get("prompt", "").lower()
        
        # Detect trigger phrases
        detected_triggers = [
            trigger for trigger in self.capture_triggers
            if trigger in prompt
        ]
        
        if detected_triggers:
            logger.info(f"hooks-memory-protocol: Detected capture triggers: {detected_triggers}")
            
            # Set flag for post-execution validation
            self._set_state("needs_capture", True)
            self._set_state("capture_context", {
                "prompt_excerpt": data.get("prompt", "")[:200],
                "detected_triggers": detected_triggers,
                "turn_detected": self._get_turn_count()
            })
        
        return HookResult(action="continue")
    
    async def on_execution_end(self, event: str, data: dict[str, Any]) -> HookResult:
        """Validate capture occurred if needed.
        
        Args:
            event: Event name (execution:end)
            data: Event data
            
        Returns:
            HookResult with reminder if capture missing, continue otherwise
        """
        # Check if capture was needed
        if not self._get_state("needs_capture"):
            return HookResult(action="continue")
        
        # Get capture context
        capture_context = self._get_state("capture_context") or {}
        
        # Check if write_file was called to memory paths
        # Note: This requires coordinator to expose recent tool calls
        # For now, we'll clear the flag and rely on pre-request reminder
        # TODO: Implement tool call tracking when coordinator API available
        
        # For initial version: just remind, don't validate
        # This makes the hook gentler and avoids false positives
        logger.info("hooks-memory-protocol: Capture was needed this turn (validation not yet implemented)")
        
        # Clear flag for next turn
        self._set_state("needs_capture", False)
        
        return HookResult(action="continue")
    
    def _get_turn_count(self) -> int:
        """Get current turn count from coordinator."""
        try:
            # Access coordinator's context manager to get turn count
            context_manager = getattr(self.coordinator, "context", None)
            if context_manager and hasattr(context_manager, "get_messages"):
                messages = context_manager.get_messages()
                # Count user messages (each user message = one turn)
                return len([m for m in messages if m.get("role") == "user"])
        except Exception:
            pass
        return 0
    
    def _set_state(self, key: str, value: Any):
        """Set state value in coordinator."""
        full_key = f"{self._state_key_prefix}_{key}"
        # Store in coordinator's shared state
        if not hasattr(self.coordinator, "_hook_state"):
            self.coordinator._hook_state = {}
        self.coordinator._hook_state[full_key] = value
    
    def _get_state(self, key: str) -> Any:
        """Get state value from coordinator."""
        full_key = f"{self._state_key_prefix}_{key}"
        if hasattr(self.coordinator, "_hook_state"):
            return self.coordinator._hook_state.get(full_key)
        return None
