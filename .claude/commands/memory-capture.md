---
name: memory-capture
description: Capture knowledge to appropriate memory location following dual-write protocol
---

Capturing to memory: $ARGUMENTS

## Dual-Write Decision

Applying decision tree:

### Step 1: Personal Information?

Is this about the user personally? (preferences, constraints, work patterns)

**→ YES**: Capture to `~/.canvas/memory/personal/` ONLY

**→ NO**: Continue to Step 2...

### Step 2: Helpful to Project?

Is this project-specific knowledge?

**→ NO**: Skip capture or user memory only

**→ YES**: Continue to Step 3...

### Step 3: README Test

"Could this appear in project README without causing harm?"

**→ NO**: Capture to `~/.canvas/memory/projects/{name}/` ONLY

**→ YES**: Dual-write to BOTH:
1. `~/.canvas/memory/projects/{name}/`
2. `.canvas/memory/`

## Capture Format

Creating memory file with:
- YAML frontmatter with keywords
- Inductive structure (conclusion first)
- Proper domain routing
- Cross-references where applicable

## Executing Capture

[Performs the actual file write operations based on routing decision]

✓ Knowledge captured successfully
