#!/usr/bin/env python3
"""
canvas-memory-search.py
YAML-aware memory search using hybrid grep + selective parse

Usage:
  canvas-memory-search.py --keyword "assigned" --domain "projects/"
  canvas-memory-search.py --keyword "task,work" --tag "epic"
  canvas-memory-search.py --keyword "assigned" --format json
"""

import argparse
import json
import subprocess
import sys
from pathlib import Path
from typing import Any, Dict, List, Optional


def parse_frontmatter(content: str) -> tuple[Optional[Dict[str, Any]], str]:
    """
    Parse YAML frontmatter from markdown content.

    Returns:
        (frontmatter_dict, body_content) or (None, full_content) if no frontmatter
    """
    if not content.startswith("---"):
        return None, content

    # Find the closing --- (on its own line)
    lines = content.split("\n")
    if len(lines) < 3:  # Need at least ---, content, ---
        return None, content

    closing_index = -1
    for i in range(1, len(lines)):
        if lines[i].strip() == "---":
            closing_index = i
            break

    if closing_index == -1:
        return None, content

    yaml_lines = lines[1:closing_index]
    body_lines = lines[closing_index + 1 :]
    body = "\n".join(body_lines)

    # Simple YAML parser (handles keywords and tags arrays)
    frontmatter = {}
    current_key = None
    current_list = []
    in_list = False

    for line in yaml_lines:
        line = line.rstrip()

        # Key-value pair
        if ":" in line and not line.strip().startswith("-"):
            if in_list and current_key:
                frontmatter[current_key] = current_list
                current_list = []
                in_list = False

            key, value = line.split(":", 1)
            key = key.strip()
            value = value.strip()

            # Check if this starts a list
            if value == "[" or value.startswith("["):
                in_list = True
                current_key = key
                # Handle inline array start: keywords: [term1,
                if value.startswith("["):
                    inline = value[1:].strip()
                    if inline and inline != "]":
                        # Remove trailing comma and quotes
                        items = [
                            item.strip().strip(",").strip('"').strip("'")
                            for item in inline.split(",")
                            if item.strip()
                        ]
                        current_list.extend(items)
            else:
                frontmatter[key] = value.strip('"').strip("'")

        # List item
        elif line.strip().startswith("-"):
            item = line.strip()[1:].strip().strip(",").strip('"').strip("'")
            if item:
                current_list.append(item)

        # List continuation (multi-line array)
        elif in_list and line.strip() and not line.strip() == "]":
            items = [
                item.strip().strip(",").strip('"').strip("'")
                for item in line.split(",")
                if item.strip() and item.strip() != "]"
            ]
            current_list.extend(items)

        # End of list
        elif line.strip() == "]" or line.strip().endswith("]"):
            if in_list and current_key:
                frontmatter[current_key] = current_list
                current_list = []
                in_list = False
                current_key = None

    # Close any remaining list
    if in_list and current_key:
        frontmatter[current_key] = current_list

    return frontmatter, body


def grep_candidates(
    search_terms: List[str], base_path: Path, include_archive: bool = False
) -> List[Path]:
    """
    Use grep to quickly find candidate files that contain any search term.

    Returns list of file paths.
    """
    candidates = set()

    # Determine search paths
    search_paths = []
    if base_path.exists():
        info_path = base_path / "information"
        if info_path.exists():
            search_paths.append(info_path)

        if include_archive:
            archive_path = base_path / "archive"
            if archive_path.exists():
                search_paths.append(archive_path)

    if not search_paths:
        return []

    # Run grep for each term
    for term in search_terms:
        for search_path in search_paths:
            try:
                result = subprocess.run(
                    ["grep", "-rl", "--include=*.md", term, str(search_path)],
                    capture_output=True,
                    text=True,
                )
                if result.returncode == 0:
                    for line in result.stdout.strip().split("\n"):
                        if line:
                            candidates.add(Path(line))
            except Exception as e:
                print(f"Warning: grep failed for term '{term}': {e}", file=sys.stderr)

    return sorted(candidates)


