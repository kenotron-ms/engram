# Epic 06: Memory System - Features

**Owner:** Chris Park
**Tech Writer:** AI Assistant
**Last Updated:** 2026-01-22
**Status:** Design Complete → Ready for Architecture

---

## Executive Summary

Canvas memory enables AI to automatically learn from conversations and apply context without users repeating themselves. Three scopes (user, project, workspace) with progressive discovery via tools. ChatGPT-style auto-extraction + Claude-style on-demand retrieval = best of both worlds.

**Phase 1 (MVP):** Auto-discovery foundation
**Phase 2:** Intelligence layer (patterns, cleanup)
**Phase 3:** Team collaboration

---

## Design Foundation

### What We Learned from Industry Research

**ChatGPT Memory ([OpenAI](https://openai.com/index/memory-and-new-controls-for-chatgpt/)):**
- ✅ Auto-discovers facts from conversations (passive learning)
- ✅ Discrete facts, not synthesis paragraphs (simple, transparent)
- ✅ Deletion-only UI in Settings (proven sufficient)
- ✅ Confidence-based extraction (quality over quantity)

**Claude Projects ([Anthropic](https://docs.claude.com/en/docs/agents-and-tools/tool-use/memory-tool)):**
- ✅ Tool-based progressive discovery (AI searches on-demand)
- ✅ Project-scoped memory (isolation)
- ✅ 84% token reduction (not pre-loading everything)
- ✅ File-based storage with transparency

**Mem0/MemU ([Mem0](https://mem0.ai/research), [MemU](https://memu.pro/)):**
- ✅ Vector search for semantic similarity (26% accuracy improvement)
- ✅ Memory decay for cleanup (90% cost reduction)
- ✅ Usage tracking for ranking (most-used first)
- ✅ Graph relationships for conflict detection

### Core Principles

1. **Auto-discovery first** - AI extracts facts automatically (80%+ of memories)
2. **Progressive loading** - AI searches on-demand via tools (not pre-loaded)
3. **Deletion-only management** - No editing needed (delete and re-learn)
4. **Three scopes** - User (everywhere), Project (one project), Workspace (all projects in workspace)
5. **Conversation-first** - Optional commands, not required
6. **Settings-based UI** - Not prominent panel (follows ChatGPT/Claude patterns)

---

## Phase 1: Auto-Discovery Foundation (MVP)

**Goal:** AI learns automatically and applies context transparently

### F1.1: Automatic Fact Extraction
**AI extracts discrete facts from conversations after each session**

**User Experience:**
```
[User working in "Q1 Board Deck" project]

User: "Create a board deck for the CEO. He prefers slides
       to be concise - max 3 bullets per slide."

AI: [generates deck with 3 bullets per slide]

[Session ends]

Notification appears:
"💭 Remembered:
    • CEO prefers max 3 bullets per slide
    • User creates board decks"
```

**How It Works:**

**Trigger:** After chat session ends (user closes chat or switches projects)

**Process:**
1. AI analyzes full session transcript
2. Extracts 0-5 discrete facts per session
3. Assigns scope (user/project/workspace) based on context
4. Stores facts with confidence score (>70% threshold)
5. Notifies user of what was learned

**What Gets Extracted:**
- **Preferences:** "Prefers Inter font", "Likes concise summaries"
- **Context:** "EA supporting C-suite", "Works in healthcare"
- **Constraints:** "HIPAA compliant - use Jane Doe examples"
- **Patterns:** "Board decks start with metrics", "Presentations use 3-slide structure"

**Scope Inference (Automatic):**
- Mentioned in one project → `scope: project`
- Mentioned across 2+ projects in workspace → `scope: workspace`
- Mentioned across 2+ workspaces → `scope: user`
- AI infers, user can override via deletion

**Duplicate Prevention:**
Before saving fact:
1. Search for similar existing facts
2. LLM checks: Is this a duplicate?
3. If duplicate → Skip saving (already have it)
4. If new → Save it

**No Post-Extraction Notification:**
- AI learns silently (no popup, no banner)
- Settings badge shows: "3 new" (passive indicator)
- User reviews in Settings when convenient

**Tool Call Result (Natural Discovery):**
```
[After session ends, AI extracts facts via tool call]

Chat shows (if tool results visible):
<tool_result>
  Created 2 memories:
  • CEO prefers 3 bullets max per slide
  • User creates board materials
</tool_result>

User may or may not see this (depends on UI settings)
Main discovery: Settings badge appears
```

**Success Criteria:**
- 80%+ facts extracted without manual commands
- 90%+ accuracy (user doesn't delete immediately)
- No workflow interruption (silent learning)
- Duplicates prevented 95%+ of time

**Technical Requirements:**
- LLM fact extraction prompt (after session)
- Database storage (`memory_facts` table)
- Conflict detection (LLM compares facts)
- Notification system (toast/banner)

---

### F1.2: Progressive Memory Discovery (Tool-Based)
**AI searches and loads memories on-demand, not pre-loaded**

**User Experience:**
```
[User opens project "Q1 Board Deck"]

AI (visible to user):
"Continuing Q1 Board Deck. Last time you were working
 on the metrics slide."

AI (internal, hidden from user):
<tool_use>
  <name>memory_search</name>
  <parameters>
    <query>CEO board deck preferences</query>
    <scope_ids>["project-123", "workspace-456", "user-789"]</scope_ids>
  </parameters>
</tool_use>

Result: ["CEO prefers 3 bullets max", "Board deck: Metrics→Product→Strategy"]

[AI loads these 2 facts into context for this conversation]
```

**Available Memory Tools:**

**1. memory_search(query, scope_ids)**
- Searches across specified scopes
- Returns ranked facts (relevance + usage + confidence)
- Used when: Project opened, topic mentioned, AI needs context

**2. memory_list(scope, scope_id)**
- Lists all facts for a specific scope
- Returns: All user facts, all project facts, all workspace facts
- Used when: User asks "What do you remember?", full context needed

**3. memory_create(fact, scope, scope_id, metadata)**
- Creates new fact immediately
- Used when: User says "Remember:", manual override
- Includes conflict check before saving

**4. memory_delete(id)**
- Deletes specific fact
- Used when: User says "Forget about X", removes obsolete fact
- Returns: Confirmation of deletion

**When AI Uses Tools:**
- **Session start:** Search for relevant project/workspace/user memories
- **Topic mentioned:** Search for facts related to topic ("CEO", "brand", "HIPAA")
- **User asks:** "What do you remember?" → List all facts
- **Uncertain:** "Do I have preferences for X?" → Search before guessing
- **User commands:** "Remember X" → Create fact, "Forget Y" → Delete fact

**NOT Pre-Loaded:**
- Memories aren't in every message prompt
- AI decides what's relevant via search
- Loads top 10-20 facts only
- Saves 70-90% tokens vs full context (Mem0 benchmark)

**Success Criteria:**
- AI finds relevant memories 90%+ of time
- Tool calls complete in <500ms
- User doesn't notice tool overhead
- Token usage 70-90% lower than full-context baseline

**Technical Requirements:**
- Backend tool handlers (memory_search, memory_list, memory_create, memory_delete)
- Database indexes for fast search (gin full-text index)
- Tool execution logging (track what AI searched for)
- Latency optimization (<500ms per tool call)

---

### F1.3: Three Memory Scopes
**Facts automatically scoped to user, project, or workspace**

**Scope Hierarchy:**

**User Scope** (applies everywhere)
```
Facts:
- "EA supporting C-suite executives"
- "Prefers Inter font for all documents"
- "Conversational but professional tone"

Visibility: All projects for this user
Priority: Lowest (overridden by workspace/project)
Storage: user/{user_id}/memories
```

**Project Scope** (applies to one project only)
```
Facts:
- "Q1 board deck for CEO"
- "Focus: Growth metrics (40% YoY), Series B announcement"
- "Progress: Metrics slide complete, working on product roadmap"

Visibility: Only this specific project
Priority: Highest (most specific context)
Storage: project/{project_id}/memories
```

**Workspace Scope** (applies to all projects in workspace)
```
Facts:
- "CEO prefers max 3 bullets per slide"
- "Board deck structure: Metrics → Product → Strategy → Asks"
- "Strategic focus, not operational details"

Visibility: All projects within this workspace
Priority: Medium (more specific than user, less than project)
Storage: workspace/{workspace_id}/memories
```

**Priority When Applying:** Project > Workspace > User

**Example:**
```
User (global): "I prefer detailed explanations"
Workspace (CEO): "CEO wants concise (3 bullets max)"
Project (Q1 Deck): "This deck: 5 bullets for complex topics"

AI working in Q1 Deck project:
→ Uses: "5 bullets for complex topics" (project wins)
→ Ignores: Workspace and User preferences (lower priority)
```

**Scope Inference (Automatic):**

**During extraction, AI determines scope:**
```typescript
// Example LLM prompt for scope inference
const scopePrompt = `
Analyze this fact: "CEO prefers 3 bullets max per slide"

Context:
- Current project: "Q1 Board Deck" (workspace: "CEO Materials")
- User has 5 other projects in "CEO Materials" workspace
- User has 2 other workspaces

Which scope should this fact have?

A) user - Applies to ALL of user's work
B) workspace - Applies to "CEO Materials" workspace only
C) project - Applies to "Q1 Board Deck" only

Reasoning:
"CEO" is mentioned → Related to workspace
"3 bullets" is presentation preference → Applies to all CEO presentations
Not project-specific detail → Workspace scope

Answer: B) workspace
`;
```

**User Override:**
- If wrong scope assigned → User deletes fact from wrong scope, re-creates in right scope
- Or waits for AI to re-extract in correct scope next session

**Success Criteria:**
- 90%+ facts assigned correct scope automatically
- CEO workspace facts don't leak to CFO workspace
- User facts apply everywhere consistently
- Priority hierarchy works (project wins conflicts)

**Technical Requirements:**
- Scope inference logic (LLM-based)
- Database query filters by scope_id
- Tool search accepts multiple scope_ids
- Priority sorting in search results

---

### F1.4: Silent Memory Application
**AI uses memories transparently without announcing every time**

**User Experience:**
```
User: "Create a board deck for CEO"

AI: "I'll create a CEO board deck with concise bullet points."

[Generates deck with 3 bullets per slide]
```

**What user DOESN'T see:**
- ❌ "Searching memories..."
- ❌ "Loaded 3 memories: X, Y, Z"
- ❌ "Applying CEO preferences..."
- ❌ "Using memory #mem-123..."

**What user DOES see:**
- ✅ Output quality (deck has 3 bullets as expected)
- ✅ AI mentions context naturally ("concise bullet points")
- ✅ Can ask "Did you remember X?" if curious

**Behind the Scenes:**
```
AI (internal):
1. User mentioned "CEO board deck"
2. <tool: memory_search("CEO board deck")>
3. Found: "CEO prefers 3 bullets max"
4. Adds to system prompt context
5. Generates response using that context
```

**Exception - First-Time Notification:**
```
[After AI extracts a NEW fact]

💭 Remembered: CEO prefers 3 bullets max
[Dismiss]
```
Shows once when created, never again when used

**Success Criteria:**
- Memories applied silently (no workflow interruption)
- Output reflects context automatically
- User trusts AI "gets it" without announcements
- Feels ambient and intelligent, not robotic

**Technical Requirements:**
- Memory search results injected into system prompt
- No user-facing "memory applied" messages
- One-time notification on creation only
- Optional user query: "What memories did you use?" (debug mode)

---

### F1.5: Memory Management (Settings UI)
**Simple deletion-only interface following ChatGPT pattern**

**Location:** Settings > Memory

**User Experience:**

**View All Memories:**
```
┌────────────────────────────────────────────────────┐
│ Memory                                        [×]  │
├────────────────────────────────────────────────────┤
│                                                    │
│ Canvas automatically learns from your             │
│ conversations. You can delete anything you        │
│ don't want remembered.                            │
│                                                    │
│ ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━ │
│                                                    │
│ YOUR MEMORIES (applies everywhere)          3     │
│                                                    │
│ • EA supporting C-suite executives          [×]   │
│   Created: Jan 10 • Used: 15 times                │
│                                                    │
│ • Prefers Inter font                        [×]   │
│   Created: Jan 12 • Used: 8 times                 │
│                                                    │
│ • Conversational but professional tone      [×]   │
│   Created: Jan 14 • Used: 12 times                │
│                                                    │
│ ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━ │
│                                                    │
│ WORKSPACE: CEO Board Materials              2     │
│                                                    │
│ • CEO prefers 3 bullets max per slide       [×]   │
│   Created: Jan 11 • Used: 10 times                │
│                                                    │
│ • Board deck: Metrics→Product→Strategy      [×]   │
│   Created: Jan 13 • Used: 5 times                 │
│                                                    │
│ ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━ │
│                                                    │
│ PROJECT: Q1 Board Deck                      2     │
│                                                    │
│ • Q1 focus: Growth metrics, Series B        [×]   │
│   Created: Jan 15 • Used: 7 times                 │
│                                                    │
│ • Progress: Metrics slide complete          [×]   │
│   Created: Jan 18 • Used: 3 times                 │
│                                                    │
│ ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━ │
│                                                    │
│                              [Clear all memories]  │
│                                                    │
└────────────────────────────────────────────────────┘
```

**UI Features:**
- ✅ List all facts grouped by scope
- ✅ Show metadata: created date, use count
- ✅ Delete individual fact ([×] icon)
- ✅ Clear all memories (bulk delete with confirmation)
- ❌ NO editing (deletion is sufficient - ChatGPT proven)
- ❌ NO categories/tags (flat chronological list)
- ❌ NO quality stars (use count is sufficient)
- ❌ NO search (just scroll - small list in Phase 1)

**Deletion Confirmation:**
```
Delete memory?
"CEO prefers 3 bullets max per slide"

This memory has been used 10 times.

[Cancel] [Delete]
```

**Clear All Confirmation:**
```
Clear all memories?

This will delete:
• 3 user memories
• 2 workspace memories
• 2 project memories

You can't undo this action.

[Cancel] [Clear All]
```

**Why Deletion-Only?**
1. **ChatGPT proved it works** - Millions of users, no editing needed
2. **If fact is wrong** → Delete, AI re-learns correctly next session
3. **If fact is outdated** → Delete, AI extracts updated version
4. **Simpler mental model** - "Let AI learn, or delete if wrong"

**Success Criteria:**
- User can audit all memories in one view
- Deletion is instant (<200ms)
- UI is simple and unintimidating
- Grouped by scope (easy to scan)
- <5% of facts deleted (proves extraction accuracy)

**Technical Requirements:**
- Query: `SELECT * FROM memory_facts WHERE scope_id IN (...) ORDER BY scope, created_at DESC`
- Delete: `DELETE FROM memory_facts WHERE id = $1`
- Clear all: `DELETE FROM memory_facts WHERE scope_id = $1` (with confirmation)
- Group by scope for display

---

### F1.6: Conversation-Based Commands
**Optional natural language control (convenience, not requirement)**

**Commands:**

**Create Memory (Explicit):**
```
User: "Remember this: CEO prefers 3 bullets max"

AI: "✓ Remembered for workspace: CEO Materials

     'CEO prefers 3 bullets max per slide'

     This will apply to all CEO projects."
```

**Delete Memory:**
```
User: "Forget about the 3 bullets rule"

AI (internal):
<tool: memory_search("3 bullets rule")>
Found: "CEO prefers 3 bullets max per slide"
<tool: memory_delete(id)>

AI: "✓ Forgot: 'CEO prefers 3 bullets max per slide'

     I won't apply this anymore."
```

**Query Memories:**
```
User: "What do you remember about CEO?"

AI (internal):
<tool: memory_search("CEO")>

AI: "I remember:
     • CEO prefers max 3 bullets per slide
     • Board deck structure: Metrics → Product → Strategy
     • Strategic focus, not operational

     [View all in Settings]"
```

**Skip Extraction:**
```
User: "Don't remember anything from this conversation"

AI: "✓ Got it. I won't extract memories from this session."

[Session ends, extraction skipped]
```

**Behavior:**
- Commands are **optional** (auto-discovery works without them)
- Commands give **immediate control** (bypass auto-extraction delay)
- Commands are **natural language** (not rigid syntax)
- Confirmation is **brief** (1-2 lines max)

**Success Criteria:**
- Commands work in any chat context
- Natural phrasing detected ("Remember", "Forget", "What do you remember")
- Confirmation doesn't interrupt workflow
- <20% of facts created via commands (auto-discovery is primary)

**Technical Requirements:**
- Detect command patterns in user input (LLM or regex)
- Map to tool calls (memory_create, memory_delete, memory_search)
- Confirm action in response
- Flag session for skip-extraction if requested

---

## Chat History Reference (Full Conversations)

Unlike memories (extracted facts), chat history reference allows AI to search and retrieve **full past conversations**. This enables users to say "Remember when we discussed X?" and get context from the actual conversation, not just extracted facts.

### F1.7: Chat History Storage
**Store all project conversations in searchable format for later reference**

**User Experience:**
```
[User has had 10 chat sessions in "Q1 Board Deck" project over 2 weeks]

User: "Remember that conversation where we decided on the
       metrics slide structure?"

AI (internal):
<tool_use>
  <name>conversation_search</name>
  <parameters>
    <query>metrics slide structure decision</query>
    <project_id>project-123</project_id>
  </parameters>
</tool_use>

Result: Found conversation from Jan 15 with relevant discussion

AI: "Yes! On January 15th we discussed the metrics slide and
     decided to lead with YoY growth (40%), then Series B
     announcement, then market expansion. Here's that exchange:

     [Shows relevant snippet from past conversation]

     Would you like to continue from there?"
```

**What Gets Stored:**
- Full conversation transcripts (user + AI messages)
- Timestamps for each message
- Project/workspace association
- Session boundaries (when conversation started/ended)

**Storage Scope:**
- **Project conversations** - All chats within a specific project
- **Cross-project search** - Can search across workspace if needed
- **User-only** - Only user's own conversations (not team in Phase 1)

**Retention:**
- Default: 90 days of conversation history
- User can clear history anytime
- Respects "Chat History Reference Toggle" (F1.11)

**Success Criteria:**
- Conversations searchable within 1 second
- Full transcript retrievable
- Project isolation maintained
- Storage efficient (deduplicated, compressed)

**Technical Requirements:**
- Database table: `conversation_history` (project_id, messages, timestamps)
- Full-text search index (gin/tsvector for PostgreSQL)
- Compression for older conversations
- Cleanup job for expired history

---

### F1.8: Chat History Search Tool
**AI searches past conversations on-demand via `conversation_search` tool**

**Tool Definition:**
```typescript
conversation_search(
  query: string,           // What to search for
  project_id?: string,     // Limit to specific project
  workspace_id?: string,   // Or search entire workspace
  date_range?: {           // Optional time filter
    start: Date,
    end: Date
  },
  limit?: number           // Max results (default: 5)
): ConversationSnippet[]
```

**Returns:**
```typescript
interface ConversationSnippet {
  conversation_id: string;
  project_id: string;
  project_name: string;
  date: Date;
  snippet: string;          // Relevant excerpt (500 chars max)
  full_context_available: boolean;
  relevance_score: number;
}
```

**When AI Uses This Tool:**
- User asks "Remember when we..." or "What did we discuss about..."
- User references past work without specifics
- AI needs context from previous sessions
- User asks to continue from a past conversation

**Example Tool Calls:**
```
User: "What did we decide about the color scheme?"

AI (internal):
<tool_use>
  <name>conversation_search</name>
  <parameters>
    <query>color scheme decision</query>
    <project_id>project-123</project_id>
  </parameters>
</tool_use>

Results:
[
  {
    conversation_id: "conv-456",
    date: "2026-01-18",
    snippet: "...decided to use the blue accent (#0066CC) for headers and green (#00AA55) for CTAs...",
    relevance_score: 0.92
  }
]
```

**NOT Pre-Loaded:**
- Conversation history isn't in every prompt
- AI searches on-demand (like memory_search)
- Only loads relevant snippets
- Saves tokens significantly

**Success Criteria:**
- Search returns relevant results 90%+ of time
- Tool call completes in <500ms
- Results ranked by relevance
- User doesn't notice overhead

**Technical Requirements:**
- Full-text search with ranking
- Tool handler in agent executor
- Result summarization (show relevant snippet, not full transcript)
- Optional: Vector search for semantic matching (Phase 2)

---

### F1.9: Conversation Context Loading
**Load relevant conversation snippets into context when needed**

**User Experience:**
```
User: "Let's continue where we left off on the product roadmap"

AI (internal):
1. <tool: conversation_search("product roadmap")>
2. Found: Last session ended discussing Q2 priorities
3. Loads summary of that conversation into context

AI: "Last time we were working on the product roadmap. We had
     finalized Q1 priorities and were starting to discuss Q2.
     You mentioned wanting to focus on:
     1. Mobile app launch
     2. Enterprise tier
     3. Integration partnerships

     Should we continue from there?"
```

**How It Works:**
1. AI identifies need for past context
2. Searches conversation history
3. Retrieves relevant snippet(s)
4. Loads into system prompt context
5. Responds with awareness of past work

**Context Loading Strategy:**
- **Summary mode** (default): Load condensed summary of past conversation
- **Full mode** (if user asks): Load complete relevant exchange
- **Token budget**: Max 2000 tokens from conversation history per request

**Difference from Memories:**
| Memories (Facts) | Conversation History |
|------------------|---------------------|
| Extracted preferences | Full dialogue |
| "CEO prefers 3 bullets" | "We discussed metrics and you said..." |
| Always relevant | Situationally relevant |
| Compact (50 tokens) | Verbose (500+ tokens) |

**When to Use Each:**
- **Memories**: Applying preferences, constraints, patterns
- **Conversation History**: Continuing past work, referencing specific discussions

**Success Criteria:**
- AI naturally references past conversations when relevant
- Context loading doesn't exceed token budget
- User feels continuity across sessions
- No redundant loading (respects memories for preferences)

**Technical Requirements:**
- Conversation summarization (LLM call to condense)
- Token counting before loading
- Integration with memory_search (check both sources)
- Context prioritization (memories + relevant history)

---

## Privacy Controls

Users must have full control over what Canvas remembers and references. These controls match ChatGPT's privacy model.

### F1.10: Memory Collection Toggle
**Enable/disable automatic fact extraction (user controls data collection)**

**User Experience:**
```
Settings > Memory:

┌────────────────────────────────────────────────────────────┐
│ Reference saved memories                         [●━━━]    │
│ Let Canvas save and use memories when responding           │
└────────────────────────────────────────────────────────────┘

[Toggle OFF]

┌────────────────────────────────────────────────────────────┐
│ Reference saved memories                         [━━━○]    │
│ Let Canvas save and use memories when responding           │
│                                                            │
│ ⚠️ Memory collection is paused. Canvas won't save new      │
│ memories or use existing ones.                             │
└────────────────────────────────────────────────────────────┘
```

**Behavior When OFF:**
- ❌ No new facts extracted from conversations
- ❌ Existing memories not used in responses
- ✅ Existing memories preserved (not deleted)
- ✅ Can still manually view/delete memories in Settings
- ✅ Chat history reference still works (separate toggle)

**Behavior When ON (default):**
- ✅ Facts extracted automatically after sessions
- ✅ Memories used to personalize responses
- ✅ Full memory system active

**Why This Matters:**
- Privacy: Users control their data
- Temporary disable: "Don't learn from this project"
- Testing: See how Canvas works without memory
- Compliance: Some users may need to disable for work

**Success Criteria:**
- Toggle instantly stops collection
- Existing memories preserved when off
- Clear indication of current state
- No confusion between memory and chat history toggles

**Technical Requirements:**
- User preference: `memory_collection_enabled: boolean`
- Check before fact extraction
- Check before memory_search tool calls
- UI toggle in Settings

---

### F1.11: Chat History Reference Toggle
**Enable/disable AI's ability to reference past conversations**

**User Experience:**
```
Settings > Memory:

┌────────────────────────────────────────────────────────────┐
│ Reference chat history                           [●━━━]    │
│ Let Canvas reference all previous conversations when       │
│ responding                                                 │
└────────────────────────────────────────────────────────────┘

[Toggle OFF]

┌────────────────────────────────────────────────────────────┐
│ Reference chat history                           [━━━○]    │
│ Let Canvas reference all previous conversations when       │
│ responding                                                 │
│                                                            │
│ ⚠️ Chat history reference is disabled. Canvas won't        │
│ search past conversations.                                 │
└────────────────────────────────────────────────────────────┘
```

**Behavior When OFF:**
- ❌ AI won't use conversation_search tool
- ❌ "Remember when we..." won't work
- ✅ Chat history still stored (for user's own reference)
- ✅ Memory system still works (separate toggle)
- ✅ Current conversation context still works

**Behavior When ON (default):**
- ✅ AI can search past conversations
- ✅ "Remember when we discussed..." works
- ✅ Full conversation continuity

**Why Separate from Memory Toggle:**
- Different data types (facts vs full conversations)
- Different privacy concerns (extracted info vs full dialogue)
- User may want one but not the other
- Matches ChatGPT's two-toggle model

**Success Criteria:**
- Clear distinction from memory toggle
- Instant effect when toggled
- Stored history preserved when off
- User understands what each toggle controls

**Technical Requirements:**
- User preference: `chat_history_reference_enabled: boolean`
- Check before conversation_search tool calls
- Continue storing history even when off (user may want later)
- UI toggle in Settings

---

### F1.12: Memory Settings UI
**Settings page with toggles, "Manage" button, and privacy info**

**User Experience:**
```
Settings > Memory:

┌────────────────────────────────────────────────────────────┐
│ Memory ⓘ                                       [Manage]    │
├────────────────────────────────────────────────────────────┤
│                                                            │
│ Reference saved memories                         [●━━━]    │
│ Let Canvas save and use memories when responding           │
│                                                            │
│ Reference chat history                           [●━━━]    │
│ Let Canvas reference all previous conversations when       │
│ responding                                                 │
│                                                            │
│ ───────────────────────────────────────────────────────── │
│                                                            │
│ Canvas may use Memory to personalize your experience.      │
│ Learn more                                                 │
│                                                            │
└────────────────────────────────────────────────────────────┘

[Click "Manage" button → Opens Memory Management UI (F1.5)]
```

**UI Components:**

1. **Header with Help**
   - "Memory" title
   - ⓘ info icon (hover for explanation)
   - "Manage" button (opens full memory list)

2. **Reference Saved Memories Toggle**
   - Toggle switch (on by default)
   - Description text
   - Warning when off

3. **Reference Chat History Toggle**
   - Toggle switch (on by default)
   - Description text
   - Warning when off

4. **Privacy Footer**
   - Brief explanation of how Memory is used
   - "Learn more" link to documentation

**Manage Button → Opens (F1.5 UI):**
```
┌────────────────────────────────────────────────────────────┐
│ Manage Memories                                       [×]  │
├────────────────────────────────────────────────────────────┤
│                                                            │
│ YOUR MEMORIES (applies everywhere)                   3     │
│ • EA supporting C-suite executives                   [×]   │
│ • Prefers Inter font                                 [×]   │
│ • Conversational but professional tone               [×]   │
│                                                            │
│ WORKSPACE: CEO Board Materials                       2     │
│ • CEO prefers 3 bullets max per slide                [×]   │
│ • Board deck: Metrics→Product→Strategy               [×]   │
│                                                            │
│                               [Clear all memories]         │
└────────────────────────────────────────────────────────────┘
```

**Success Criteria:**
- Matches ChatGPT's familiar pattern
- Both toggles visible and clear
- "Manage" opens memory list
- Privacy information accessible
- Mobile-responsive

**Technical Requirements:**
- Settings page component
- Two toggle components with state
- Navigation to memory management modal
- Help tooltip/modal for ⓘ icon
- "Learn more" link to docs

---

## Token Efficiency (Cross-Cutting Concern)

### Silent Token Efficiency
**Memory system reduces token usage 70-90% vs full-context**

**Problem It Solves:**
```
Without memory (full context every message):
User has 10 projects with 50 messages each = 500 messages

Every new message loads ALL 500 messages:
500 messages × 200 tokens avg = 100,000 input tokens per request
10 requests = 1,000,000 tokens

Cost: ~$3.00 per 10 messages
```

**With memory (progressive discovery):**
```
User has 10 projects with 50 facts extracted total

AI searches and loads relevant facts only:
Search returns 15 facts × 50 tokens = 750 input tokens per request
10 requests = 7,500 tokens

Cost: ~$0.02 per 10 messages

Savings: 99% token reduction, 99% cost reduction
```

**How It Works:**
1. Chat history converted to discrete facts (50 facts from 500 messages)
2. AI searches facts, not full history (15 relevant facts loaded)
3. Only relevant context in prompt (750 tokens vs 100,000 tokens)
4. Massive efficiency gain (99% reduction)

**Trade-off:**
- ✅ Huge cost/token savings
- ✅ Faster responses (less context to process)
- ⚠️ Potential loss of nuance (facts vs full conversation)
- ⚠️ Extraction accuracy matters (bad facts = bad memory)

**Success Criteria:**
- Token usage <10% of full-context baseline
- Response latency <2 seconds (vs 5+ with full context)
- No user complaints about "AI forgot important details"
- Extraction accuracy >90% (minimal information loss)

**Technical Requirements:**
- Fact extraction quality (high accuracy)
- Efficient search (fast retrieval)
- Token tracking (measure savings)
- Fallback to chat history if needed (safety valve)

---

## Phase 2: Intelligence Layer

**Goal:** AI gets smarter from patterns, usage, and learning

### F2.1: Usage Tracking & Smart Ranking
**Track which facts are useful and rank search accordingly**

**User Experience:**
```
Settings > Memory:

YOUR MEMORIES:

• CEO prefers 3 bullets max               [×]
  Created: Jan 10 • Used: 23 times ← High usage

• Prefers Inter font                      [×]
  Created: Jan 12 • Used: 8 times

• Use Anthropic Claude for AI tasks       [×]
  Created: Jan 18 • Used: 1 time ← Low usage
```

**Ranking Algorithm:**
```typescript
priority_score =
  (use_count / 10) * 0.4 +              // Usage weight: 40%
  (recency_days / 90) * 0.3 +           // Recency weight: 30%
  (metadata.confidence) * 0.3;           // Confidence weight: 30%
```

**Search Behavior:**
```
User asks: "Create presentation for CEO"

AI searches: "CEO presentation"
Results (ranked by priority):
1. "CEO prefers 3 bullets" (23 uses, recent, 0.95 confidence) → Score: 0.92
2. "Board deck structure: Metrics→..." (10 uses, older, 0.90 confidence) → Score: 0.65
3. "CEO email signature format" (2 uses, old, 0.85 confidence) → Score: 0.30

AI loads top 2, ignores #3 (low relevance)
```

**Benefits:**
- High-value facts surface first
- Stale facts sink to bottom
- Relevance improves over time
- Efficient (only load top-N)

**Success Criteria:**
- Most-used facts rank higher in search
- Search relevance improves 20-30% vs chronological
- Low-usage facts don't pollute context
- User doesn't complain "AI used wrong preference"

**Technical Requirements:**
- Track `use_count` and `last_used` on every search
- Ranking algorithm in search query
- `ORDER BY (use_count * 0.4 + ...) DESC`

---

### F2.2: Memory Decay (Automatic Cleanup)
**Remove stale, unused facts automatically**

**User Experience:**
```
[90 days after fact creation, if unused]

Notification:
"🧹 Memory cleanup:

Removed 3 unused memories from 90+ days ago:
• 'Use blue accent color #0066CC' (used 0 times)
• 'Target audience: College students' (used 1 time)
• 'Include emoji in titles' (used 0 times)

[View removed] [Undo cleanup]"
```

**Cleanup Rules:**
```
Delete facts where ALL of:
- last_used < 90 days ago (or never used)
- use_count < 3
- confidence < 0.8

Soft delete:
- Keep for 30 days (undo window)
- Permanently delete after 30 days
```

**Undo Window:**
```
Settings > Memory > Recently Removed (30 days):

• CEO prefers 3 bullets max                [Restore]
  Removed: 3 days ago • Permanent delete in: 27 days
```

**Why Cleanup Matters:**
1. **Prevents memory pollution** - Old preferences don't linger
2. **Reduces token usage** - Fewer facts to search
3. **Maintains relevance** - Only active facts remain
4. **Storage efficiency** - Scales better over time

**Success Criteria:**
- Removes 5-10% of facts per quarter (right balance)
- Zero complaints "AI forgot important preference" (proves rules work)
- User can undo within 30 days (safety net)
- Memory stays <100 facts per user (lean)

**Technical Requirements:**
- Scheduled job (weekly cleanup)
- Soft delete: `UPDATE memory_facts SET deleted_at = NOW() WHERE ...`
- Restore: `UPDATE memory_facts SET deleted_at = NULL WHERE id = $1`
- Hard delete: `DELETE FROM memory_facts WHERE deleted_at < NOW() - INTERVAL '30 days'`

---

### F2.3: Pattern Suggestion (Passive Learning)
**AI suggests memories based on repeated behavior**

**User Experience:**
```
[User creates 3 CEO board decks with same structure]

Session 3 ends →

Notification:
"💡 I noticed a pattern:

You've started 3 CEO board decks with:
Metrics → Product → Strategy → Asks

Save this as a preference?

[Yes] [No thanks] [Don't suggest patterns]"

User clicks [Yes] →

AI: "✓ Saved to workspace: CEO Materials

     Future CEO decks will follow this structure."
```

**Detection Criteria:**
- **Frequency:** 3+ occurrences of same pattern
- **Consistency:** 70%+ of sessions (not every time)
- **Timeframe:** Across different sessions (not just one)
- **Confidence:** Pattern extraction >80% confidence

**Suggestion Limits:**
- Max 1 suggestion per day (avoid spam)
- User can disable: "Don't suggest patterns" (permanent opt-out)
- Only suggest after artifact completion (not mid-creation)

**Pattern Types Detected:**
```
Structure patterns:
- "Board decks start with Metrics slide" (3/3 decks)
- "Presentations have 3-slide intro" (5/6 presentations)

Style patterns:
- "Use Inter font" (8/10 documents)
- "Include YoY comparisons in metrics" (4/5 metrics slides)

Constraint patterns:
- "Use Jane Doe examples" (3/3 patient apps)
- "Strategic focus, not operational" (4/4 CEO decks)
```

**Success Criteria:**
- Patterns detected accurately (>80%)
- Suggestions feel helpful, not spammy
- Max 1 suggestion per day enforced
- User can disable permanently
- 50%+ suggestion acceptance rate (proves value)

**Technical Requirements:**
- Analyze session history for patterns (LLM)
- Detect repetition across sessions
- Suggestion UI (banner/modal with buttons)
- Opt-out preference storage
- Rate limiting (1/day per user)

---

### F2.4: Memory Clarification (Humble Learning)
**Ask for clarification when AI notices differences, let user explain naturally**

**User Experience (During Conversation):**
```
User: "Make this slide have 5 bullets"

Memory: "CEO prefers 3 bullets max"

AI: "I noticed this is different than what I remember
     (I thought CEO prefers 3 bullets max).

     Can you clarify?"

User: "Oh yeah, this deck is for detailed product review,
       so 5 bullets is fine. Keep the 3 bullet rule for
       regular board updates."

AI: "Got it! I'll use 5 bullets for this product review deck.

     [Uses 5 bullets, synthesizer later extracts:
      'Product review decks can have more detail (5 bullets)']"
```

**Approach: Curious, Not Accusatory**
- ❌ "This contradicts your memory"
- ❌ "Conflict detected"
- ✅ "I noticed this is different than what I remember"
- ✅ "Can you clarify?"
- ✅ "Help me understand..."

**AI's Job:**
1. Notice the difference
2. Ask for clarification
3. Listen to user's explanation
4. Let synthesizer extract updated understanding
5. Use the clarification immediately

**User's Job:**
- Explain naturally (no forced choice)
- Context is preserved in chat
- Synthesizer picks it up later

**Detection (Lightweight):**
```typescript
// During generation, if AI loaded a memory
async function checkForDifference(userRequest: string, loadedFacts: Fact[]) {
  // Simple LLM check: Does request differ from loaded facts?
  const check = await llm.analyze({
    prompt: `
    User request: "${userRequest}"
    What I remember: ${loadedFacts.map(f => f.fact).join(', ')}

    Does the user's request differ from what I remember?
    Return JSON: {
      isDifferent: bool,
      whichFact: string | null,
      shouldAskForClarification: bool
    }
    `
  });

  return check;
}
```

**When to Ask:**
- User request differs from loaded memory
- Difference is significant (not trivial)
- AI is uncertain which to follow

**When NOT to Ask:**
- User request aligns with memory (just apply it)
- Difference is minor (user's phrasing, not preference change)
- Context makes it obvious (no clarification needed)

**Success Criteria:**
- AI asks for clarification (not forces choice)
- User explains naturally in conversation
- Updated understanding extracted in next synthesis
- No accusatory language ("conflict", "contradiction")
- Feels collaborative, not defensive

**Limitations (Phase 1):**
- Only asks during active conversation (not proactive scanning)
- LLM-based detection (slower, ~$0.001 per check)
- User must explain in chat (can't just click button)

**Technical Requirements:**
- Difference detection during conversation
- Humble clarification prompt generation
- User's explanation preserved in chat history
- Synthesizer extracts updated fact later (not immediate update)

---

## Phase 3: Team Collaboration

**Goal:** Share institutional knowledge across team members

### F3.1: Workspace Team Memory
**Workspace-scoped facts shared across all team members**

**User Experience:**
```
[Lisa creates fact in "CEO Board Materials" workspace]

Lisa: "Remember: CEO prefers 3 bullets max"

AI: "✓ Saved to workspace: CEO Board Materials

     This will apply to all team members working in this workspace."

[New EA joins team]

New EA opens "CEO Board Materials" workspace →

AI: "Welcome to CEO Board Materials.

     💭 Team knowledge:
        • CEO prefers 3 bullets max per slide
        • Board deck structure: Metrics → Product → Strategy

     (Shared by Lisa)"
```

**Permissions:**
- Any team member can create workspace facts
- Any team member can delete **their own** workspace facts
- Workspace admins can delete **any** workspace fact
- All team members can **view** workspace facts

**Attribution:**
```
Settings > Memory > WORKSPACE: CEO Board Materials:

• CEO prefers 3 bullets max               [×]
  Created by: Lisa • Jan 10 • Used: 45 times (team-wide)
```

**Success Criteria:**
- New team members inherit workspace knowledge automatically
- Knowledge survives team turnover (Lisa leaves, facts remain)
- Clear attribution (know who created)
- Safe deletion (can't accidentally delete teammate's facts)

**Technical Requirements:**
- Team membership table (who's in workspace)
- Permission checks (can delete = creator OR admin)
- Creator attribution (created_by user_id)
- Team-wide use_count (aggregated across members)

---

### F3.2: Personal vs Team Priority
**Handle conflicts between personal preferences and team workspace**

**User Experience:**
```
[Lisa's personal workspace]

Personal memory: "I prefer detailed explanations with examples"

[Working in team workspace: CEO Board Materials]

Workspace memory: "CEO wants concise (3 bullets max)"

AI working in CEO workspace:
→ Applies: Workspace preference (3 bullets)
→ Ignores: Personal preference (detailed)
→ Silent: No announcement

User can ask: "Why did you use 3 bullets?"
AI: "Using CEO workspace preference (3 bullets max).
     Your personal preference (detailed) is overridden in this workspace.

     [Use my preference instead]"
```

**Priority Rules:**
1. Project-specific > Workspace > User
2. Team workspace > Personal workspace
3. More specific > Less specific

**Conflict Notification (Proactive - Optional):**
```
[First time working in team workspace]

AI: "💭 Note: Your personal preference (detailed explanations)
    differs from CEO workspace preference (concise, 3 bullets).

    I'll use workspace preference for CEO content.

    [OK, got it] [Use my preference instead]"
```

**Success Criteria:**
- Team preferences win in team context (no confusion)
- Personal preferences win in personal context
- User aware of override (transparency)
- Can override temporarily if needed

**Technical Requirements:**
- Priority sorting in search (scope hierarchy)
- Preference conflict detection
- Optional notification on first use
- Temporary override mechanism (session-level)

---

## Feature Summary by Phase

### Phase 1: Foundation (12 Features)

| # | Feature | Complexity | Dependencies | Value |
|---|---------|------------|--------------|-------|
| **Memory System (Facts)** |||||
| F1.1 | Automatic fact extraction | M | LLM, DB | Critical |
| F1.2 | Progressive discovery (tools) | M | DB, tool system | Critical |
| F1.3 | Three scopes | S | DB schema | Critical |
| F1.4 | Silent application | S | Tool system | High |
| F1.5 | Settings UI (deletion) | S | Frontend | High |
| F1.6 | Conversation commands | M | LLM, tools | Medium |
| **Chat History Reference** |||||
| F1.7 | Chat history storage | M | DB, full-text search | Critical |
| F1.8 | Chat history search tool | M | Tool system, search | Critical |
| F1.9 | Conversation context loading | S | LLM summarization | High |
| **Privacy Controls** |||||
| F1.10 | Memory collection toggle | S | User preferences | Critical |
| F1.11 | Chat history reference toggle | S | User preferences | Critical |
| F1.12 | Memory settings UI | S | Frontend | Critical |

**Deliverable:** AI learns from conversations AND can reference full past conversations; user has full privacy control via toggles

**Success metric:** Users stop repeating preferences AND can say "remember when we discussed X?"

---

### Phase 2: Intelligence (4 Features)

| # | Feature | Complexity | Dependencies | Value |
|---|---------|------------|--------------|-------|
| F2.1 | Usage tracking & ranking | S | DB columns | High |
| F2.2 | Memory decay & cleanup | M | Background jobs | Medium |
| F2.3 | Pattern suggestion | L | LLM analysis | High |
| F2.4 | Basic conflict detection | M | LLM checker | High |

**Deliverable:** AI learns patterns, suggests memories, handles conflicts, stays lean

**Success metric:** Memory quality improves over time (high usage facts, few deletions)

---

### Phase 3: Collaboration (2 Features)

| # | Feature | Complexity | Dependencies | Value |
|---|---------|------------|--------------|-------|
| F3.1 | Team workspace memory | M | Team system | Critical |
| F3.2 | Personal vs team priority | M | Conflict resolution | High |

**Deliverable:** Team knowledge compounds, new members inherit context

**Success metric:** Team productivity improves (no repeated explanations)

---

## Advanced Technologies (Gated by Feature Needs)

**See:** [TECHNOLOGY-ROADMAP.md](./TECHNOLOGY-ROADMAP.md) for detailed analysis

### Vector Search (pgvector) - Phase 2 Enhancement
**Enables semantic pattern detection and better search**

**Triggered by:**
- User has 50+ facts (keyword search becomes insufficient)
- F2.3 Pattern suggestion is active (needs clustering)
- User complaints: "AI didn't find relevant memory"

**Features it unlocks:**
- Semantic search ("concise" finds "3 bullets max")
- Pattern clustering (groups similar intents)
- Better ranking (meaning-based, not keyword-based)

**Cost:** Low ($0.00002/fact, 100ms overhead)
**ROI:** High (Mem0 reports 26% accuracy improvement)

---

### Graph Relationships - Phase 2+ Enhancement
**Enables conflict detection, consolidation, inference**

**Triggered by:**
- Users have duplicate facts (redundancy problem)
- F2.4 Conflict detection needs enhancement
- Memory consolidation becomes valuable

**Features it unlocks:**
- Advanced conflict detection (finds non-obvious contradictions)
- Memory consolidation (merge similar facts automatically)
- Inference (apply CEO pattern to CFO when missing)
- Smart decay (keep hub facts even if low usage)

**Cost:** Medium (2x storage, LLM calls per fact pair)
**ROI:** Medium (valuable but expensive)

---

### Memory Versioning - Phase 3 Requirement
**Enables undo, audit trail, trend detection**

**Triggered by:**
- Users request undo feature (>10% request rate)
- Phase 3 team collaboration starts (audit trail needed)
- Governance/compliance requirements

**Features it unlocks:**
- Undo deletion (30-day window)
- History view (see preference evolution)
- Audit trail (team governance: who changed what)
- Trend detection (preferences over time)

**Cost:** High (5x storage, soft delete complexity, UI)
**ROI:** Low for individuals, High for teams

---

## Success Criteria (Overall)

**Phase 1 Success:**
> User works across 5+ projects without repeating preferences

**Metrics:**
- 80%+ facts auto-discovered (not manual)
- 90%+ facts accurate (low deletion rate)
- Token usage <10% of full-context
- User satisfaction: "AI remembers me"

---

**Phase 2 Success:**
> AI suggests helpful patterns and keeps memory clean

**Metrics:**
- Pattern suggestion acceptance rate >50%
- Memory decay removes 5-10% facts/quarter
- Conflict detection catches 70%+ contradictions
- Search relevance improves 20-30%

---

**Phase 3 Success:**
> Teams share knowledge seamlessly

**Metrics:**
- New team members productive day 1 (inherit context)
- Knowledge survives turnover (facts persist after creator leaves)
- Zero workspace conflicts (priority rules work)
- Team satisfaction: "AI knows our standards"

---

## What We're NOT Building

**Explicitly excluded (forever):**
- ❌ Manual memory forms/CRUD (conversation-first)
- ❌ Categories/tags UI (flat list is simpler)
- ❌ Quality stars/badges (usage count is sufficient)
- ❌ Sidebar memory panel (Settings location better)
- ❌ Pre-loaded context (progressive discovery better)

**Strategically deferred (build when needed):**
- 🔮 Vector search → When 50+ facts or pattern detection starts
- 🔮 Graph relationships → When duplicate/conflict problems emerge
- 🔮 Memory versioning → When undo requested or team phase starts

---

## Change History

| Date | Author | Changes |
|------|--------|---------|
| 2026-01-21 | AI Assistant | Initial feature extraction |
| 2026-01-22 | AI Assistant | Complete rewrite: ChatGPT auto-discovery + Claude progressive discovery + conflict handling + technology roadmap |
| 2026-01-22 | Ken Chau | Added Chat History Reference features (F1.7-F1.9) and Privacy Controls (F1.10-F1.12) based on ChatGPT Memory UI analysis |
