#!/usr/bin/env python3
"""
Tests for canvas-memory-search.py
"""

import sys
from pathlib import Path
from tempfile import TemporaryDirectory

# Add parent directory to path to import the module
sys.path.insert(0, str(Path(__file__).parent.parent))

# Import by running the script as a module
import importlib.util

spec = importlib.util.spec_from_file_location(
    "canvas_memory_search",
    Path(__file__).parent.parent / "canvas-memory-search.py",
)
if spec and spec.loader:
    canvas_memory_search = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(canvas_memory_search)
    parse_frontmatter = canvas_memory_search.parse_frontmatter
    search_memory = canvas_memory_search.search_memory
else:
    raise ImportError("Failed to load canvas-memory-search.py")


class TestParseFrontmatter:
    """Test YAML frontmatter parsing."""

    def test_simple_frontmatter(self):
        """Test parsing simple key-value frontmatter."""
        content = """---
id: test-001
created: 2026-02-18
domain: projects/test
---

# Test Content
"""
        frontmatter, body = parse_frontmatter(content)

        assert frontmatter is not None
        assert frontmatter["id"] == "test-001"
        assert frontmatter["created"] == "2026-02-18"
        assert frontmatter["domain"] == "projects/test"
        assert "# Test Content" in body

    def test_inline_array(self):
        """Test parsing inline array syntax."""
        content = """---
keywords: [assigned, tasks, work, priorities]
---

Content
"""
        frontmatter, body = parse_frontmatter(content)

        assert frontmatter is not None
        assert "keywords" in frontmatter
        assert frontmatter["keywords"] == ["assigned", "tasks", "work", "priorities"]

    def test_multiline_array(self):
        """Test parsing multi-line array syntax."""
        content = """---
keywords: [
  assigned, tasks,
  work, priorities,
  todo
]
---

Content
"""
        frontmatter, body = parse_frontmatter(content)

        assert frontmatter is not None
        assert "keywords" in frontmatter
        assert frontmatter["keywords"] == [
            "assigned",
            "tasks",
            "work",
            "priorities",
            "todo",
        ]

    def test_list_syntax(self):
        """Test parsing list syntax with dashes."""
        content = """---
tags:
  - epic
  - memory-system
  - architecture
---

Content
"""
        frontmatter, body = parse_frontmatter(content)

        assert frontmatter is not None
        assert "tags" in frontmatter
        assert frontmatter["tags"] == ["epic", "memory-system", "architecture"]

    def test_quoted_values(self):
        """Test parsing quoted values."""
        content = """---
keywords: ["bottom line", "executive audience", concise]
---

Content
"""
        frontmatter, body = parse_frontmatter(content)

        assert frontmatter is not None
        assert "keywords" in frontmatter
        assert "bottom line" in frontmatter["keywords"]
        assert "executive audience" in frontmatter["keywords"]
        assert "concise" in frontmatter["keywords"]

    def test_no_frontmatter(self):
        """Test content without frontmatter."""
        content = """# Regular Markdown

No frontmatter here.
"""
        frontmatter, body = parse_frontmatter(content)

        assert frontmatter is None
        assert body == content

    def test_mixed_frontmatter(self):
        """Test frontmatter with mixed types."""
        content = """---
id: info-2026-02-18-001
created: 2026-02-18T13:00:00Z
domain: personal/preferences
scope: user
tags: [presentation, structure]
keywords: [
  presentation, presentations,
  "bottom line", conclusion,
  executive, audience
]
dimensions:
  confidence: 0.95
  importance: high
---

# Content
"""
        frontmatter, body = parse_frontmatter(content)

        assert frontmatter is not None
        assert frontmatter["id"] == "info-2026-02-18-001"
        assert frontmatter["domain"] == "personal/preferences"
        assert "presentation" in frontmatter["tags"]
        assert "structure" in frontmatter["tags"]
        assert "bottom line" in frontmatter["keywords"]
        assert "presentations" in frontmatter["keywords"]