def search_memory(
    keywords: Optional[List[str]] = None,
    tags: Optional[List[str]] = None,
    domain: Optional[str] = None,
    memory_path: Optional[Path] = None,
    include_archive: bool = False,
) -> List[Dict[str, Any]]:
    """
    Search memory using hybrid grep + YAML parse.

    Args:
        keywords: List of keywords to search for (OR logic)
        tags: List of tags to search for (OR logic)
        domain: Domain to scope search (e.g., "projects/memory-system/")
        memory_path: Base memory path (defaults to ~/.canvas/memory/)
        include_archive: Whether to search archive/ in addition to information/

    Returns:
        List of matching results with file, matched_field, matched_value, frontmatter
    """
    if memory_path is None:
        memory_path = Path.home() / ".canvas" / "memory"

    if not memory_path.exists():
        return []

    # Apply domain filter to path
    if domain:
        memory_path = memory_path / domain
        if not memory_path.exists():
            return []

    # Collect all search terms for grep
    search_terms = []
    if keywords:
        search_terms.extend(keywords)
    if tags:
        search_terms.extend(tags)

    if not search_terms:
        return []

    # Step 1: Grep to narrow candidates
    candidates = grep_candidates(search_terms, memory_path, include_archive)

    # Step 2: Parse YAML frontmatter of candidates only
    results = []
    for file_path in candidates:
        try:
            with open(file_path, "r", encoding="utf-8") as f:
                content = f.read()
                frontmatter, body = parse_frontmatter(content)

                if frontmatter is None:
                    continue

                # Step 3: Check if term matches in keywords or tags
                matched = False
                matched_field = None
                matched_values = []

                # Check keywords
                if keywords and "keywords" in frontmatter:
                    fm_keywords = frontmatter["keywords"]
                    if isinstance(fm_keywords, str):
                        fm_keywords = [fm_keywords]

                    for kw in keywords:
                        for fm_kw in fm_keywords:
                            if kw.lower() in fm_kw.lower():
                                matched = True
                                matched_field = "keywords"
                                matched_values.append(fm_kw)

                # Check tags
                if tags and "tags" in frontmatter:
                    fm_tags = frontmatter["tags"]
                    if isinstance(fm_tags, str):
                        fm_tags = [fm_tags]

                    for tag in tags:
                        for fm_tag in fm_tags:
                            if tag.lower() in fm_tag.lower():
                                matched = True
                                matched_field = (
                                    "tags"
                                    if not matched_field
                                    else f"{matched_field},tags"
                                )
                                matched_values.append(fm_tag)

                if matched:
                    results.append(
                        {
                            "file": str(file_path),
                            "matched_field": matched_field,
                            "matched_values": list(set(matched_values)),
                            "frontmatter": frontmatter,
                        }
                    )

        except Exception as e:
            print(f"Warning: Failed to parse {file_path}: {e}", file=sys.stderr)

    return results


def main():
    parser = argparse.ArgumentParser(
        description="Search Canvas memory with YAML-aware frontmatter parsing"
    )
    parser.add_argument(
        "--keyword", "-k", help="Keyword(s) to search (comma-separated for OR logic)"
    )
    parser.add_argument(
        "--tag", "-t", help="Tag(s) to search (comma-separated for OR logic)"
    )
    parser.add_argument(
        "--domain",
        "-d",
        help='Domain to scope search (e.g., "projects/memory-system/")',
    )
    parser.add_argument(
        "--memory-path",
        "-p",
        type=Path,
        help="Base memory path (defaults to ~/.canvas/memory/)",
    )
    parser.add_argument(
        "--include-archive",
        action="store_true",
        help="Search archive/ in addition to information/",
    )
    parser.add_argument(
        "--format", "-f", choices=["text", "json"], default="text", help="Output format"
    )

    args = parser.parse_args()

    # Parse keywords and tags
    keywords = [k.strip() for k in args.keyword.split(",")] if args.keyword else None
    tags = [t.strip() for t in args.tag.split(",")] if args.tag else None

    if not keywords and not tags:
        print("Error: Must specify at least one --keyword or --tag", file=sys.stderr)
        sys.exit(1)

    # Search
    results = search_memory(
        keywords=keywords,
        tags=tags,
        domain=args.domain,
        memory_path=args.memory_path,
        include_archive=args.include_archive,
    )

    # Output
    if args.format == "json":
        print(json.dumps(results, indent=2))
    else:
        if not results:
            print("No matches found.")
        else:
            print(f"Found {len(results)} matches:\n")
            for result in results:
                print(f"File: {result['file']}")
                print(
                    f"Matched: {result['matched_field']} = {', '.join(result['matched_values'])}"
                )
                if "domain" in result["frontmatter"]:
                    print(f"Domain: {result['frontmatter']['domain']}")
                if "tags" in result["frontmatter"]:
                    tags_list = result["frontmatter"]["tags"]
                    if isinstance(tags_list, list):
                        print(f"Tags: {', '.join(tags_list)}")
                    else:
                        print(f"Tags: {tags_list}")
                print()


if __name__ == "__main__":
    main()
