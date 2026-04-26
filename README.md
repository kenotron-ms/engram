# engram

Personal memory system for AI agents. Rust-native, cross-platform, encrypted.

## What it is

Engram stores your life's knowledge as markdown files, extracts atomic facts into an encrypted database, provides hybrid full-text + semantic search, and syncs encrypted to cloud storage. Any AI harness — Amplifier, Claude Code, Cursor — can connect via the MCP server or CLI.

## Install

```bash
cargo install --path crates/engram-cli
```

Or build from source:
```bash
cargo build --release
# binary at target/release/engram
```

## Quick Start

```bash
engram init                          # first-time setup, creates ~/.engram/
engram status                        # vault, index, store, sync status
engram index                         # index vault for search
engram search "Sofia dietary needs"  # hybrid semantic + full-text search
```

## CLI Reference

### Memory & Search
```bash
engram index [--vault PATH] [--force]    # index vault content (full-text + vector)
engram search "<query>" [--limit N]      # hybrid search (BM25 + semantic + RRF)
engram store                             # write a memory to the encrypted store
engram recall                            # recall stored memories
engram observe <session-path>            # extract facts from session transcript via LLM
engram load [--format context]           # emit vault context for AI harness injection
```

### Cloud Sync
```bash
engram auth add s3 --endpoint <url> --bucket <name>   # S3-compatible (R2, MinIO, B2, AWS)
engram auth add onedrive                               # Microsoft OneDrive (OAuth2)
engram auth add azure --account <name> --container ... # Azure Blob Storage
engram auth add gdrive --bucket <name> --key-file ...  # Google Drive / GCS
engram auth list
engram auth remove <backend>
engram sync [--backend <name>]           # encrypt + push all vault files to cloud
```

### Service Management
```bash
engram daemon        # start background observer + MCP stdio server
engram install       # register as system service (launchd on macOS, systemd on Linux)
engram uninstall     # remove system service
engram doctor        # diagnose vault, index, keychain, and service health
```

## Architecture

```
engram-core      vault I/O · AES-256/XChaCha20-Poly1305 · Argon2id KDF
                 platform keychain (iOS Keychain / Android Keystore / macOS Keychain /
                 Windows Credential Manager / libsecret)
                 SQLCipher encrypted memory store (atomic facts, CRUD, temporal)

engram-search    tantivy BM25 full-text search
                 fastembed AllMiniLML6V2 (384-dim embeddings, runs locally)
                 sqlite-vec KNN vector search
                 Hybrid RRF ranking · incremental content-hash reindexing

engram-sync      SyncBackend trait (S3 / Azure Blob / GCS / OneDrive)
                 object_store crate (S3-compatible covers Cloudflare R2, MinIO, AWS, Backblaze B2)
                 OneDrive via Microsoft Graph REST API
                 Client-side XChaCha20-Poly1305 encryption before every upload
                 OAuth2 + platform keychain for credential storage
```

## Encryption Model

**Local vault** — plaintext files on your OS-encrypted drive (FileVault, BitLocker, iOS, Android encryption). Trust the OS.

**Cloud sync** — always client-side encrypted before upload. The remote storage backend never sees plaintext. Key is derived via Argon2id from your password and stored in the platform keychain.

**Memory store** (`~/.engram/memory.db`) — always SQLCipher encrypted (AES-256), even without cloud sync.

## Storage Layout

```
~/.lifeos/memory/          vault — markdown files, git-tracked, human-readable
  .engram/
    index/                 tantivy full-text index (rebuilt locally, not synced)
    vectors.db             sqlite-vec vector index (rebuilt locally, not synced)
    memory.db              SQLCipher encrypted memory store (rebuilt locally)

Cloud backend              only encrypted ciphertext (never plaintext)
```

## Mobile (UniFFI)

`engram-core` generates native bindings for iOS (Swift), Android (Kotlin), and Python via [UniFFI](https://mozilla.github.io/uniffi-rs/).

See [`crates/engram-core/bindings/`](crates/engram-core/bindings/) for generated files and usage examples.

```swift
// iOS Swift
let salt = generateSaltFfi()
let keyBytes = try deriveKey(password: Array("password".utf8), salt: salt)
let store = try MemoryStoreHandle(dbPath: "~/.engram/memory.db", keyBytes: keyBytes)
try store.insertMemory(entity: "Sofia", attribute: "dietary", value: "vegetarian", source: nil)
```

```kotlin
// Android Kotlin
val keyBytes = deriveKey("password".toByteArray().toList(), generateSaltFfi())
val store = MemoryStoreHandle(dbPath = "memory.db", keyBytes = keyBytes)
store.insertMemory("Sofia", "dietary", "vegetarian", null)
```

## AI Harness Integration

### MCP Server
Run `engram daemon` — it starts an MCP stdio server exposing `memory_search`, `memory_load`, and `memory_status` tools. Compatible with Claude Code, Cursor, Windsurf, and any MCP client.

Configure in your harness:
```json
{
  "mcpServers": {
    "engram": {
      "command": "engram",
      "args": ["daemon"]
    }
  }
}
```

### Amplifier
See [`modules/`](modules/) for Amplifier hook and tool modules:
- `hook-memory-context` — injects vault context at session start
- `hook-memory-observe` — processes session transcripts at session end
- `tool-memory` — exposes search, load, and status as agent tools

## Development

```bash
cargo build --workspace
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
cargo fmt --all

# Run engram from source
cargo run --bin engram -- status
```

## License

MIT
