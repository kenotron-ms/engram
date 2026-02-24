# Agent Instructions: Engram

Agentic memory augmentation. **Violating the letter of the rules is violating the spirit of the rules.**

---

## Mandatory Loop (Every Message)

```
RETRIEVE    → RESPOND → CAPTURE
(automatic)   (visible)  (silent)
```

**Before responding:** Relevant memory is injected automatically — apply any `<retrieved-memory>` in your context.
**After responding:** Did I learn something NEW? If yes → capture immediately, silently.

Details: `@engram:context/protocols/inline-capture.md`

---

## Quick Dispatch

| You learned... | Protocol | Storage |
|----------------|----------|---------|
| User preference or constraint | `@engram:context/protocols/inline-capture.md` | hot: `information/{domain}/` |
| New domain knowledge | `@engram:context/protocols/knowledge-extraction.md` | hot or cold by size |
| Decision/pattern (2nd+ occurrence) | `@engram:context/protocols/inline-capture.md` | hot: `information/{domain}/` |
| Project context (shareable) | `@engram:context/protocols/dual-write-decision.md` | user + project |
| Cross-reference needed | `@engram:context/protocols/cross-reference-cascade.md` | update existing items |

## Storage Routing

| Size | Destination |
|------|-------------|
| ≤500 words | Hot: `~/.canvas/memory/information/{domain}/{topic}.md` |
| >500 words | Cold: `~/.canvas/memory/archive/{domain}/{date}-{topic}.md` + hot reference |

## Domain Routing

| Domain | Signals |
|--------|---------|
| `projects/{name}/` | Project-specific discussion |
| `professional/{area}/` | HIPAA, compliance, domain expertise |
| `personal/preferences/` | How user likes to work |

Details: `@engram:context/protocols/scope-routing.md`

## Dual-Write

- Personal observation → `~/.canvas/memory/` ONLY
- Project-helpful + public-safe → BOTH (`~/.canvas/` and `./.canvas/`)

Details: `@engram:context/protocols/dual-write-decision.md`

---

## Protocols

| Protocol | File | Use when |
|----------|------|----------|
| Inline Capture | `@engram:context/protocols/inline-capture.md` | Every message |
| Knowledge Extraction | `@engram:context/protocols/knowledge-extraction.md` | New domain knowledge |
| Dual-Write Decision | `@engram:context/protocols/dual-write-decision.md` | User-only vs user+project |
| Scope Routing | `@engram:context/protocols/scope-routing.md` | Domain inference and search |
| Cross-Reference Cascade | `@engram:context/protocols/cross-reference-cascade.md` | After any capture |

---

## Rules

1. **Domain-first search** — scope before grepping. Folder structure IS the optimization.
2. **Keywords mandatory** — include variations: singular/plural, synonyms, acronyms, quoted phrases.
3. **Inductive writing** — conclusion first, evidence below.
4. **Retrieve-optimized structure** — ask "what question will I ask when I look for this?"
5. **Self-verify** — re-scan source, check locations, verify keywords. Don't ask user to verify.
6. **Cross-reference cascade** — after capture, check related items, projects, patterns, temporal effects.
7. **Temporal awareness** — check `expires:` before applying old knowledge.
8. **Update on mistakes** — fix issue + update the relevant protocol. Conversation-only learnings are lost.
9. **Read before asking** — check if the file exists before asking the user.

---

## Manual Search (if auto-retrieval missed something)

```bash
grep -r "term" ~/.canvas/memory/information/{domain}/
```

**At project start:** `read_file ./.canvas/memory/context.md`

---

## File Formats

See `@engram:context/file-format.md` for hot memory, cold storage, and project memory templates.

---

**Spec:** `@engram:context/memory-system.md` | **Protocols:** `@engram:context/protocols/`
