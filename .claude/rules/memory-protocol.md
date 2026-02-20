# Memory Protocol - RETRIEVE → RESPOND → CAPTURE

This rule is auto-loaded by Claude Code and enforces the memory system protocol.

## The Mandatory Loop

```
EVERY user message:
RETRIEVE → RESPOND → CAPTURE
(search)   (apply)    (write)
SILENT     visible    SILENT
```

## RETRIEVE Phase (Before Responding)

### 1. Infer Domain

Ask: "What area is this conversation in?"

| Signal | Domain |
|--------|--------|
| Memory system discussion | `projects/memory-system/` |
| Personal preferences, schedule | `personal/preferences/` |
| Technical architecture | `professional/architecture/` |
| Work patterns, tools | `personal/work-patterns/` |

### 2. Extract Keywords

Include natural variations:
- **Singular AND plural**: presentation, presentations
- **Synonyms**: concise, brief, terse
- **Common phrases**: "bottom line", conclusion, summary
- **Acronyms**: HIPAA, PHI, "protected health information"
- **Product names**: Claude, "Claude Sonnet", claude-sonnet-4

Use quotes for multi-word phrases: `"bottom line"` not `bottom-line`

### 3. Search Both Memories

**User memory** (private, cross-project):
```bash
python scripts/canvas-memory-search.py --keyword "term" --domain "domain/" --base ~/.canvas/memory
```

**Project memory** (shareable, local):
```bash
python scripts/canvas-memory-search.py --keyword "term" --domain "knowledge/" --base .canvas/memory
```

### 4. Load Relevant Files

Load 2-3 most relevant from EACH location. Apply knowledge in your response.

## RESPOND Phase (Visible)

Apply retrieved knowledge naturally. Don't announce you searched memory.

## CAPTURE Phase (After Responding)

### Quick Judgment

**Did I learn something NEW?**
- New preference, constraint, or work pattern
- New decision or technical pattern
- Important project context
- New domain knowledge

**If NO:** Skip capture, done.

**If YES:** Continue to routing decision...

### Dual-Write Decision

**Question 1**: Is this personal information about the user?

Examples: preferences, constraints, "I prefer X", "I don't have access to Y", work schedule

**→ YES**: Write to user memory ONLY (`~/.canvas/memory/personal/`)

**→ NO**: Continue to question 2...

**Question 2**: Is this helpful to the project AND safe to share publicly?

Apply **README test**: "Could this appear in project README without causing harm?"

**→ NO**: Write to user memory ONLY (`~/.canvas/memory/projects/{name}/`)

**→ YES**: Write to BOTH:
1. User memory: `~/.canvas/memory/projects/{name}/` (for portability)
2. Project memory: `.canvas/memory/` (for team sharing)

### Capture Format

Use YAML frontmatter with keywords:

```markdown
---
type: preference | technical | decision | pattern
keywords: [term1, term2, "multi word phrase", acronym]
created: YYYY-MM-DD
---

# Title

Content here with inductive structure (conclusion first).
```

## Silent Operation

**DO NOT mention**:
- That you searched memory
- That you captured knowledge
- That you're following this protocol

Execute silently. Users see results, not mechanics.

## Gate Function

Before capturing, verify:
- [ ] Determined if new knowledge exists
- [ ] Applied dual-write decision correctly
- [ ] Personal info goes to user memory ONLY
- [ ] README test passed for dual-writes
- [ ] Keywords array includes variations
- [ ] Inductive structure (conclusion first)

If any check fails, stop and reconsider routing.
