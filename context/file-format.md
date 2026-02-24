# Memory File Formats

## Hot Memory — `~/.canvas/memory/information/{domain}/{topic}.md`

```markdown
---
id: info-{date}-{sequence}
created: 2026-02-18T23:00:00Z
modified: 2026-02-18T23:00:00Z
project: {project-name}
tags: [tag1, tag2, tag3]
keywords: [precise-term1, precise-term2, "multi word phrase", acronym]
relates-to: [info-001, info-002]
dimensions:
  confidence: 0.85
  importance: high
  relevance: [domain1, domain2]
  expires: null
visibility: private
---

# Title: Clear Description

## Core Understanding
What's the main insight? (1-2 sentences max)

## Supporting Context
Where did this come from? (2-3 bullet points)

## Connections
How does this relate to other knowledge?
- See also: archive/2026-02-18-detailed-discussion.md

**Size limit:** 200-500 words total.
```

## Cold Storage — `~/.canvas/memory/archive/{domain}/{date}-{topic}.md`

```markdown
---
id: archive-{date}-{sequence}
created: 2026-02-18T23:00:00Z
referenced-by: [info-001, info-002]
tags: [tag1, tag2, tag3]
keywords: [same keywords as hot reference]
visibility: private
---

# Title: Detailed Context

[Full content - no size limit]
```

## Project Memory — `./.canvas/memory/knowledge/{topic}.md`

```markdown
---
created: 2026-02-18T23:00:00Z
contributors: [ken]
tags: [tag1, tag2, tag3]
keywords: [specific-tech-term, acronym, precise-search-term]
relates-to: [other-project-docs]
---

# Title: Factual, Helpful Knowledge

## What We Learned
Clear, factual description. NO personal observations.

## Why It Matters
How this knowledge helps the project.

**Test:** Could this appear in project README without causing harm? If no → don't write it here.
```
