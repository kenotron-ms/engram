# Source Intake Protocol

> **Use when:** Content arrives in a format you cannot read directly (PDF, image, audio, video, archive, etc.)

```
NO CONTENT PROCESSING WITHOUT FORMAT VALIDATION
```

**Violating the letter of this protocol is violating the spirit of this protocol.**

---

## What is Source Intake?

Source intake handles incoming content that isn't plain text, converting it to readable form before applying knowledge-extraction.md.

**Input:** URL, file path, or reference to content in non-readable format
**Output:** Clean text + metadata, ready for knowledge extraction

---

## When to Use This Protocol

| Use when... | Example |
|-------------|---------|
| User provides PDF URL | Research paper, technical document |
| User provides image | Screenshot, diagram, whiteboard photo |
| User provides audio/video | Lecture recording, meeting transcript |
| User provides Office doc | Word doc, PowerPoint, Excel |
| User provides archive | ZIP, TAR with multiple files |
| User provides URL to rich content | YouTube video, paywalled article |

**Don't use for:** Plain text URLs (use web_fetch), markdown files (use read_file), code files (use read_file)

---

## The Intake Process

| Step | Action |
|------|--------|
| 1 | **Detect format**: What type of content is this? |
| 2 | **Choose extraction method**: What tool/agent can extract it? |
| 3 | **Extract content**: Delegate to appropriate tool/agent |
| 4 | **Validate extraction**: Did we get usable text? |
| 5 | **Prepare metadata**: Source, format, extraction method, confidence |
| 6 | **Handoff to knowledge-extraction.md**: Now apply knowledge protocols |

---

## Format Detection

### By Extension

| Extension | Format | Extraction Method |
|-----------|--------|-------------------|
| `.pdf` | PDF document | Delegate to foundation:web-research for alt sources, or escalate |
| `.png`, `.jpg`, `.jpeg` | Image | OCR extraction (escalate - no current tool) |
| `.mp3`, `.wav`, `.m4a` | Audio | Transcription (escalate - no current tool) |
| `.mp4`, `.mov`, `.avi` | Video | Transcription (escalate - no current tool) |
| `.docx`, `.doc` | Word | Text extraction (escalate - no current tool) |
| `.pptx`, `.ppt` | PowerPoint | Text + slide extraction (escalate - no current tool) |
| `.xlsx`, `.xls` | Excel | Structured data extraction (escalate - no current tool) |
| `.zip`, `.tar`, `.gz` | Archive | Recursive extraction (escalate - no current tool) |

### By URL Pattern

| Pattern | Format | Extraction Method |
|---------|--------|-------------------|
| `youtube.com/watch` | Video | Delegate to foundation:web-research for transcript search |
| `*.pdf` | PDF | Try web_fetch, then search for text version |
| `github.com/.../blob/` | Code/Text | Use web_fetch with raw.githubusercontent.com |

---

## Extraction Methods

### Method 1: Search for Alternative Format

**When:** PDF available on web, might have text version

```bash
1. Delegate to foundation:web-research
2. Search for: "{paper title} text version" OR "{paper title} markdown"
3. If found: Extract text from alternative source
4. If not found: Proceed to Method 2
```

### Method 2: User-Provided Extraction

**When:** No automated extraction available

```
1. Explain format limitation clearly
2. Ask user to provide text version or key excerpts
3. If user provides: Continue to knowledge-extraction.md
4. If user cannot: Document the gap, suggest future capability
```

### Method 3: Future - Automated Extraction

**When:** Tool/agent capability exists

```
1. Delegate to appropriate extraction agent
2. Validate output quality
3. Proceed to knowledge-extraction.md
```

---

## Current Capabilities (2026-02-19)

| Format | Status | Method |
|--------|--------|--------|
| Plain text URLs | ✅ Available | `web_fetch` tool |
| GitHub files | ✅ Available | `web_fetch` with raw.githubusercontent.com |
| Markdown files | ✅ Available | `read_file` tool |
| PDF files | ⚠️ Limited | Search for text version via foundation:web-research, or escalate to user |
| Images | ❌ Not available | Escalate to user |
| Audio/Video | ❌ Not available | Search for transcript, or escalate to user |
| Office docs | ❌ Not available | Escalate to user |
| Archives | ❌ Not available | Escalate to user |

