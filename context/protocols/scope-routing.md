# Scope Routing Protocol

> **Use when:** Before ANY memory search. Domain inference is MANDATORY.

```
NO SEARCH WITHOUT DOMAIN INFERENCE
```

**Violating the letter of this protocol is violating the spirit of this protocol.**

---

## The Core Principle

**The folder structure IS the search optimization.**

Searching all 1000 memory items returns 50+ matches requiring ranking. Searching 20-50 items in the correct domain returns 3-5 directly relevant matches.

Domain inference must happen BEFORE grep.

---

## The Three-Axis Model

Memory is organized by your relationship to the content:

| Axis | What it contains | Signals |
|------|------------------|---------|
| **projects/{name}/** | Project-specific knowledge | Conversation about specific project, project name mentioned |
| **professional/{area}/** | Domain expertise (healthcare, architecture, security, etc.) | Technical domain knowledge, work expertise |
| **personal/{area}/** | How you work, constraints, preferences | Communication style, access limitations, working patterns |

---

## Domain Inference Strategy

### Step 1: Identify Topic Signals

What is the conversation about?

| Signal Type | Examples |
|-------------|----------|
| **Project names** | "memory-system", "auth-service", "data-pipeline" |
| **Technical domains** | HIPAA, security, architecture, compliance |
| **Preference words** | "I prefer", "how I work", "my style" |
| **Constraint words** | "I don't have access", "I can't", "limited to" |

### Step 2: Map Signals to Domains

| If conversation contains... | Primary domain | Secondary domain |
|------------------------------|----------------|------------------|
| Project name | `projects/{name}/` | Related professional domain |
| HIPAA, PHI, compliance | `professional/healthcare/` | Current project if applicable |
| Architecture, design patterns | `professional/architecture/` | Current project if applicable |
| "I prefer", "my style" | `personal/preferences/` | Current project if relevant |
| "I don't have access" | `personal/constraints/` | N/A |
| Security, threats, vulnerabilities | `professional/security/` | Current project if applicable |

### Step 3: Search Strategy

| Certainty Level | Action |
|-----------------|--------|
| **High certainty (90%+)** | Search single domain |
| **Medium certainty (60-90%)** | Search 2-3 most likely domains |
| **Low certainty (<60%)** | Search all relevant professional/ + personal/ domains |

**Never default to searching everything.** Even low certainty should be domain-scoped (e.g., all of `professional/` but not `projects/`).

---

## Domain Catalog

### Projects Domain

**Pattern:** `projects/{project-name}/`

**What belongs here:**
- Project-specific decisions and context
- Why this project uses certain patterns
- Project constraints and requirements
- Team agreements and conventions

**Signals:**
- Conversation explicitly about a project
- Project name mentioned
- "In this project", "for this codebase"

**Example:**
```
User: "Why did we choose this architecture for the memory system?"
→ Domain: projects/memory-system/
→ Search: keywords about architecture decisions
```

### Professional Domain

**Pattern:** `professional/{area}/`

**Common areas:**
- `professional/healthcare/` - HIPAA, PHI, medical compliance
- `professional/architecture/` - System design, patterns, best practices
- `professional/security/` - Security patterns, threat models
- `professional/data/` - Data modeling, database design

**What belongs here:**
- Portable domain expertise (not project-specific)
- Technical knowledge that applies across projects
- Standards and compliance requirements
- Best practices and patterns

**Signals:**
- Technical domain terminology
- "Generally speaking", "best practice"
- Standards like HIPAA, GDPR, SOC2
- Not tied to specific project

**Example:**
```
User: "What are the HIPAA requirements for data encryption?"
→ Domain: professional/healthcare/
→ Search: keywords about HIPAA, encryption, data protection
```

### Personal Domain

**Pattern:** `personal/{area}/`

**Common areas:**
- `personal/preferences/` - How you like things done
- `personal/constraints/` - Access limitations, tool restrictions
- `personal/communication/` - Communication style, audience preferences
- `personal/workflow/` - How you work, process preferences

**What belongs here:**
- How you prefer to work
- Your constraints and limitations
- Your communication style
- Your access patterns

**Signals:**
- "I prefer", "I like", "my style"
- "I don't have access to", "I can't use"
- "When I present to", "my audience"
- First-person statements about working style

**Example:**
```
User: "Remember, I prefer bottom-line-first presentations"
→ Domain: personal/preferences/
→ Subdomain: communication or presentation-style
→ Capture this as preference
```

---

## Self-Organizing Domains

**New domains emerge from use.** When content doesn't fit existing domains:

1. Recognize the mismatch
2. Propose new domain to user
3. On approval, create folder and initial structure
4. Update this protocol with the new domain

**Example emergence pattern:**
```
Week 1: All professional knowledge in professional/
Week 4: professional/architecture/ emerges
Week 8: professional/security/ emerges
Week 12: professional/data/ emerges
```

Each split happens when a subdomain accumulates enough distinct knowledge to merit separate organization.

---

## Multi-Domain Searches

Sometimes content spans domains. Search multiple:

| Scenario | Search Strategy |
|----------|-----------------|
| Project + domain expertise | Search `projects/{name}/` AND `professional/{area}/` |
| Preference + project context | Search `personal/preferences/` AND `projects/{name}/` |
| Broad technical question | Search all `professional/` domains |

**Use parallel searches:**
```bash
python scripts/canvas-memory-search.py --keyword "HIPAA" --domain "professional/healthcare/"
python scripts/canvas-memory-search.py --keyword "encryption" --domain "projects/memory-system/"
```

---

## The Relationship Test

**The same TOPIC can route to different domains based on your relationship to it.**

| Topic | Context | Domain | Why |
|-------|---------|--------|-----|
| HIPAA | General knowledge about requirements | `professional/healthcare/` | Portable expertise |
| HIPAA | How we implemented it in project X | `projects/{x}/` | Project-specific |
| Presentations | General preference for style | `personal/preferences/` | Your working style |
| Presentations | Project Y's stakeholder requirements | `projects/{y}/` | Project-specific |

**Ask:** "Is this about HOW I work, or about WHAT this project needs?"

---

## Gate Function

```
BEFORE searching memory:
  1. CHECK: Identified at least one topic signal?
  2. CHECK: Mapped signal to domain(s)?
  3. CHECK: Determined search strategy (single vs multi-domain)?
  4. CHECK: Constructed domain-scoped search command?
  If ANY check fails: Do not search. Complete domain inference.
```

---

## Three-Failure Escalation

If the Gate Function fails 3 times in the same session (e.g., repeatedly searching all domains without inference, misrouting content to wrong domains), STOP immediately.

1. State what you attempted and what failed each time
2. Ask the user for explicit guidance
3. Do not resume until the user provides direction

Domain routing is critical for performance. Repeated failures indicate the inference mechanism is broken. Escalate.

---

## Red Flags

If you catch yourself thinking:
- "I'll just search everything to be safe"
- "This could be anywhere, so I won't scope it"
- "Domain inference takes too much time"
- "The search tool will find it regardless"

**All of these mean: STOP. Re-read the Core Principle section above.**

---

## Common Rationalizations

| Excuse | Reality |
|--------|---------|
| "Searching everything is safer" | It's 10x slower and returns noise. Inference is faster. |
| "I'm not sure which domain" | Medium certainty = search 2-3 domains. Still better than all. |
| "This is a quick search" | Quick searches still need domain scoping for performance. |
| "The tool will rank results" | Grep doesn't rank. Domain scoping IS the ranking. |

---

## Anti-Patterns

| Don't | Do |
|-------|-----|
| Search `~/.canvas/memory/information/` (entire tree) | Infer domain, search `~/.canvas/memory/information/projects/memory-system/` |
| Default to broad search when uncertain | Search 2-3 most likely domains |
| Skip inference for "simple" searches | Infer domain for EVERY search |
| Rely on grep to find needles in haystack | Use folder structure as first-pass filter |

---

## Success Metrics

**You're doing this well when:**
- ✅ Searches complete in <50ms
- ✅ Results are directly relevant (not noise)
- ✅ 3-5 matches per search, not 50+
- ✅ You can explain why you searched each domain

**You need to improve when:**
- ❌ Searches take >500ms
- ❌ Many irrelevant results
- ❌ Frequently searching entire memory tree
- ❌ Can't explain domain choices
