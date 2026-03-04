# engram-lite Memory System Test Script

**Purpose:** Step-by-step protocol to verify that explicit and implicit memory capture and recall are working correctly with the engram-lite two-layer architecture.  
**Time required:** ~20 minutes across 3 short sessions  
**Prerequisites:** `amplifier` CLI installed, `engram-lite` installed and configured in your Amplifier bundle

---

## Architecture Quick Reference

Before running tests, understand the two-layer model you're verifying:

| Layer | Location | Written by | Read by |
|-------|----------|------------|---------|
| **Deep (DB)** | `~/.engram/engram.db` (user), `.engram/engram.db` (project) | `memory_capture()` tool | `memory_recall()` / `memory_search()` |
| **Hot (surface)** | `~/.engram/MEMORY.md` (user), `.engram/MEMORY.md` (project) | `memory_index(action="write", ...)` tool | Injected into context at session start |

**Critical:** `memory_capture()` writes ONLY to the SQLite DB. The agent must then separately call `memory_index(action="write")` to update MEMORY.md. The tests verify BOTH layers work.

---

## How to Read This Script

Each test has:
- **Setup** — what state to start in
- **Script** — exactly what to say to the agent
- **Expected behavior** — what the agent should do (and which tools it should call)
- **Verification** — shell commands to confirm it happened
- **Pass/Fail** — clear criteria

Run the tests in order. Tests 1–3 happen in **Session A**. Tests 4–6 happen in a **new Session B** (fresh conversation to test cross-session recall).

---

## How to Start a Session

```bash
# From your project directory (must contain .engram/ for project-space memories)
cd /path/to/your/project
amplifier
```

Or with an explicit bundle:
```bash
amplifier --bundle ./bundle.md
```

---

## Pre-Test: Verify Clean State

Before starting, record the baseline state of both DB layers:

```bash
# Initialize .engram/ in your project if it doesn't exist yet
engram-lite init --project-name my-test-project

# Check current memory counts (both user and project DBs)
engram-lite status

# Record row counts as your baseline
sqlite3 ~/.engram/engram.db \
  "SELECT COUNT(*) as user_memory_count FROM memories" 2>/dev/null || echo "User DB: not yet created"

sqlite3 .engram/engram.db \
  "SELECT COUNT(*) as project_memory_count FROM memories" 2>/dev/null || echo "Project DB: not yet created"

# Snapshot the current MEMORY.md files
echo "=== User MEMORY.md ===" && cat ~/.engram/MEMORY.md 2>/dev/null || echo "(does not exist yet)"
echo "=== Project MEMORY.md ===" && cat .engram/MEMORY.md 2>/dev/null || echo "(does not exist yet)"

# Save baseline row count for later diff
BASELINE_USER=$(sqlite3 ~/.engram/engram.db "SELECT COUNT(*) FROM memories" 2>/dev/null || echo "0")
BASELINE_PROJECT=$(sqlite3 .engram/engram.db "SELECT COUNT(*) FROM memories" 2>/dev/null || echo "0")
echo "Baseline: user=$BASELINE_USER project=$BASELINE_PROJECT"
```

If the DBs already have entries, note the existing IDs so you can distinguish new captures from old ones:

```bash
sqlite3 ~/.engram/engram.db \
  "SELECT id, domain, content_type, json_extract(data,'$.summary') FROM memories" 2>/dev/null
```

---

## SESSION A — Capture Tests

Open a new Amplifier session in your project directory. Tests 1, 2, and 3 run in the same session.

---

### Test 1: Explicit Global Preference Capture

**What we're testing:** Agent captures a clearly-stated personal preference into **user** space (persists across all projects) and updates the hot surface.

#### Script

Say this to the agent:

```
For all work you do with me, I want you to always structure any list 
of items as a numbered list, not bullet points. I prefer numbers 
because I reference items by number in follow-up messages. 
This applies everywhere — responses, summaries, code comments, everything.
```

#### Expected Behavior

The agent should:
1. Acknowledge the preference in its response
2. **Silently** call `memory_capture(content=..., content_type="preference", domain="personal/prefs", space="user", importance="high")`
3. **Silently** call `memory_index(action="read", scope="user")` then `memory_index(action="write", scope="user", content=<updated markdown>)` to persist to MEMORY.md
4. Use numbered lists in any subsequent response that has list content

#### Verification

Wait for the agent to respond, then run:

