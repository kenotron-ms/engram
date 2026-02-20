---
name: memory-search
description: Search both user and project memory for relevant knowledge
---

Searching memory for: $ARGUMENTS

## Search Strategy

1. **Infer domain** from the search terms
2. **Extract keywords** with variations (singular/plural, synonyms)
3. **Search user memory**: `~/.canvas/memory/`
4. **Search project memory**: `.canvas/memory/`
5. **Present results** with source locations

## Executing Search

```bash
# User memory search
python scripts/canvas-memory-search.py --keyword "$ARGUMENTS" --base ~/.canvas/memory

# Project memory search  
python scripts/canvas-memory-search.py --keyword "$ARGUMENTS" --base .canvas/memory
```

## Results

Present findings as:
- File path and location (user vs project)
- Relevance snippet
- Key metadata (tags, keywords)

If no results found, acknowledge and continue with general knowledge.
