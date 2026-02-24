"""Memory search utilities for tool-memory-search.

Hybrid grep + YAML-frontmatter search, adapted from scripts/canvas-memory-search.py.
"""

import re
import subprocess
from pathlib import Path
from typing import Any

# Common words that add noise to keyword searches
_STOP_WORDS = {
    "a",
    "an",
    "the",
    "is",
    "are",
    "was",
    "were",
    "be",
    "been",
    "being",
    "have",
    "has",
    "had",
    "do",
    "does",
    "did",
    "will",
    "would",
    "could",
    "should",
    "may",
    "might",
    "can",
    "to",
    "of",
    "in",
    "for",
    "on",
    "with",
    "at",
    "by",
    "from",
    "up",
    "about",
    "into",
    "then",
    "when",
    "where",
    "how",
    "all",
    "some",
    "no",
    "not",
    "just",
    "i",
    "me",
    "my",
    "we",
    "our",
    "you",
    "your",
    "he",
    "she",
    "it",
    "its",
    "they",
    "them",
    "their",
    "this",
    "that",
    "these",
    "those",
    "what",
    "which",
    "who",
    "and",
    "but",
    "if",
    "or",
    "as",
    "let",
    "get",
    "use",
    "also",
    "new",
}


def extract_keywords(text: str, max_keywords: int = 6) -> list[str]:
    """Extract meaningful keywords from a user message for memory search."""
    text = text.lower()
    text = re.sub(r"[^\w\s-]", " ", text)
    words = text.split()
    keywords = [w for w in words if w not in _STOP_WORDS and len(w) > 2]
    seen: set[str] = set()
    unique = []
    for w in keywords:
        if w not in seen:
            seen.add(w)
            unique.append(w)
    return unique[:max_keywords]


def _parse_frontmatter(content: str) -> tuple[dict[str, Any] | None, str]:
    """Parse YAML frontmatter from markdown content."""
    if not content.startswith("---"):
        return None, content

    lines = content.split("\n")
    if len(lines) < 3:
        return None, content

    closing_index = -1
    for i in range(1, len(lines)):
        if lines[i].strip() == "---":
            closing_index = i
            break

    if closing_index == -1:
        return None, content

    yaml_lines = lines[1:closing_index]
    body = "\n".join(lines[closing_index + 1 :])

    frontmatter: dict[str, Any] = {}
    current_key = None
    current_list: list[str] = []
    in_list = False

    for line in yaml_lines:
        line = line.rstrip()
        if ":" in line and not line.strip().startswith("-"):
            if in_list and current_key:
                frontmatter[current_key] = current_list
                current_list = []
                in_list = False
            key, value = line.split(":", 1)
            key = key.strip()
            value = value.strip()
            if value == "[" or value.startswith("["):
                in_list = True
                current_key = key
                if value.startswith("["):
                    inline = value[1:].strip()
                    if inline and inline != "]":
                        items = [
                            item.strip().strip(",").strip('"').strip("'")
                            for item in inline.split(",")
                            if item.strip()
                        ]
                        current_list.extend(items)
            else:
                frontmatter[key] = value.strip('"').strip("'")
        elif line.strip().startswith("-"):
            item = line.strip()[1:].strip().strip(",").strip('"').strip("'")
            if item:
                current_list.append(item)
        elif in_list and line.strip() and line.strip() != "]":
            items = [
                item.strip().strip(",").strip('"').strip("'")
                for item in line.split(",")
                if item.strip() and item.strip() != "]"
            ]
            current_list.extend(items)
        elif line.strip() == "]" or line.strip().endswith("]"):
            if in_list and current_key:
                frontmatter[current_key] = current_list
                current_list = []
                in_list = False
                current_key = None

    if in_list and current_key:
        frontmatter[current_key] = current_list

    return frontmatter, body


def _grep_candidates(search_terms: list[str], base_path: Path) -> list[Path]:
    """Grep for candidate files containing any search term."""
    candidates: set[Path] = set()
    info_path = base_path / "information"
    if not info_path.exists():
        return []
    for term in search_terms:
        try:
            result = subprocess.run(
                ["grep", "-rl", "--include=*.md", term, str(info_path)],
                capture_output=True,
                text=True,
                timeout=5,
            )
            if result.returncode == 0:
                for line in result.stdout.strip().split("\n"):
                    if line:
                        candidates.add(Path(line))
        except Exception:
            pass
    return sorted(candidates)


def search_memory(
    keywords: list[str],
    memory_path: Path,
) -> list[dict[str, Any]]:
    """Search memory using hybrid grep + YAML frontmatter parse.

    Args:
        keywords: Keywords to search for (OR logic).
        memory_path: Resolved base memory path (no ~ expansion needed).

    Returns:
        List of dicts with 'file' and 'frontmatter' keys, ordered by match.
    """
    if not memory_path.exists() or not keywords:
        return []

    candidates = _grep_candidates(keywords, memory_path)
    results = []

    for file_path in candidates:
        try:
            content = file_path.read_text(encoding="utf-8")
            frontmatter, _ = _parse_frontmatter(content)
            if frontmatter is None:
                continue

            fm_keywords = frontmatter.get("keywords", [])
            if isinstance(fm_keywords, str):
                fm_keywords = [fm_keywords]

            for kw in keywords:
                if any(kw.lower() in fk.lower() for fk in fm_keywords):
                    results.append({"file": str(file_path), "frontmatter": frontmatter})
                    break
        except Exception:
            pass

    return results