```bash
# 1. Check the DB — did a preference get written to user space?
sqlite3 ~/.engram/engram.db \
  "SELECT id, domain, content_type, importance, confidence, json_extract(data,'$.summary') 
   FROM memories 
   WHERE content_type='preference' 
   ORDER BY created_at DESC 
   LIMIT 5"

# 2. Search for the specific preference by content
sqlite3 ~/.engram/engram.db \
  "SELECT id, json_extract(data,'$.summary'), json_extract(data,'$.tags')
   FROM memory_fts 
   WHERE memory_fts MATCH 'numbered list OR bullet'
   LIMIT 5" 2>/dev/null || \
sqlite3 ~/.engram/engram.db \
  "SELECT id, json_extract(data,'$.summary') 
   FROM memories 
   WHERE json_extract(data,'$.content') LIKE '%numbered%' 
      OR json_extract(data,'$.summary') LIKE '%numbered%'"

# 3. Check the hot surface — did MEMORY.md get updated?
cat ~/.engram/MEMORY.md

# Look specifically for the pref entry
grep -E "\[pref\].*list|numbered|bullet" ~/.engram/MEMORY.md
```

#### Pass Criteria

- [ ] `sqlite3` query shows a new row in `~/.engram/engram.db` with `content_type='preference'` and `space='user'`
- [ ] `domain` is `personal/prefs` or similar (agent may infer this)
- [ ] `importance` is `high` (explicitly stated preference)
- [ ] `confidence` is `0.85` or higher (explicitly stated)
- [ ] `~/.engram/MEMORY.md` contains a `- [pref]` entry in the `## You` section referencing numbered lists
- [ ] Agent uses numbered format in its next response that contains list content

#### Fail Indicators

- No new row in DB → agent is not calling `memory_capture()` (check that engram-lite hook is enabled)
- DB row exists but MEMORY.md unchanged → agent wrote to DB but skipped the `memory_index(write)` step (two-step protocol not being followed)
- DB row with `confidence < 0.7` → agent treating explicit statement as uncertain
- `space='project'` instead of `space='user'` → formatting preference being scoped incorrectly (preferences should be user-global)

---

### Test 2: Explicit Project Constraint Capture

**What we're testing:** Agent captures a project-specific technical constraint into **project** space (scoped to this project directory) and updates the project-level MEMORY.md.

#### Script

```
This project has a strict no-mocking policy for tests. 
All tests must use real implementations — PGlite for database, 
real HTTP calls for API tests. Never use jest.mock() or vi.mock() 
for core business logic. This is a non-negotiable architectural decision.
```

#### Expected Behavior

The agent should:
1. Acknowledge the constraint
2. Call `memory_capture(..., content_type="constraint", space="project", importance="high")`
3. Update `.engram/MEMORY.md` (project-scoped) with a `- [constraint]` entry in `## Project`

#### Verification

```bash
# 1. Check the PROJECT DB (not user DB)
sqlite3 .engram/engram.db \
  "SELECT id, domain, content_type, importance, confidence, json_extract(data,'$.summary') 
   FROM memories 
   WHERE content_type IN ('constraint', 'decision')
   ORDER BY created_at DESC 
   LIMIT 5"

# 2. Search by content
sqlite3 .engram/engram.db \
  "SELECT id, json_extract(data,'$.summary')
   FROM memories
   WHERE json_extract(data,'$.content') LIKE '%mock%' 
      OR json_extract(data,'$.summary') LIKE '%mock%'
      OR json_extract(data,'$.content') LIKE '%PGlite%'"

# 3. Check project MEMORY.md hot surface
cat .engram/MEMORY.md

# Look for the constraint entry
grep -E "\[constraint\]|\[decision\]" .engram/MEMORY.md
grep -iE "mock|PGlite" .engram/MEMORY.md
```

#### Pass Criteria

- [ ] New row in `.engram/engram.db` (project DB) with `content_type='constraint'` and `importance='high'`
- [ ] `space='project'` (not `space='user'` — this is project-scoped)
- [ ] `.engram/MEMORY.md` has a `- [constraint]` entry in `## Project: <name>` section
- [ ] The entry references no-mocking or the PGlite policy

#### Note on DB vs. User Space

The agent might also write to `~/.engram/engram.db` under `domain='projects/<name>'`. Both are acceptable — the key test is that project-specific constraints live somewhere they can be recalled in future sessions when working in this project.

---

### Test 3: Implicit Pattern Capture (Multi-Turn)

**What we're testing:** Agent captures a preference it *infers* from repeated corrections — without being explicitly told "save this as a preference."

This test requires 3 exchanges. The agent should recognize the pattern by the 2nd or 3rd correction and call `memory_capture()` proactively.

#### Script

**Turn 1 — Ask, then correct toward brevity:**
```
Write a 3-sentence description of what this project does, 
for someone who has never heard of it.
```

