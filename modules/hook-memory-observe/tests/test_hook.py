"""Tests for hook-memory-observe module."""
from __future__ import annotations

import subprocess
from unittest.mock import MagicMock, patch

import pytest

from amplifier_module_hook_memory_observe import mount


@pytest.fixture
def coordinator():
    coord = MagicMock()
    coord.hooks = MagicMock()
    coord.hooks.register = MagicMock()
    return coord


def make_event(transcript_path=None):
    event = MagicMock()
    event.context = {"transcript_path": transcript_path} if transcript_path else {}
    return event


@pytest.mark.asyncio
async def test_mount_registers_hook(coordinator):
    await mount(coordinator, {})
    coordinator.hooks.register.assert_called_once()
    assert coordinator.hooks.register.call_args[0][0] == "execution:end"


@pytest.mark.asyncio
async def test_handler_skips_when_no_transcript_path(coordinator):
    await mount(coordinator, {})
    handler = coordinator.hooks.register.call_args[0][1]
    result = await handler(make_event(transcript_path=None))
    assert result is None


@pytest.mark.asyncio
async def test_handler_calls_engram_observe(coordinator):
    await mount(coordinator, {})
    handler = coordinator.hooks.register.call_args[0][1]

    with patch("subprocess.Popen") as mock_popen:
        with patch.dict("os.environ", {"ANTHROPIC_API_KEY": "test-key"}):
            await handler(make_event("/tmp/transcript.jsonl"))
        mock_popen.assert_called_once()
        cmd = mock_popen.call_args[0][0]
        assert "engram" in cmd
        assert "observe" in cmd
        assert "/tmp/transcript.jsonl" in cmd


@pytest.mark.asyncio
async def test_handler_tolerates_missing_engram(coordinator):
    await mount(coordinator, {})
    handler = coordinator.hooks.register.call_args[0][1]
    with patch("subprocess.Popen", side_effect=FileNotFoundError):
        result = await handler(make_event("/tmp/transcript.jsonl"))
    assert result is None
