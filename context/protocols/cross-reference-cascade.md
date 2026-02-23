# Cross-Reference Cascade Protocol

> **Use when:** After capturing ANY knowledge. Every piece of knowledge touches more than one file.

```
NO KNOWLEDGE CAPTURED WITHOUT CASCADE CHECK
```

**Violating the letter of this protocol is violating the spirit of this protocol.**

---

## The Core Principle

**Every piece of knowledge touches more than one file. Find ALL of them.**

When you capture a preference, decision, or piece of knowledge, it doesn't exist in isolation. It connects to:
- Related knowledge items
- Active projects
- Established patterns
- Temporal knowledge that may become stale

**The user should NEVER have to say:** "Did you connect this to X?"

If the knowledge relates to something you already know, find the relationship and record it.

---

## The Cascade Check Table

After capturing any knowledge, scan for secondary effects:

| Check | What to look for | Action |
|-------|------------------|--------|
| **Related items** | Does this connect to existing knowledge in memory? | Add `relates-to` links, cross-reference in both items |
| **Projects** | Does this mention progress on active work? Does this change project context? | Update project memory with new context |
| **Patterns** | Is this the 2nd+ occurrence of something? | Mark as pattern, increase confidence, add to existing item |
| **Temporal** | Does this make older knowledge stale? Does this update previous decisions? | Mark old knowledge expired or superseded, update with current |

---

## Check 1: Related Items

**Question:** Does this new knowledge connect to existing knowledge?

### How to Find Related Items

1. **Extract topics from new knowledge**: What is this about?
2. **Search related domains**: Where would connected knowledge live?
3. **Load candidate items**: Read items that might be related
4. **Identify connections**: How does new knowledge relate to existing?

### Types of Relationships

| Relationship Type | Example | Action |
|-------------------|---------|--------|
| **Elaborates** | New knowledge adds detail to existing | Add reference in existing item to new item |
| **Contradicts** | New knowledge changes previous understanding | Mark old knowledge as superseded, link to new |
| **Complements** | New knowledge is related but distinct | Add `relates-to` links in both directions |
| **Depends on** | New knowledge requires understanding existing | Add "See also" reference to prerequisite |

### Recording Relationships

**In both items:**

```markdown
# Item A
relates-to: [item-b-id]

## Connections
- Related to [[Item B]]: How they connect
```

```markdown
# Item B
relates-to: [item-a-id]

## Connections
- Related to [[Item A]]: How they connect
```

---

## Check 2: Projects

**Question:** Does this knowledge affect any active project?

### Project Impact Signals

| Signal | Action |
|--------|--------|
| Mentions project name | Update project context |
| Describes project constraint | Add to project constraints |
| Explains project decision | Record in project rationale |
| Changes project approach | Update project strategy |

### Where to Update

**User memory:** `~/.canvas/memory/information/projects/{project-name}/`
- Personal notes about project
- Your observations and context
- Full details including sensitive information

**Project memory:** `./.canvas/memory/knowledge/`
- Only if public-safe (README test)
- Factual, technical content only
- No personal observations

**See:** `dual-write-decision.md` for complete decision tree.

---

## Check 3: Patterns (2nd+ Occurrence)

**Question:** Is this the 2nd+ time you've learned this?

### Pattern Recognition Signals

| Signal | What it means |
|--------|---------------|
| User corrects same thing again | This is a strong preference/pattern |
| User restates previous preference | Reinforcing existing knowledge |
| Similar feedback on different content | Pattern applies broadly |

### Pattern Actions

1. **Find the existing item** where 1st occurrence was captured
2. **Update confidence**: Increase from 0.70 → 0.85 → 0.95
3. **Add context**: "Stated multiple times" or "Corrected on 2026-02-18"
4. **Broaden scope**: If pattern applies more widely than originally captured
5. **Don't create duplicate**: Enrich existing item, don't create new one

### Example: Presentation Style

**1st occurrence:**
```markdown
# Presentation Style
User mentioned preference for bottom-line-first in email discussions.

dimensions:
  confidence: 0.70
```

**2nd occurrence (user corrects document):**
```markdown
# Presentation Style
User prefers inductive writing: conclusion first, supporting detail follows.
Stated explicitly for emails (2026-02-15) and corrected document format (2026-02-18).
Applies to ALL writing: documents, code comments, commit messages.

dimensions:
  confidence: 0.95
```

---

## Check 4: Temporal Updates

**Question:** Does this knowledge make older knowledge stale?

### Temporal Impact Types

| Type | Example | Action |
|------|---------|--------|
| **Supersedes** | New decision replaces old | Mark old as superseded, link to new |
| **Expires** | Knowledge has shelf life | Set `expires` date when capturing |
| **Updates** | Partial change to existing | Update existing item, note date modified |

### Handling Superseded Knowledge

**Old item:**
```markdown
---
dimensions:
  expires: 2026-02-18
  superseded-by: info-2026-02-18-002
---

# Old Approach

**SUPERSEDED:** This approach was replaced on 2026-02-18.
**See:** [[New Approach]]

[Original content preserved for history]
```