After it responds:
```
That's too long. I need it shorter — one sentence max. 
Can you tighten it up?
```

**Turn 2 — Ask, then correct again:**
```
Now write a short explanation of how the memory system works 
for a developer reading the docs for the first time.
```

After it responds:
```
Still too much. My rule is: if it can be said in one sentence, 
it should be. Shorter please.
```

**Turn 3 — Ask, then correct once more:**
```
Describe what a project MEMORY.md file is, in your own words.
```

After it responds:
```
Good, but still could be tighter. I always want the minimum viable 
explanation — nothing more.
```

#### Expected Behavior

By the 2nd or 3rd correction, the agent should:
1. Recognize the pattern: user consistently prefers brevity
2. Call `memory_capture(content="...", content_type="preference", domain="personal/prefs", space="user")` — with **lower confidence** than Test 1 (~0.6–0.75) since this was inferred
3. Optionally update MEMORY.md with a `- [pref]` brevity entry
4. Write shorter responses proactively in Turn 3 without waiting to be corrected

#### Verification

```bash
# Search for a brevity/conciseness preference in user DB
sqlite3 ~/.engram/engram.db \
  "SELECT id, json_extract(data,'$.summary'), confidence, created_at
   FROM memories
   WHERE (json_extract(data,'$.content') LIKE '%brief%' 
      OR json_extract(data,'$.content') LIKE '%concis%' 
      OR json_extract(data,'$.content') LIKE '%short%'
      OR json_extract(data,'$.content') LIKE '%minimum%'
      OR json_extract(data,'$.summary') LIKE '%brevity%'
      OR json_extract(data,'$.summary') LIKE '%terse%')
   AND content_type='preference'
   ORDER BY created_at DESC"

# Check if confidence is lower than the explicit preference from Test 1
sqlite3 ~/.engram/engram.db \
  "SELECT id, confidence, json_extract(data,'$.summary') 
   FROM memories 
   WHERE content_type='preference' 
   ORDER BY created_at DESC 
   LIMIT 5"

# Check MEMORY.md
grep -iE "\[pref\].*brief|short|concis|minimum|terse" ~/.engram/MEMORY.md
```

#### Pass Criteria

- [ ] A new preference entry appears in `~/.engram/engram.db` capturing brevity/minimum-viable-explanation
- [ ] `confidence` is lower than the Test 1 entry (likely 0.6–0.75 vs. 0.85+)
- [ ] **OR:** Agent writes noticeably shorter, tighter responses in Turn 3 without being corrected

#### Acceptable Partial Pass

