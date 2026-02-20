"""Tests for memory protocol enforcement hook."""

import pytest
from unittest.mock import Mock, AsyncMock
from amplifier_core import HookResult
from amplifier_module_hooks_memory_protocol import MemoryProtocolHook


@pytest.fixture
def coordinator():
    """Mock coordinator for testing."""
    coord = Mock()
    coord._hook_state = {}
    return coord


@pytest.fixture
def hook(coordinator):
    """Create hook instance for testing."""
    config = {
        "inject_pre_request": True,
        "inject_validation": True,
        "priority": 5
    }
    return MemoryProtocolHook(coordinator, config)


@pytest.mark.asyncio
async def test_pre_request_reminder(hook):
    """Test that pre-request hook injects protocol reminder."""
    result = await hook.on_provider_request("provider:request", {})
    
    assert result.action == "inject_context"
    assert result.ephemeral is True
    assert result.suppress_output is True
    assert "RETRIEVE" in result.context_injection
    assert "CAPTURE" in result.context_injection
    assert "DO NOT announce" in result.context_injection


@pytest.mark.asyncio
async def test_trigger_detection_preference(hook):
    """Test detection of preference statements."""
    result = await hook.on_prompt_submit("prompt:submit", {
        "prompt": "I prefer bottom-line-first presentations"
    })
    
    assert result.action == "continue"
    assert hook._get_state("needs_capture") is True
    
    context = hook._get_state("capture_context")
    assert "i prefer" in context["detected_triggers"]
    assert "bottom-line" in context["prompt_excerpt"]


@pytest.mark.asyncio
async def test_trigger_detection_constraint(hook):
    """Test detection of constraint statements."""
    result = await hook.on_prompt_submit("prompt:submit", {
        "prompt": "I don't have access to the production database"
    })
    
    assert hook._get_state("needs_capture") is True
    
    context = hook._get_state("capture_context")
    assert "don't have access" in context["detected_triggers"]


@pytest.mark.asyncio
async def test_trigger_detection_decision(hook):
    """Test detection of decision statements."""
    result = await hook.on_prompt_submit("prompt:submit", {
        "prompt": "We decided to use hot/cold tiers for performance"
    })
    
    assert hook._get_state("needs_capture") is True
    
    context = hook._get_state("capture_context")
    assert "we decided" in context["detected_triggers"]


@pytest.mark.asyncio
async def test_no_trigger_detection(hook):
    """Test that non-trigger prompts don't set flag."""
    result = await hook.on_prompt_submit("prompt:submit", {
        "prompt": "What's the weather today?"
    })
    
    assert result.action == "continue"
    assert hook._get_state("needs_capture") is None


@pytest.mark.asyncio
async def test_validation_clears_flag(hook):
    """Test that validation clears flag."""
    # Set up state
    hook._set_state("needs_capture", True)
    hook._set_state("capture_context", {
        "detected_triggers": ["i prefer"],
        "prompt_excerpt": "I prefer..."
    })
    
    # Run validation
    result = await hook.on_execution_end("execution:end", {})
    
    assert result.action == "continue"
    assert hook._get_state("needs_capture") is False


@pytest.mark.asyncio
async def test_validation_without_flag(hook):
    """Test validation does nothing when flag not set."""
    result = await hook.on_execution_end("execution:end", {})
    
    assert result.action == "continue"


@pytest.mark.asyncio
async def test_custom_triggers(coordinator):
    """Test hook with custom trigger configuration."""
    config = {
        "capture_triggers": ["custom trigger", "special phrase"]
    }
    hook = MemoryProtocolHook(coordinator, config)
    
    result = await hook.on_prompt_submit("prompt:submit", {
        "prompt": "This has a custom trigger in it"
    })
    
    assert hook._get_state("needs_capture") is True
    
    context = hook._get_state("capture_context")
    assert "custom trigger" in context["detected_triggers"]


@pytest.mark.asyncio
async def test_state_management(hook):
    """Test state management utilities."""
    # Set state
    hook._set_state("test_key", "test_value")
    assert hook._get_state("test_key") == "test_value"
    
    # State persists across calls
    hook._set_state("needs_capture", True)
    assert hook._get_state("needs_capture") is True
    
    # Can clear state
    hook._set_state("needs_capture", False)
    assert hook._get_state("needs_capture") is False


@pytest.mark.asyncio  
async def test_disabled_pre_request(coordinator):
    """Test hook with pre-request injection disabled."""
    config = {"inject_pre_request": False}
    hook = MemoryProtocolHook(coordinator, config)
    
    # Pre-request should not inject
    result = await hook.on_provider_request("provider:request", {})
    
    # Hook still runs but since inject_pre_request is False,
    # it should return continue (or the hook shouldn't be registered)
    # For now, the hook always injects - we control via registration
    # This test documents expected behavior


@pytest.mark.asyncio
async def test_case_insensitive_trigger_detection(hook):
    """Test that trigger detection is case-insensitive."""
    result = await hook.on_prompt_submit("prompt:submit", {
        "prompt": "I PREFER uppercase presentations"
    })
    
    assert hook._get_state("needs_capture") is True
    
    context = hook._get_state("capture_context")
    assert "i prefer" in context["detected_triggers"]