**New item:**
```markdown
---
dimensions:
  supersedes: info-2026-01-15-001
---

# New Approach

## What Changed
Previous approach used X. New approach uses Y because Z.

## History
- 2026-01-15: Original approach with X
- 2026-02-18: Switched to Y due to performance issues
```

### Expiration Dates

Some knowledge has shelf life. Set `expires` when capturing:

```yaml
dimensions:
  expires: 2025-Q1  # Pricing data
  expires: 2026-06-01  # Temporary constraint
  expires: null  # Timeless (preferences, patterns)
```

**When applying old knowledge:** If expired, verify before using.

---

## The Cascade Process

```
1. Capture new knowledge
   ↓
2. Run cascade check table
   ↓
3. For each finding:
   - Update target file
   - Add cross-references
   - Adjust confidence/dates
   ↓
4. Verify no further cascade effects
   ↓
5. Complete
```

**The cascade continues until there's nothing left to update.**

---

## Examples

### Example 1: Preference Creates Multiple Connections

**New capture:** User prefers concise communication

**Cascade finds:**
- Related to: `presentation-style.md` (both about communication)
- Related to: `executive-audience.md` (concise for executives)
- Related to: `writing-patterns.md` (general writing preference)

**Action:** Add `relates-to` links in all 4 items, cross-reference with explanations.

### Example 2: Decision Affects Project

**New capture:** Hot/cold architecture decision for memory system

**Cascade finds:**
- Project: `projects/memory-system/`
- Pattern: This is architectural decision pattern
- Temporal: Supersedes previous "flat structure" approach

**Action:**
- Update project memory (dual-write: user + project if public-safe)
- Link to existing architecture patterns
- Mark old "flat structure" as superseded

### Example 3: Pattern Recognition

**New capture:** User corrects "too verbose" again

**Cascade finds:**
- Pattern: 2nd occurrence of "prefer concise" feedback
- Related items: presentation-style, communication preferences

**Action:**
- Update existing preference item
- Increase confidence: 0.70 → 0.95
- Add note: "Corrected multiple times"
- Don't create duplicate

### Example 4: Temporal Update

**New capture:** Project deadline moved to March 2026

**Cascade finds:**
- Temporal: Previous deadline (February 2026) now stale
- Project: Affects project timeline

**Action:**
- Update project context with new deadline
- Mark old deadline as superseded
- Update any schedule-dependent items

---

## Gate Function

```
AFTER capturing knowledge:
  1. CHECK: Searched for related items?
  2. CHECK: Identified project impacts?
  3. CHECK: Recognized patterns (2nd+ occurrence)?
  4. CHECK: Checked for temporal updates?
  5. CHECK: Recorded ALL connections found?
  If ANY check fails: Cascade incomplete. Continue checking.
```

---

## Three-Failure Escalation

If the Gate Function fails 3 times in the same session (e.g., repeatedly missing related items, failing to recognize patterns, or ignoring temporal updates), STOP immediately.

1. State what you attempted and what failed each time
2. Ask the user for explicit guidance
3. Do not resume until the user provides direction

The cascade is critical for knowledge graph coherence. Repeated failures indicate the relationship-finding mechanism is broken. Escalate.

---

## Red Flags

If you catch yourself thinking:
- "This knowledge stands alone"
- "I don't think there are related items"
- "This doesn't affect any projects"
- "No need to check for patterns"
- "Temporal updates don't apply here"

**All of these mean: STOP. Re-read the Core Principle section above.**

---

## Common Rationalizations

| Excuse | Reality |
|--------|---------|
| "This knowledge is isolated" | No knowledge is truly isolated. Search for connections. |
| "I don't see any related items" | Did you search related domains? Load candidates? |
| "This doesn't affect projects" | Check anyway. Project impact is often subtle. |
| "This is the first time I learned this" | Check memory. User may have stated this before. |
| "Old knowledge is still valid" | New knowledge often updates or supersedes old. Check temporal. |
| "Cross-referencing will take too long" | Missing connections wastes more time when user has to repeat themselves. |

---

## Anti-Patterns

| Don't | Do |
|-------|-----|
| Capture knowledge and move on | Run cascade check table after EVERY capture |
| Assume knowledge is isolated | Search for connections in related domains |
| Create duplicate items for patterns | Recognize 2nd+ occurrence, enrich existing item |
| Leave old knowledge unmarked when superseded | Mark old as superseded, link to new |
| Skip project impact check | Always check if knowledge affects active projects |
| Miss cross-domain connections | Search professional/, personal/, projects/ domains |

---

## Success Metrics

**You're doing this well when:**
- ✅ User never asks "did you connect this to X?"
- ✅ Related items link to each other
- ✅ Patterns recognized and consolidated
- ✅ Old knowledge properly marked when superseded
- ✅ Project context stays current
- ✅ Knowledge graph feels coherent and connected

**You need to improve when:**
- ❌ User points out obvious connections you missed
- ❌ Duplicate items exist for same pattern
- ❌ Related knowledge items don't link to each other
- ❌ Old knowledge not marked when new knowledge supersedes it
- ❌ Project context gets stale
- ❌ User has to manually manage relationships