Agent writes shorter responses in Turn 3 proactively (it's tracking in-context) but hasn't yet written a formal memory entry. Correct behavior — explicit capture threshold is 2-3 consistent occurrences. The agent may wait until it has enough signal.

---

## SESSION B — Recall Tests

**Close the current conversation and start a completely fresh Amplifier session** in the same project directory. Do not carry over context or mention anything from Session A. The whole point is verifying the agent recalls *without being told*.

```bash
# Confirm both DBs have the memories from Session A before starting Session B
engram-lite status

# Confirm MEMORY.md files are populated (this is what gets injected at session start)
echo "--- User MEMORY.md ---" && cat ~/.engram/MEMORY.md
echo "--- Project MEMORY.md ---" && cat .engram/MEMORY.md
```

Then start a new Amplifier session.

---

### Test 4: Recall of Explicit Global Preference

**What we're testing:** The numbered-list preference from Test 1 is applied automatically in a new session.

#### How Recall Works (Two Paths)

1. **Hot surface injection:** `~/.engram/MEMORY.md` is injected at session start. If Test 1 wrote the `[pref]` entry there, the agent sees it before you type anything.
2. **Active recall:** Agent calls `memory_recall("user preferences formatting")` when context suggests prior prefs exist.

Both paths should result in correct behavior. The hot surface path is most reliable.

#### Script

Start the new session with a neutral task that naturally produces a list:

```
Can you list the main steps to set up a local development environment 
for a Python project? Just the key steps.
```

Do NOT mention numbered lists, formatting, or anything from Session A.

#### Expected Behavior

Agent should use numbered format (`1. First step`) automatically, not bullets (`- First step`).

#### Verification

Look at the response format. Then:

```bash
# Verify the MEMORY.md was injected (check file is populated, not empty)
grep "\[pref\]" ~/.engram/MEMORY.md

# The agent reads MEMORY.md at session start — no DB query needed for hot-path recall
# But to confirm the DB entry still exists:
sqlite3 ~/.engram/engram.db \
  "SELECT id, json_extract(data,'$.summary'), confidence 
   FROM memories 
   WHERE content_type='preference' 
   ORDER BY created_at DESC 
   LIMIT 3"
```

#### Pass Criteria

- [ ] Response uses `1. ... 2. ... 3. ...` format without being asked
- [ ] MEMORY.md `[pref]` entry still exists (recall does not delete memories)

#### Pass Path Distinction

| Response format | Explanation | Verdict |
|-----------------|-------------|---------|
| Numbered `1. 2. 3.` | Hot surface OR active recall worked | **PASS** |
| Bullet `- - -` | Neither path retrieved the preference | **FAIL** |

---

### Test 5: Recall of Project Constraint

**What we're testing:** The no-mocking constraint from Test 2 is applied when writing a test.

#### Script

Ask for a test — naturally, without mentioning the constraint:

```
Write a unit test for a function that saves a user record to the database.
```

#### Expected Behavior

Agent should:
1. Recall the no-mocking policy (from `.engram/MEMORY.md` hot surface OR via `memory_recall()`)
2. Write the test using PGlite or a real database approach
3. Not use `vi.mock()` or `jest.mock()` for the database layer
4. Ideally reference the constraint: *"...using PGlite per your no-mocking policy..."*

#### Verification

Inspect the generated test code:

```bash
# Confirm project MEMORY.md has the constraint (hot surface path)
grep -iE "\[constraint\]|\[decision\]" .engram/MEMORY.md
grep -iE "mock|PGlite" .engram/MEMORY.md

# Confirm project DB entry still exists
sqlite3 .engram/engram.db \
  "SELECT id, json_extract(data,'$.summary'), importance 
   FROM memories 
   WHERE content_type IN ('constraint','decision') 
   ORDER BY created_at DESC 
   LIMIT 5"
```

#### Pass Criteria

- [ ] Test code does NOT contain `vi.mock(` or `jest.mock(` for the database layer
- [ ] Test uses a real implementation approach (PGlite, real DB connection, or in-memory SQLite)
- [ ] Bonus: Agent explicitly references the no-mocking policy in its explanation

---

### Test 6: Brevity Preference Recall (Implicit)

**What we're testing:** The implicitly-captured brevity preference from Test 3 carries over.

#### Script

```
Explain what SSE (Server-Sent Events) is and why we use it.
```

#### Expected Behavior

If the implicit pattern was captured and written to MEMORY.md, the agent should write a short, tight explanation (1-3 sentences) rather than a comprehensive overview.

Compare: a default LLM answer for "explain SSE" would typically be 4-6+ sentences with code examples. A brevity-aware response should cut directly to the minimum viable explanation.

#### Verification

Count the sentences in the response:

```bash
# Verify whether a brevity preference is in MEMORY.md (hot surface)
grep -iE "brief|short|concis|minimum|terse|length" ~/.engram/MEMORY.md

# And/or in the DB
sqlite3 ~/.engram/engram.db \
  "SELECT id, confidence, json_extract(data,'$.summary') 
   FROM memories 
   WHERE (json_extract(data,'$.content') LIKE '%brief%' 
      OR json_extract(data,'$.summary') LIKE '%brevity%'
      OR json_extract(data,'$.summary') LIKE '%minimum%')
   AND content_type='preference'"
```

#### Pass Criteria (soft)

- [ ] Response is 1-3 sentences
- [ ] No preamble ("Great question! SSE stands for...") or post-amble ("I hope this helps!")
- [ ] Noticeably more concise than a typical LLM answer for this topic

This test is a **soft pass** — implicit capture has lower confidence and may not have been written to MEMORY.md if the agent didn't see enough signal in Session A.

---

## Post-Test: Full Memory Audit

After completing all tests, audit both layers:

```bash
echo "==========================="
echo "=== engram-lite status  ==="
echo "==========================="
engram-lite status

echo ""
echo "==============================="
echo "=== User DB: all memories  ==="
echo "==============================="
sqlite3 ~/.engram/engram.db \
  "SELECT id, space, domain, content_type, importance, confidence,
          json_extract(data,'$.summary') as summary,
          created_at
   FROM memories
   ORDER BY created_at DESC" \
  2>/dev/null || echo "User DB not found"

echo ""
echo "=================================="
echo "=== Project DB: all memories  ==="
echo "=================================="
sqlite3 .engram/engram.db \
  "SELECT id, space, domain, content_type, importance, confidence,
          json_extract(data,'$.summary') as summary,
          created_at
   FROM memories
   ORDER BY created_at DESC" \
  2>/dev/null || echo "Project DB not found"

echo ""
echo "=================================="
echo "=== Hot Surface: User MEMORY.md ==="
echo "=================================="
cat ~/.engram/MEMORY.md 2>/dev/null || echo "(does not exist)"

echo ""
echo "====================================="
echo "=== Hot Surface: Project MEMORY.md ==="
echo "====================================="
cat .engram/MEMORY.md 2>/dev/null || echo "(does not exist)"

echo ""
echo "==========================="
echo "=== Knowledge Graph     ==="
echo "==========================="
sqlite3 ~/.engram/engram.db \
  "SELECT from_id, relation_type, to_id, strength FROM memory_relations" \
  2>/dev/null | head -20
```

---

## Results Scorecard

| Test | What's Tested | Pass | Fail | Notes |
|------|--------------|------|------|-------|
| 1 | Explicit global preference → user DB + user MEMORY.md | | | |
| 2 | Explicit project constraint → project DB + project MEMORY.md | | | |
| 3 | Implicit brevity pattern inferred (lower confidence) | | | |
| 4 | Global preference recalled cross-session (numbered lists) | | | |
| 5 | Project constraint recalled cross-session (no-mocking) | | | |
| 6 | Implicit preference recalled cross-session (brevity) | | | |

**Score:** ___/6 (Tests 1–5 are hard requirements; Test 6 is soft)

---

## Interpreting Results

| Score | Meaning | Action |
|-------|---------|--------|
| 5–6/6 | Both layers fully functional | Ship it |
| 3–4/6 | Capture works, recall unreliable | Check MEMORY.md update step; verify hook is injecting context at session start |
| 1–2/6 | DB writes work, MEMORY.md not updating | Agent calling `memory_capture()` but not `memory_index(action="write")` — check the two-step protocol in context/memory-instructions.md |
| 0/6 | Nothing working | Check `~/.engram/` exists and is writable; verify `engram-lite` hook is configured in bundle |

---

## Common Failure Patterns and Fixes

### DB not being written to

```bash
# Verify directories exist and are writable
ls -la ~/.engram/ 2>/dev/null || mkdir -p ~/.engram
ls -la .engram/ 2>/dev/null || engram-lite init

# Check that the hook module is enabled (should appear in bundle config)
cat bundle.md | grep -A5 "engram"
```

### DB is written but MEMORY.md is not updated

This is the most common failure. The agent is capturing to SQLite (via `memory_capture()`) but not running the second step (`memory_index(action="write", ...)`). The MEMORY.md hot surface will be stale/empty.

Check `context/memory-instructions.md` — the **Phase 3 CAPTURE** section requires both steps. If MEMORY.md is consistently not updating, the memory instructions context may not be loaded in your bundle.

You can manually trigger a rebuild from the DB:
```bash
engram-lite rebuild-index --scope user
engram-lite rebuild-index --scope project
```

### Recall works in-session but not cross-session

The agent found the memory via `memory_recall()` in Session A but Session B starts cold. This means:
- MEMORY.md hot surface is empty (not injected at session start), AND
- The agent is not calling `memory_recall()` proactively at the start of Session B

Check that `hooks/amplifier_hook.py` is loading and injecting MEMORY.md into the context header. Run `engram-lite status` at the start of Session B to confirm the DB has entries before starting.

### MEMORY.md exists but recall fails in Session B

The agent may be seeing MEMORY.md injected but not parsing/applying the entries. Check the format — MEMORY.md must have the correct frontmatter and section structure:

```markdown
---
scope: user
updated: 2026-03-04T04:24:29Z
managed-by: engram-lite
entries: 3
---

# Memory

## You
- [pref] Prefers numbered lists over bullet points for all list content
- [pref] Prefers minimum viable explanations — one sentence if possible

## Now
- [event] Testing memory system (Session A)
```

If entries are missing the `- [type]` prefix, the context hook may not recognize them as structured entries.

### Implicit capture never triggers (Test 3)

Implicit capture requires 2-3 consistent in-session corrections. If the agent doesn't capture:
- Try spacing corrections more explicitly: "This is too long again — I always prefer one sentence"
- Add an explicit signal at Turn 3: "You keep over-explaining things — I want you to internalize that I prefer minimum viable answers"
- Implicit capture has a lower confidence threshold and may appear as `confidence: 0.60–0.75`

---

## Simulating Tests Without a Fresh Session

To test recall without switching sessions, manually inspect what the agent knows:

```
What do you know about my formatting preferences?
```

```
What constraints apply to this project's tests?
```

The agent should call `memory_recall()` and surface what was captured. This verifies DB retrieval without requiring a cross-session reset, though it doesn't test the MEMORY.md injection path.

---

*Run this test script whenever making changes to the memory system protocol, hook configuration, or agent context files to verify nothing regressed.*
