# Epic 06: Memory System - Documentation

**Organized using the [Divio Documentation System](https://docs.divio.com/documentation-system/)**

---

## The Four Quadrants

```
                    STUDY          |         ACTION
            -------------------|-------------------
PRACTICAL   Implementation     |  How-To Guides
            (learning)        |  (problem-solving)
            -------------------|-------------------
THEORETICAL Explanation        |  Reference
            (understanding)   |  (information)
```

---

## Quick Navigation

**Want to build the memory system?**  
→ `implementation/implementation_plan/00-OVERVIEW.md`

**Need to integrate tools or debug extraction?**  
→ `how-to/01-agent-sdk-integration.md`

**Looking for architecture or API specs?**  
→ `reference/01-architecture.md`

**Want to understand why we extract certain facts?**  
→ `explanation/01-extraction-criteria.md`

---

## Directory Structure

### implementation/ (Plans & Status)

**Current implementation progress:**
- `implementation_plan/` - Phases 1-12 with validation
  - Phases 1-7: ✅ Complete (backend, extraction, tools)
  - Phases 8-12: 🔲 Pending (frontend UI)
- `implementation-status.md` - Detailed progress tracking

### how-to/ (Problem-Solving)

**Solve specific integration problems:**
1. `01-agent-sdk-integration.md` - Claude Agent SDK tool integration
2. `02-amplifier-cli-integration.md` - Amplifier CLI support
3. `03-debug-extraction-logs.md` - Debugging extraction issues

### reference/ (Technical Specs)

**Factual specifications:**
1. `01-architecture.md` - System architecture and data flow
2. `02-package-specification.md` - `@workspaces/memory` package spec
3. `03-features-list.md` - Complete feature list
4. `04-recipe-design.md` - Amplifier recipe structure
5. `05-amplifier-extraction-guide.md` - LLM extraction guide
- `mockups/` - Visual mockups (PNG files)

### explanation/ (Design Rationale)

**Why we made certain decisions:**
1. `01-extraction-criteria.md` - What deserves to be remembered
2. `02-extraction-recommendations.md` - Why these patterns
3. `03-technology-roadmap.md` - When to add vector search, graph

---

## Memory UI Design

**New work (root ai_working/):**
- `ai_working/memory-ui-design.md` - Design specification
- `ai_working/memory-ui-mockup.html` - Lofi prototype
- `ai_working/memory-ui-implementation-plan.md` - Implementation steps

**Design:** Three-scope tabs (User/Project/Workspace) with AI summaries and delete-only curation.

---

## Current Status

✅ Backend Complete - Phases 1-7  
✅ Agent Integration Complete - All 4 project types  
🎨 UI Design Complete - Lofi mockup and implementation plan  
🔲 Frontend Implementation - Not started

See `implementation/implementation-status.md` for detailed progress.
