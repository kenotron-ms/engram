"""Tests for hook-memory-context module."""
from __future__ import annotations

import subprocess
from unittest.mock import MagicMock, patch

import pytest

from amplifier_module_hook_memory_context import mount


@pytest.fixture
def coordinator():
    """Mock Amplifier coordinator."""
    coord = MagicMock()
    coord.hooks = MagicMock()
    coord.hooks.register = MagicMock()
    return coord


@pytest.mark.asyncio
async def test_mount_registers_hook(coordinator):
    """mount() registers exactly one hook on prompt:submit."""
    await mount(coordinator, {})
    coordinator.hooks.register.assert_called_once()
    args = coordinator.hooks.register.call_args
    assert args[0][0] == "prompt:submit"


@pytest.mark.asyncio
async def test_handler_calls_engram_load(coordinator):
    """Handler subprocess-calls engram awareness."""
    await mount(coordinator, {})
    handler = coordinator.hooks.register.call_args[0][1]

    with patch("subprocess.run") as mock_run:
        mock_run.return_value = MagicMock(returncode=0, stdout="<engram-context>\n## Personal\nDomains: Work (89)\n</engram-context>")
        await handler(MagicMock())
        mock_run.assert_called_once_with(
            ["engram", "awareness"],
            capture_output=True,
            text=True,
            timeout=10,
        )


@pytest.mark.asyncio
async def test_handler_tolerates_missing_engram_binary(coordinator):
    """Handler returns None gracefully when engram binary is not installed."""
    await mount(coordinator, {})
    handler = coordinator.hooks.register.call_args[0][1]

    with patch("subprocess.run", side_effect=FileNotFoundError):
        result = await handler(MagicMock())
    assert result is None


@pytest.mark.asyncio
async def test_handler_tolerates_timeout(coordinator):
    """Handler returns None gracefully when engram times out."""
    await mount(coordinator, {})
    handler = coordinator.hooks.register.call_args[0][1]

    with patch("subprocess.run", side_effect=subprocess.TimeoutExpired(cmd="engram", timeout=5)):
        result = await handler(MagicMock())
    assert result is None
