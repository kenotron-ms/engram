"""memory_capture_hot — LLM-assisted hot surface writer for engram-lite."""

from __future__ import annotations

from pathlib import Path

from amplifier_module_engram_lite.db import memory_md as mmd

_MERGE_SYSTEM = (
    "You maintain a MEMORY.md hot surface — a short flowing narrative about a user "
    "and their active projects. Write in prose, not bullet points or key-value entries. "
    "Two zones: Zone 1 is the narrative (~20 lines max). Zone 2 after --- is the depth "
    "map (topic labels only, ~5 lines, one hint line for memory_recall). "
    "No frontmatter. No rigid section headers. No bullet lists in Zone 1."
)

_MERGE_PROMPT = """\
Update the MEMORY.md narrative to incorporate the new information below.
Rewrite as needed — this is a living document, not an append-only log.
Keep Zone 1 as flowing prose: weave together who the person is, what they're
working on, and the constraints that matter. Zone 2 (after ---) is the depth
map: topic labels only, one hint line for memory_recall.

Current MEMORY.md:
{current}

New information to incorporate:
{new_info}

Write the complete updated MEMORY.md. No preamble — just the content."""

_NEW_PROMPT = """\
Create a MEMORY.md narrative from the information below.
Zone 1: flowing prose (~20 lines max) about who the person is and what they're working on.
Zone 2: after ---, topic labels only, one hint line for memory_recall.
No frontmatter, no rigid section headers, no bullet lists in Zone 1.

Information:
{new_info}

Write the MEMORY.md. No preamble — just the content."""


async def _call_provider(coordinator: object, prompt: str) -> str | None:
    """Call the coordinator's first available provider. Returns text or None on failure."""
    try:
        providers: dict = getattr(coordinator, "mount_points", {}).get("providers", {})
        if not providers:
            return None
        provider = next(iter(providers.values()))

        from amplifier_core.message_models import ChatRequest, Message  # type: ignore[import-not-found]

        request = ChatRequest(
            messages=[Message(role="user", content=prompt)],
        )
        response = await provider.complete(request)
        text_blocks = [b for b in response.content if getattr(b, "type", None) == "text"]
        return text_blocks[0].text.strip() if text_blocks else None
    except Exception:
        return None


async def memory_capture_hot(
    new_info: str,
    *,
    scope: str = "user",
    coordinator: object | None = None,
    project_dir: Path | None = None,
) -> dict:
    """
    Read current MEMORY.md, merge new_info using LLM, write back as prose narrative.

    Uses the coordinator's first available provider to produce a merged narrative.
    Falls back to appending raw text if no provider is available.

    Returns:
        {written, scope, path, chars, llm_assisted}
    """
    if scope not in ("user", "project", "local"):
        scope = "user"

    current = mmd.read(scope, project_dir)

    prompt = (
        _MERGE_PROMPT.format(current=current.strip(), new_info=new_info.strip())
        if current
        else _NEW_PROMPT.format(new_info=new_info.strip())
    )

    merged: str | None = None
    llm_assisted = False

    if coordinator is not None:
        merged = await _call_provider(coordinator, prompt)
        if merged:
            llm_assisted = True

    if not merged:
        # Fallback: append the raw info to whatever exists
        merged = (
            current.rstrip() + f"\n\n{new_info.strip()}\n"
            if current
            else new_info.strip() + "\n"
        )

    path = mmd.get_path(scope, project_dir)
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(merged)

    return {
        "written": True,
        "scope": scope,
        "path": str(path),
        "chars": len(merged),
        "llm_assisted": llm_assisted,
    }