---

## Validation Criteria

Before handing off to knowledge-extraction.md:

| Check | Requirement |
|-------|-------------|
| **Readable text** | Extracted content is plain text or markdown |
| **Sufficient length** | Content is substantial enough to extract knowledge from |
| **Coherent structure** | Not garbled, corrupted, or nonsensical |
| **Source metadata** | Original URL/path, format, extraction method recorded |

---

## Metadata Format

When extraction succeeds, provide metadata:

```yaml
source:
  original_url: "https://github.com/microsoft/Mnemis/blob/main/Mnemis_paper.pdf"
  format: "pdf"
  extraction_method: "alternative_text_source"
  alternative_url: "https://arxiv.org/abs/xxxx" # if found
  extraction_date: "2026-02-19T21:50:00Z"
  confidence: "high" # high/medium/low based on extraction quality
```

---

## Escalation to User

When automated extraction fails:

**Template:**
```
I cannot directly read {format} files yet. 

To extract knowledge from this source, I have these options:

1. [If applicable] Search for text version online
2. You provide key excerpts or text version
3. [Future] Build automated {format} extraction capability

Which approach would you prefer?
```

**Don't:**
- Silently give up
- Pretend you read it
- Make up content

**Do:**
- Explain limitation clearly
- Offer concrete options
- Document the gap for future improvement

---

## Gate Function

```
BEFORE proceeding to knowledge-extraction.md:
  1. CHECK: Format detected correctly?
  2. CHECK: Extraction method chosen?
  3. CHECK: Extraction attempted?
  4. CHECK: Output validated as readable text?
  5. CHECK: Source metadata captured?
  If ANY check fails: Do not proceed to knowledge extraction.
```

---

## Three-Failure Escalation

If extraction fails 3 times for the same source (e.g., trying different methods, all failing):

1. State what you attempted and what failed each time
2. Present user with clear options (provide text, skip, build capability)
3. Do not retry without new information

Repeated extraction failures indicate either:
- Format genuinely not supported → need new capability
- Source is corrupted/protected → need alternative source
- User input required → escalate cleanly

---

## Red Flags

If you catch yourself thinking:
- "I'll just tell the user what I would have extracted"
- "The title is enough, I don't need the full content"
- "I can guess what a PDF about X probably says"
- "I'll skip this and move on"

**All of these mean: STOP. Follow the escalation procedure.**

---

## Common Rationalizations

| Excuse | Reality |
|--------|---------|
| "I can infer what the paper says from the title" | You need actual content. Search for alternatives or escalate. |
| "This format is too hard to extract" | That's why we have this protocol. Follow the methods. |
| "The user won't mind if I skip this" | The user asked for it. Try extraction methods, then escalate clearly. |
| "I'll just summarize based on what I know" | Knowledge extraction requires source content, not guesses. |

---

## Anti-Patterns

| Don't | Do |
|-------|-----|
| Silently fail and move on | Attempt extraction methods, then escalate clearly |
| Make up content you "think" is in the source | Extract actual content or ask user for it |
| Skip formats you don't support | Follow escalation procedure |
| Pretend to have read unreadable formats | Be transparent about limitations |

---

## Success Metrics

**You're doing this well when:**
- ✅ All extractable formats get extracted
- ✅ Clear escalation when extraction not possible
- ✅ User understands limitations and options
- ✅ Source metadata preserved for future reference

**You need to improve when:**
- ❌ Silently skipping non-text formats
- ❌ Making up content from unread sources
- ❌ User confused about why you can't read something
- ❌ No record of what extraction was attempted

---

## Future Capabilities

As new extraction tools/agents become available, update this protocol:

**Planned capabilities:**
- PDF text extraction agent
- OCR for images
- Audio/video transcription
- Office document parser
- Archive unpacker

**Update process:**
1. When new capability added, update "Current Capabilities" table
2. Add to "Extraction Methods" section
3. Update examples