class TestSearchMemory:
    """Test memory search functionality."""

    def test_search_with_keyword_match(self):
        """Test searching for keywords that match."""
        with TemporaryDirectory() as tmpdir:
            memory_path = Path(tmpdir) / ".canvas" / "memory"
            info_path = memory_path / "information" / "projects"
            info_path.mkdir(parents=True)

            # Create test file
            test_file = info_path / "test.md"
            test_file.write_text("""---
keywords: [assigned, tasks, work]
tags: [epic, memory-system]
---

# Test Memory Item
""")

            # Search for keyword
            results = search_memory(keywords=["assigned"], memory_path=memory_path)

            assert len(results) == 1
            assert "assigned" in results[0]["matched_values"]
            assert results[0]["matched_field"] == "keywords"

    def test_search_with_tag_match(self):
        """Test searching for tags."""
        with TemporaryDirectory() as tmpdir:
            memory_path = Path(tmpdir) / ".canvas" / "memory"
            info_path = memory_path / "information"
            info_path.mkdir(parents=True)

            test_file = info_path / "test.md"
            test_file.write_text("""---
keywords: [memory, search]
tags: [epic, phase-1]
---

# Test
""")

            results = search_memory(tags=["epic"], memory_path=memory_path)

            assert len(results) == 1
            assert "epic" in results[0]["matched_values"]

    def test_search_with_domain_scope(self):
        """Test domain-scoped search."""
        with TemporaryDirectory() as tmpdir:
            memory_path = Path(tmpdir) / ".canvas" / "memory" / "information"

            # Create files in different domains
            projects_path = memory_path / "projects"
            projects_path.mkdir(parents=True)
            (projects_path / "project.md").write_text("""---
keywords: [assigned]
---
Project item
""")

            personal_path = memory_path / "personal"
            personal_path.mkdir(parents=True)
            (personal_path / "personal.md").write_text("""---
keywords: [assigned]
---
Personal item
""")

            # Search only in projects domain
            results = search_memory(
                keywords=["assigned"],
                domain="information/projects",
                memory_path=memory_path.parent,
            )

            assert len(results) == 1
            assert "projects" in results[0]["file"]

    def test_search_no_matches(self):
        """Test search with no matches."""
        with TemporaryDirectory() as tmpdir:
            memory_path = Path(tmpdir) / ".canvas" / "memory"
            info_path = memory_path / "information"
            info_path.mkdir(parents=True)

            test_file = info_path / "test.md"
            test_file.write_text("""---
keywords: [other, terms]
---
Content
""")

            results = search_memory(keywords=["nonexistent"], memory_path=memory_path)

            assert len(results) == 0

    def test_search_multiple_keywords_or_logic(self):
        """Test OR logic with multiple keywords."""
        with TemporaryDirectory() as tmpdir:
            memory_path = Path(tmpdir) / ".canvas" / "memory"
            info_path = memory_path / "information"
            info_path.mkdir(parents=True)

            # File 1 has 'assigned'
            file1 = info_path / "file1.md"
            file1.write_text("""---
keywords: [assigned, work]
---
Content
""")

            # File 2 has 'tasks'
            file2 = info_path / "file2.md"
            file2.write_text("""---
keywords: [tasks, todo]
---
Content
""")

            # Search for either keyword
            results = search_memory(
                keywords=["assigned", "tasks"], memory_path=memory_path
            )

            assert len(results) == 2

    def test_search_case_insensitive(self):
        """Test case-insensitive search."""
        with TemporaryDirectory() as tmpdir:
            memory_path = Path(tmpdir) / ".canvas" / "memory"
            info_path = memory_path / "information"
            info_path.mkdir(parents=True)

            test_file = info_path / "test.md"
            test_file.write_text("""---
keywords: [Assigned, Tasks, WORK]
---
Content
""")

            results = search_memory(keywords=["assigned"], memory_path=memory_path)

            assert len(results) == 1

    def test_search_without_memory_directory(self):
        """Test graceful handling when memory directory doesn't exist."""
        with TemporaryDirectory() as tmpdir:
            memory_path = Path(tmpdir) / "nonexistent"

            results = search_memory(keywords=["test"], memory_path=memory_path)

            assert len(results) == 0
