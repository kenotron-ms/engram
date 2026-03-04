# SPEC-EMBEDDINGS: Canvas Memory Embeddings Specification

**Version:** 0.1.0
**Status:** Draft
**Last Updated:** 2026-03-03

---

## 1. Overview

Canvas Memory uses dense vector embeddings for semantic similarity search. Embeddings convert memory content into fixed-dimensional float vectors stored in sqlite-vec's `vec0` virtual table, enabling KNN (k-nearest-neighbor) retrieval.

The embedding subsystem must support:

1. **Multiple providers** — OpenAI (default), Azure OpenAI, local models via Ollama.
2. **Asymmetric search** — Queries and memories are embedded differently.
3. **Batch operations** — Efficient bulk embedding for import/migration.
4. **Caching** — Avoid re-embedding unchanged content.
5. **Dimensionality flexibility** — Handle 768-dim local models alongside 1536-dim cloud models.

---

## 2. Model Selection

### 2.1 Default Model: `text-embedding-3-small`

| Property | Value |
|----------|-------|
| Provider | OpenAI |
| Model ID | `text-embedding-3-small` |
| Native dimensions | 1536 |
| Max input tokens | 8191 |
| Cost (as of 2026) | $0.02 / 1M tokens |
| MRL support | Yes (can output 512 or 256 dims) |
| Normalization | L2-normalized output vectors |

**Why this model:**

- **Cost-effective.** 10× cheaper than `text-embedding-3-large` with adequate quality for personal memory recall (not web-scale retrieval).
- **Quality.** Outperforms `text-embedding-ada-002` on MTEB benchmarks across retrieval, classification, and clustering.
- **MRL support.** Matryoshka Representation Learning allows dimensionality reduction at query time without retraining.
- **Asymmetric capability.** The model handles the query-document asymmetry well because it was trained on question-answer pairs in addition to document similarity.

### 2.2 Alternative: `text-embedding-3-large` with MRL Compression

| Property | Value |
|----------|-------|
| Provider | OpenAI |
| Model ID | `text-embedding-3-large` |
| Native dimensions | 3072 |
| Compressed dimensions | 1536 (via `dimensions` parameter) |
| Max input tokens | 8191 |
| Cost | $0.13 / 1M tokens |

When to use: If the user has a large memory corpus (> 10K memories) and retrieval precision matters more than cost. The `dimensions=1536` parameter truncates the output to 1536 dimensions using MRL, giving better quality than `3-small` at the same storage cost.

```python
# MRL compression: request 1536 dims from the 3072-dim model
response = client.embeddings.create(
    model="text-embedding-3-large",
    input=text,
    dimensions=1536  # MRL truncation
)
```

### 2.3 Alternative: Local Models via Ollama

| Property | Value |
|----------|-------|
| Provider | Ollama (local) |
| Model ID | `nomic-embed-text` |
| Native dimensions | 768 |
| Stored dimensions | 1536 (zero-padded) |
| Max input tokens | 8192 |
| Cost | Free (local compute) |

When to use: Offline environments, privacy-sensitive use cases, or when the user doesn't want to send content to OpenAI.

**Dimensionality handling:** See Section 8 for how 768-dim vectors are stored in a 1536-dim `vec0` table.

### 2.4 Model Comparison

| Model | Dims | Quality (MTEB Avg) | Latency (single) | Cost/1M tokens | Offline |
|-------|------|---------------------|-------------------|-----------------|---------|
| `text-embedding-3-small` | 1536 | 62.3 | ~100 ms | $0.02 | No |
| `text-embedding-3-large` @ 1536 | 1536 | 64.6 | ~150 ms | $0.13 | No |
| `nomic-embed-text` (Ollama) | 768 → 1536 | 55.7 | ~50 ms | Free | Yes |

---

## 3. Input Construction

### 3.1 Canonical Embedding Input Format

Every memory is embedded using a structured input format:

```python
def construct_embedding_input(
    content: str,
    content_type: str,
    summary: str | None = None
) -> str:
    """Construct the canonical embedding input for a memory.
    
    Format: "{content_type}: {summary}\n\n{content[:512]}"
    
    The content is truncated to 512 chars to keep the embedding
    focused on the core semantic content, not incidental details.
    """
    header = f"{content_type}: {summary}" if summary else f"{content_type}:"
    body = content[:512]
    return f"{header}\n\n{body}"
```

**Example inputs:**

```
# Preference memory
preference: User prefers composition over inheritance in TypeScript
for better testing and avoiding deep class hierarchies.

In our discussion about TypeScript design patterns, the user explained
that they always reach for composition first. They mentioned that deep
inheritance hierarchies make testing difficult and that interface-based
composition gives them more flexibility.

# Decision memory  
decision: Team chose PostgreSQL over DynamoDB for the payment service
because of ACID transaction requirements and existing team expertise.

After evaluating both options, the team decided on PostgreSQL for the
payment-service database. Key factors: need for multi-table transactions
(ACID), team's existing PostgreSQL expertise, and the requirement for
complex reporting queries that DynamoDB would struggle with.

# Skill memory
skill: User is proficient with Terraform for AWS infrastructure
provisioning, preferring modular workspace-based configurations.

The user demonstrated deep Terraform knowledge, including module
composition, workspace-based environment separation, and state
management with S3 backends. They prefer terragrunt for DRY
configurations across environments.
```

### 3.2 Format Rationale

Each component of the canonical format serves a specific purpose:

| Component | Purpose | Effect on Embedding |
|-----------|---------|---------------------|
| `content_type` prefix | **Clustering.** Memories of the same type cluster together in vector space. A `preference:` prefix creates a distinct region from `decision:` prefixes. | Improves recall precision when the agent knows what _type_ of memory it's looking for. |
| `summary` | **Semantic center.** The summary is the most informationally dense representation. Placing it first gives it outsized influence on the embedding direction. | The embedding vector points primarily toward the summary's semantic meaning. |
| `content[:512]` | **Contextual grounding.** Raw content provides specific terms, names, and details that the summary may abstract away. | Ensures the embedding captures concrete details for keyword-like semantic matching. |
| 512-char truncation | **Focus.** Long content dilutes the embedding with less relevant details. 512 chars captures the first ~100 words — typically the most important. | Prevents embedding drift from verbose content tails. |
| `\n\n` separator | **Segment boundary.** Transformer models are trained on paragraph-separated text. The double newline signals a semantic break between the summary and the body. | Allows the model to weight the segments appropriately. |

### 3.3 Token Budget

The embedding input must fit within the model's context window:

```python
MAX_EMBEDDING_TOKENS = 2048  # Conservative limit (model supports 8191)

def estimate_tokens(text: str) -> int:
    """Rough token estimate: 1 token ≈ 4 chars for English text."""
    return len(text) // 4

def construct_embedding_input_safe(
    content: str,
    content_type: str,
    summary: str | None = None
) -> str:
    """Construct embedding input, respecting token budget."""
    header = f"{content_type}: {summary}" if summary else f"{content_type}:"
    header_tokens = estimate_tokens(header)
    remaining_tokens = MAX_EMBEDDING_TOKENS - header_tokens - 10  # margin
    max_content_chars = min(512, remaining_tokens * 4)
    body = content[:max_content_chars]
    return f"{header}\n\n{body}"
```

---

## 4. Asymmetric Embedding

### 4.1 The Problem with Symmetric Search

In symmetric search, queries and documents are embedded with the same process. This works for document-to-document similarity but performs poorly for natural-language questions:

```
# Symmetric: low similarity despite semantic match
Query embedding:   "What testing framework?"
Memory embedding:  "preference: User prefers pytest with fixtures..."
# These sentences are structurally different → lower cosine similarity
```

### 4.2 The Asymmetric Approach

Memories are embedded as **statements** (declarative). Queries are embedded as **questions** (interrogative) that the memories would answer.

**Memory embedding** — uses the canonical format (Section 3.1):

```
preference: User prefers pytest with the fixtures pattern for all
Python testing, avoiding unittest-style classes.

The user consistently uses pytest in all projects. They rely heavily
on conftest.py fixtures for dependency injection and prefer function-
based tests over class-based. Mentioned that parametrize is their
most-used decorator.
```

**Query embedding** — the recall system transforms the query context into a question:

```python
def construct_query_input(query: str, context: str | None = None) -> str:
    """Construct the query-side embedding input.
    
    For asymmetric search, the query should be phrased as a question
    that the memory would answer. The raw agent context is transformed
    into an interrogative form.
    """
    if context:
        return f"Question: {query}\nContext: {context[:256]}"
    return f"Question: {query}"
```

**Example query inputs:**

```
# Agent is helping with test setup
Question: What testing framework and patterns does the user prefer?
Context: Setting up a new Python project, need to configure testing.

# Agent is making an architecture decision
Question: What database preferences or past decisions has the user made?
Context: Designing a new microservice that needs persistent storage.

# Agent is writing code
Question: What coding style and conventions does the user follow for Python?
Context: Implementing a REST API handler with error handling.
```

### 4.3 When Asymmetric Embedding Applies

| Operation | Embedding Style | Format |
|-----------|----------------|--------|
| Memory capture (insert) | **Document** (declarative) | `{content_type}: {summary}\n\n{content[:512]}` |
| Memory update (re-embed) | **Document** (declarative) | Same as capture |
| Recall query | **Query** (interrogative) | `Question: {query}\nContext: {context[:256]}` |
| Similar memory search | **Document** (declarative) | Use the source memory's embedding directly (no re-embedding) |

### 4.4 Query Transformation

The agent's raw recall context is often not in question form. The system transforms it:

```python
QUERY_TRANSFORM_PROMPT = """Transform the following recall context into a clear question 
that would be answered by a relevant memory. Keep it concise (1 sentence).

Recall context: {context}

Question:"""
```

For simple keyword queries, the transformation is skipped and the query is embedded as-is with the `Question:` prefix.

---

## 5. Embedding Pipeline

### 5.1 Capture Pipeline

```
Raw content
    │
    ▼
┌─────────────────────┐
│ Normalize content    │  Strip excess whitespace, normalize unicode
│                      │  (NFC normalization), truncate to 10K chars
└─────────┬───────────┘
          │
          ▼
┌─────────────────────┐
│ Generate summary     │  LLM call → 1-2 sentence inductive summary
│                      │  (may be async but completes before embed)
└─────────┬───────────┘
          │
          ▼
┌─────────────────────┐
│ Construct input      │  f"{content_type}: {summary}\n\n{content[:512]}"
│                      │  Enforce token budget (≤ 2048 tokens)
└─────────┬───────────┘
          │
          ▼
┌─────────────────────┐
│ Check cache          │  Key: (model_id, sha256(input_text))
│                      │  Hit → return cached embedding
└─────────┬───────────┘
          │ (cache miss)
          ▼
┌─────────────────────┐
│ Call provider        │  OpenAI / Azure / Ollama API call
│                      │  Returns float[] of dimension N
└─────────┬───────────┘
          │
          ▼
┌─────────────────────┐
│ Post-process         │  L2-normalize (if provider doesn't)
│                      │  Pad/truncate to target dimension (1536)
└─────────┬───────────┘
          │
          ▼
┌─────────────────────┐
│ Store in vec0        │  Serialize to bytes, INSERT into memory_vectors
│                      │  Update cache with (key → embedding)
└─────────┬───────────┘
          │
          ▼
    Return memory_id
```

### 5.2 Recall Pipeline

```
Query context (from agent)
    │
    ▼
┌─────────────────────┐
│ Transform query      │  If natural language: transform to question form
│                      │  If keywords: prefix with "Question: "
└─────────┬───────────┘
          │
          ▼
┌─────────────────────┐
│ Embed query          │  Same provider as DB embeddings
│                      │  (model mismatch = error, see Section 8)
└─────────┬───────────┘
          │
          ▼
┌─────────────────────┐
│ KNN search           │  SELECT FROM memory_vectors
│                      │  WHERE embedding MATCH ? AND k = ?
└─────────┬───────────┘
          │
          ▼
┌─────────────────────┐
│ Return candidates    │  List of (memory_id, distance) pairs
│                      │  Fed into hybrid search re-ranking
└─────────────────────┘
```

---

## 6. Provider Abstraction

### 6.1 Protocol Definition

```python
from abc import ABC, abstractmethod
from dataclasses import dataclass

@dataclass
class EmbeddingResult:
    """Result of an embedding operation."""
    embedding: list[float]    # The embedding vector
    model: str                # Model ID used
    dimensions: int           # Actual dimensions returned
    tokens_used: int          # Tokens consumed by this embedding
    cached: bool = False      # Whether this was a cache hit

class EmbeddingProvider(ABC):
    """Abstract base class for embedding providers.
    
    All providers must implement embed_texts() for batch embedding.
    Single-text embedding is provided as a convenience wrapper.
    """
    
    @property
    @abstractmethod
    def model_id(self) -> str:
        """The model identifier (e.g., 'text-embedding-3-small')."""
        ...
    
    @property
    @abstractmethod
    def dimensions(self) -> int:
        """The output dimensionality of this provider."""
        ...
    
    @property
    @abstractmethod
    def max_tokens(self) -> int:
        """Maximum input tokens per text."""
        ...
    
    @abstractmethod
    async def embed_texts(self, texts: list[str]) -> list[EmbeddingResult]:
        """Embed a batch of texts.
        
        Args:
            texts: List of texts to embed. Each must be ≤ max_tokens.
        
        Returns:
            List of EmbeddingResult, one per input text, in order.
        
        Raises:
            EmbeddingError: If the provider API call fails.
            TokenLimitError: If any text exceeds max_tokens.
        """
        ...
    
    async def embed_text(self, text: str) -> EmbeddingResult:
        """Embed a single text. Convenience wrapper around embed_texts."""
        results = await self.embed_texts([text])
        return results[0]
```

### 6.2 OpenAI Implementation

```python
from openai import AsyncOpenAI

class OpenAIEmbeddingProvider(EmbeddingProvider):
    """OpenAI embedding provider using the embeddings API."""
    
    def __init__(
        self,
        model: str = "text-embedding-3-small",
        dimensions: int | None = None,
        api_key: str | None = None,
    ):
        self._client = AsyncOpenAI(api_key=api_key)
        self._model = model
        self._dimensions = dimensions or self._default_dimensions(model)
        self._request_dimensions = dimensions  # None = use native
    
    @staticmethod
    def _default_dimensions(model: str) -> int:
        return {
            "text-embedding-3-small": 1536,
            "text-embedding-3-large": 3072,
            "text-embedding-ada-002": 1536,
        }.get(model, 1536)
    
    @property
    def model_id(self) -> str:
        return self._model
    
    @property
    def dimensions(self) -> int:
        return self._dimensions
    
    @property
    def max_tokens(self) -> int:
        return 8191
    
    async def embed_texts(self, texts: list[str]) -> list[EmbeddingResult]:
        kwargs = {"model": self._model, "input": texts}
        if self._request_dimensions is not None:
            kwargs["dimensions"] = self._request_dimensions
        
        response = await self._client.embeddings.create(**kwargs)
        
        results = []
        for item in response.data:
            results.append(EmbeddingResult(
                embedding=item.embedding,
                model=self._model,
                dimensions=len(item.embedding),
                tokens_used=response.usage.total_tokens // len(texts),
            ))
        return results
```

### 6.3 Ollama Implementation

```python
import httpx

class OllamaEmbeddingProvider(EmbeddingProvider):
    """Local embedding provider using Ollama's API."""
    
    def __init__(
        self,
        model: str = "nomic-embed-text",
        base_url: str = "http://localhost:11434",
    ):
        self._model = model
        self._base_url = base_url
        self._client = httpx.AsyncClient(base_url=base_url, timeout=30.0)
    
    @property
    def model_id(self) -> str:
        return self._model
    
    @property
    def dimensions(self) -> int:
        # nomic-embed-text produces 768-dim vectors
        return 768
    
    @property
    def max_tokens(self) -> int:
        return 8192
    
    async def embed_texts(self, texts: list[str]) -> list[EmbeddingResult]:
        results = []
        # Ollama doesn't support batch embedding natively;
        # we parallelize with asyncio.gather for throughput.
        for text in texts:
            response = await self._client.post(
                "/api/embeddings",
                json={"model": self._model, "prompt": text}
            )
            response.raise_for_status()
            data = response.json()
            results.append(EmbeddingResult(
                embedding=data["embedding"],
                model=self._model,
                dimensions=len(data["embedding"]),
                tokens_used=0,  # Ollama doesn't report token usage
            ))
        return results
```

### 6.4 Azure OpenAI Implementation

```python
from openai import AsyncAzureOpenAI

class AzureOpenAIEmbeddingProvider(EmbeddingProvider):
    """Azure OpenAI embedding provider.
    
    Requires: AZURE_OPENAI_ENDPOINT, AZURE_OPENAI_API_KEY,
    and a deployed embedding model.
    """
    
    def __init__(
        self,
        deployment_name: str,
        endpoint: str,
        api_key: str,
        api_version: str = "2024-02-01",
        dimensions: int | None = None,
    ):
        self._client = AsyncAzureOpenAI(
            azure_endpoint=endpoint,
            api_key=api_key,
            api_version=api_version,
        )
        self._deployment = deployment_name
        self._dimensions = dimensions or 1536
    
    @property
    def model_id(self) -> str:
        return f"azure/{self._deployment}"
    
    @property
    def dimensions(self) -> int:
        return self._dimensions
    
    @property
    def max_tokens(self) -> int:
        return 8191
    
    async def embed_texts(self, texts: list[str]) -> list[EmbeddingResult]:
        kwargs = {"model": self._deployment, "input": texts}
        if self._dimensions != 1536:
            kwargs["dimensions"] = self._dimensions
        
        response = await self._client.embeddings.create(**kwargs)
        
        results = []
        for item in response.data:
            results.append(EmbeddingResult(
                embedding=item.embedding,
                model=f"azure/{self._deployment}",
                dimensions=len(item.embedding),
                tokens_used=response.usage.total_tokens // len(texts),
            ))
        return results
```

### 6.5 Provider Selection

```python
def create_provider(config: dict) -> EmbeddingProvider:
    """Factory function to create an embedding provider from config."""
    provider_type = config.get("provider", "openai")
    
    match provider_type:
        case "openai":
            return OpenAIEmbeddingProvider(
                model=config.get("model", "text-embedding-3-small"),
                dimensions=config.get("dimensions"),
                api_key=config.get("api_key"),
            )
        case "azure":
            return AzureOpenAIEmbeddingProvider(
                deployment_name=config["deployment_name"],
                endpoint=config["endpoint"],
                api_key=config["api_key"],
                dimensions=config.get("dimensions"),
            )
        case "ollama":
            return OllamaEmbeddingProvider(
                model=config.get("model", "nomic-embed-text"),
                base_url=config.get("base_url", "http://localhost:11434"),
            )
        case _:
            raise ValueError(f"Unknown provider: {provider_type}")
```

---

## 7. Batch Embedding

### 7.1 Batch Strategy

For bulk operations (import, re-embedding, migration), texts are batched to minimize API round trips.

```python
import asyncio

BATCH_SIZE = 100  # Max texts per API call (OpenAI supports up to 2048)
MAX_TOKENS_PER_ITEM = 2048  # Our self-imposed limit per embedding input

async def batch_embed(
    provider: EmbeddingProvider,
    texts: list[str],
    batch_size: int = BATCH_SIZE,
) -> list[EmbeddingResult]:
    """Embed a list of texts in batches.
    
    Args:
        provider: The embedding provider to use.
        texts: All texts to embed.
        batch_size: Max texts per API call.
    
    Returns:
        List of EmbeddingResult, preserving input order.
    """
    results: list[EmbeddingResult] = []
    
    for i in range(0, len(texts), batch_size):
        batch = texts[i : i + batch_size]
        batch_results = await provider.embed_texts(batch)
        results.extend(batch_results)
        
        # Rate limiting: pause between batches to avoid 429s
        if i + batch_size < len(texts):
            await asyncio.sleep(0.1)  # 100ms between batches
    
    return results
```

### 7.2 Batch Size Rationale

| Factor | Constraint |
|--------|-----------|
| OpenAI batch limit | Up to 2048 inputs per request |
| Token budget per request | ~300K tokens (OpenAI tier-1 rate limit) |
| Our per-item token limit | 2048 tokens |
| Conservative batch size | **100 items** (≤ 204,800 tokens per batch) |

At 100 items per batch, a 10,000-memory re-embedding takes ~100 API calls. At ~200 ms per call, that's ~20 seconds wall-clock time (plus rate limit pauses).

### 7.3 Error Handling in Batches

```python
async def batch_embed_resilient(
    provider: EmbeddingProvider,
    texts: list[str],
    batch_size: int = BATCH_SIZE,
    max_retries: int = 3,
) -> list[EmbeddingResult | None]:
    """Batch embed with retry logic and partial failure handling.
    
    Returns None for texts that failed after all retries.
    """
    results: list[EmbeddingResult | None] = [None] * len(texts)
    
    # First pass: embed in batches
    failed_indices: list[int] = []
    
    for i in range(0, len(texts), batch_size):
        batch_indices = list(range(i, min(i + batch_size, len(texts))))
        batch_texts = [texts[j] for j in batch_indices]
        
        for attempt in range(max_retries):
            try:
                batch_results = await provider.embed_texts(batch_texts)
                for idx, result in zip(batch_indices, batch_results):
                    results[idx] = result
                break
            except Exception as e:
                if attempt == max_retries - 1:
                    failed_indices.extend(batch_indices)
                else:
                    wait = 2 ** attempt  # Exponential backoff
                    await asyncio.sleep(wait)
        
        await asyncio.sleep(0.1)
    
    if failed_indices:
        # Retry failures individually (smaller requests are less likely to fail)
        for idx in failed_indices:
            try:
                result = await provider.embed_text(texts[idx])
                results[idx] = result
            except Exception:
                pass  # Leave as None; caller handles missing embeddings
    
    return results
```

---

## 8. Dimensionality Mismatch Handling

### 8.1 The Problem

The `memory_vectors` table is defined with a fixed dimension: `FLOAT[1536]`. When a local model (e.g., `nomic-embed-text`) returns 768-dim vectors, they cannot be inserted directly.

### 8.2 Solution: Zero-Padding

Vectors from lower-dimensional models are **zero-padded** to the target dimension:

```python
TARGET_DIMENSIONS = 1536

def normalize_dimensions(
    embedding: list[float],
    target_dim: int = TARGET_DIMENSIONS
) -> list[float]:
    """Normalize an embedding vector to the target dimensionality.
    
    - If embedding is shorter than target: zero-pad.
    - If embedding equals target: return as-is.
    - If embedding is longer than target: truncate (MRL-style).
    
    Zero-padding preserves cosine similarity between vectors of the
    same original dimensionality: cos(pad(a), pad(b)) == cos(a, b).
    
    WARNING: Cosine similarity between a zero-padded vector and a
    native-dimension vector is NOT meaningful. All vectors in a
    database must come from the same model.
    """
    current_dim = len(embedding)
    
    if current_dim == target_dim:
        return embedding
    elif current_dim < target_dim:
        # Zero-pad to target dimension
        return embedding + [0.0] * (target_dim - current_dim)
    else:
        # Truncate (valid for MRL-trained models like text-embedding-3-*)
        return embedding[:target_dim]
```

### 8.3 Cross-Model Compatibility

**Vectors from different models are NOT compatible.** Cosine similarity between a `text-embedding-3-small` vector and a `nomic-embed-text` vector is meaningless — they occupy different vector spaces.

The system enforces model consistency:

```python
# Stored in the database as metadata
EMBEDDING_MODEL_KEY = "embedding_model"

def get_db_embedding_model(conn) -> str | None:
    """Read the embedding model used for this database."""
    row = conn.execute(
        "SELECT value FROM metadata WHERE key = ?",
        (EMBEDDING_MODEL_KEY,)
    ).fetchone()
    return row[0] if row else None

def set_db_embedding_model(conn, model_id: str):
    """Record which embedding model this database uses."""
    conn.execute(
        "INSERT OR REPLACE INTO metadata (key, value) VALUES (?, ?)",
        (EMBEDDING_MODEL_KEY, model_id)
    )
```

On every embedding operation:

```python
def validate_model_consistency(conn, provider: EmbeddingProvider):
    """Ensure the provider's model matches the database's model."""
    db_model = get_db_embedding_model(conn)
    if db_model is None:
        # First embedding — record the model
        set_db_embedding_model(conn, provider.model_id)
    elif db_model != provider.model_id:
        raise ModelMismatchError(
            f"Database uses '{db_model}' embeddings, but provider "
            f"is '{provider.model_id}'. Run re-embedding migration "
            f"to switch models."
        )
```

### 8.4 Metadata Table

A small key-value metadata table tracks database-level configuration:

```sql
CREATE TABLE IF NOT EXISTS metadata (
    key   TEXT PRIMARY KEY,
    value TEXT NOT NULL
);
```

Stored keys:

| Key | Example Value | Purpose |
|-----|---------------|---------|
| `embedding_model` | `text-embedding-3-small` | Model consistency validation |
| `embedding_dimensions` | `1536` | Dimension consistency check |
| `schema_version` | `1` | Redundant with `PRAGMA user_version` but queryable |

---

## 9. Caching

### 9.1 Cache Design

Embedding API calls are the dominant cost (latency and money) in the system. A content-addressed cache avoids re-embedding unchanged content.

**Cache key:** `(model_id, sha256(input_text))`

**Cache value:** The embedding vector (list of floats).

```python
import hashlib
from pathlib import Path
import json

class EmbeddingCache:
    """File-backed embedding cache.
    
    Uses a simple file-per-entry approach under a cache directory.
    Each file is named by the SHA-256 hash of the cache key and
    contains the embedding vector as a JSON array.
    
    Location: ~/.engram/cache/embeddings/
    """
    
    def __init__(self, cache_dir: str | None = None):
        self._dir = Path(cache_dir or "~/.engram/cache/embeddings").expanduser()
        self._dir.mkdir(parents=True, exist_ok=True)
    
    def _key_hash(self, model_id: str, input_text: str) -> str:
        key = f"{model_id}:{input_text}"
        return hashlib.sha256(key.encode()).hexdigest()
    
    def get(self, model_id: str, input_text: str) -> list[float] | None:
        """Look up a cached embedding. Returns None on miss."""
        path = self._dir / self._key_hash(model_id, input_text)
        if path.exists():
            return json.loads(path.read_text())
        return None
    
    def put(self, model_id: str, input_text: str, embedding: list[float]):
        """Store an embedding in the cache."""
        path = self._dir / self._key_hash(model_id, input_text)
        path.write_text(json.dumps(embedding))
    
    def invalidate(self, model_id: str, input_text: str):
        """Remove a cached embedding."""
        path = self._dir / self._key_hash(model_id, input_text)
        path.unlink(missing_ok=True)
    
    def clear(self):
        """Clear the entire cache."""
        for path in self._dir.iterdir():
            path.unlink()
```

### 9.2 Cache Integration

```python
class CachedEmbeddingProvider(EmbeddingProvider):
    """Wrapper that adds caching to any EmbeddingProvider."""
    
    def __init__(self, inner: EmbeddingProvider, cache: EmbeddingCache):
        self._inner = inner
        self._cache = cache
    
    @property
    def model_id(self) -> str:
        return self._inner.model_id
    
    @property
    def dimensions(self) -> int:
        return self._inner.dimensions
    
    @property
    def max_tokens(self) -> int:
        return self._inner.max_tokens
    
    async def embed_texts(self, texts: list[str]) -> list[EmbeddingResult]:
        results: list[EmbeddingResult | None] = [None] * len(texts)
        uncached_indices: list[int] = []
        uncached_texts: list[str] = []
        
        # Check cache for each text
        for i, text in enumerate(texts):
            cached = self._cache.get(self.model_id, text)
            if cached is not None:
                results[i] = EmbeddingResult(
                    embedding=cached,
                    model=self.model_id,
                    dimensions=len(cached),
                    tokens_used=0,
                    cached=True,
                )
            else:
                uncached_indices.append(i)
                uncached_texts.append(text)
        
        # Embed uncached texts
        if uncached_texts:
            new_results = await self._inner.embed_texts(uncached_texts)
            for idx, result in zip(uncached_indices, new_results):
                results[idx] = result
                self._cache.put(self.model_id, texts[idx], result.embedding)
        
        return results  # type: ignore (all Nones are filled)
```

### 9.3 Cache Invalidation

The cache is invalidated when:

| Event | Action |
|-------|--------|
| Memory content updated | Invalidate the old cache entry |
| Embedding model changed | Clear the entire cache |
| Cache size > 500 MB | LRU eviction of oldest entries |

### 9.4 Cache Storage Estimate

| Memories | Cache Size (JSON) | Cache Size (Binary) |
|----------|-------------------|---------------------|
| 1,000 | ~24 MB | ~6 MB |
| 10,000 | ~240 MB | ~60 MB |
| 50,000 | ~1.2 GB | ~300 MB |

The JSON format is used for debuggability. A future optimization could switch to binary (struct-packed float32) for 4× size reduction.

---

## 10. Re-Embedding

### 10.1 When to Re-Embed

| Trigger | Scope | Action |
|---------|-------|--------|
| Memory content updated | Single memory | Re-embed that memory |
| Summary regenerated | Single memory | Re-embed that memory |
| Embedding model changed | Entire database | Re-embed all memories |
| Dimension change | Entire database | Re-create vec0 table, re-embed all |

### 10.2 Model Migration

Switching embedding models requires a full re-embedding:

```python
async def migrate_embedding_model(
    conn,
    old_provider: EmbeddingProvider,
    new_provider: EmbeddingProvider,
    batch_size: int = 100,
):
    """Migrate all embeddings from one model to another.
    
    This is a destructive operation — create a backup first.
    """
    # 1. Verify current model matches old_provider
    db_model = get_db_embedding_model(conn)
    assert db_model == old_provider.model_id
    
    # 2. Fetch all memories that need re-embedding
    rows = conn.execute("""
        SELECT m.id, m.content, m.content_type, m.summary
        FROM memories m
        WHERE m.superseded_by IS NULL AND m.confidence > 0.0
    """).fetchall()
    
    # 3. Construct embedding inputs
    texts = [
        construct_embedding_input(r["content"], r["content_type"], r["summary"])
        for r in rows
    ]
    memory_ids = [r["id"] for r in rows]
    
    # 4. Batch embed with new provider
    results = await batch_embed(new_provider, texts, batch_size)
    
    # 5. Drop and recreate vec0 table if dimensions changed
    new_dim = new_provider.dimensions
    target_dim = max(new_dim, TARGET_DIMENSIONS)  # Pad if needed
    
    conn.execute("DROP TABLE IF EXISTS memory_vectors")
    conn.execute(f"""
        CREATE VIRTUAL TABLE memory_vectors USING vec0(
            memory_id TEXT PRIMARY KEY,
            embedding FLOAT[{target_dim}]
        )
    """)
    
    # 6. Insert new embeddings
    for mid, result in zip(memory_ids, results):
        embedding = normalize_dimensions(result.embedding, target_dim)
        store_embedding(conn, mid, embedding)
    
    # 7. Update metadata
    set_db_embedding_model(conn, new_provider.model_id)
    conn.execute(
        "INSERT OR REPLACE INTO metadata (key, value) VALUES (?, ?)",
        ("embedding_dimensions", str(target_dim))
    )
    
    conn.commit()
```

### 10.3 Single Memory Re-Embedding

```python
async def reembed_memory(conn, provider: EmbeddingProvider, memory_id: str):
    """Re-embed a single memory after content/summary change."""
    row = conn.execute(
        "SELECT content, content_type, summary FROM memories WHERE id = ?",
        (memory_id,)
    ).fetchone()
    
    text = construct_embedding_input(
        row["content"], row["content_type"], row["summary"]
    )
    
    result = await provider.embed_text(text)
    embedding = normalize_dimensions(result.embedding, TARGET_DIMENSIONS)
    update_embedding(conn, memory_id, embedding)
```

---

## 11. Cost Estimation

### 11.1 Token Estimation

Average memory embedding input: ~300 tokens (content_type header + summary + 512 chars of content).

Average query embedding input: ~50 tokens (question + brief context).

### 11.2 Cost Per Operation

| Operation | Tokens | Cost (3-small) | Cost (3-large) |
|-----------|--------|-----------------|-----------------|
| Single memory embed | ~300 | $0.000006 | $0.000039 |
| Single query embed | ~50 | $0.000001 | $0.0000065 |
| Batch of 100 memories | ~30,000 | $0.0006 | $0.0039 |
| Full re-embed (10K memories) | ~3,000,000 | $0.06 | $0.39 |
| Full re-embed (50K memories) | ~15,000,000 | $0.30 | $1.95 |

### 11.3 Monthly Cost Estimate

Assuming moderate agent usage:

| Usage Pattern | Captures/day | Queries/day | Monthly Cost (3-small) |
|---------------|-------------|-------------|------------------------|
| Light (personal) | 5 | 20 | ~$0.02 |
| Moderate (daily work) | 20 | 100 | ~$0.08 |
| Heavy (team, multi-project) | 100 | 500 | ~$0.40 |

At these price points, embedding costs are negligible compared to LLM inference costs for the agent itself.

### 11.4 Ollama (Local) Cost

Zero marginal cost. Fixed costs:

| Resource | Consumption |
|----------|------------|
| RAM | ~1 GB for `nomic-embed-text` model |
| CPU/GPU | ~50 ms per embedding on Apple M-series |
| Disk | ~275 MB for model weights |

---

## 12. Configuration

### 12.1 Default Configuration

```toml
# ~/.engram/config.toml

[embeddings]
provider = "openai"
model = "text-embedding-3-small"
dimensions = 1536
batch_size = 100
max_tokens_per_item = 2048

[embeddings.cache]
enabled = true
directory = "~/.engram/cache/embeddings"
max_size_mb = 500

[embeddings.openai]
# api_key loaded from OPENAI_API_KEY env var by default
# api_key = "sk-..."

[embeddings.azure]
# endpoint = "https://myinstance.openai.azure.com/"
# api_key = "..."
# deployment_name = "text-embedding-3-small"

[embeddings.ollama]
# base_url = "http://localhost:11434"
# model = "nomic-embed-text"
```

### 12.2 Environment Variables

| Variable | Purpose | Default |
|----------|---------|---------|
| `OPENAI_API_KEY` | OpenAI API key | Required for OpenAI provider |
| `AZURE_OPENAI_ENDPOINT` | Azure OpenAI endpoint | Required for Azure provider |
| `AZURE_OPENAI_API_KEY` | Azure OpenAI API key | Required for Azure provider |
| `CANVAS_EMBEDDING_PROVIDER` | Override provider | Config file value |
| `CANVAS_EMBEDDING_MODEL` | Override model | Config file value |
| `CANVAS_EMBEDDING_CACHE` | Override cache dir | `~/.engram/cache/embeddings` |

---

## Appendix A: Vector Serialization

sqlite-vec accepts vectors in two formats:

### JSON Array

```python
import json

vec_json = json.dumps(embedding)  # "[0.123, -0.456, ...]"
conn.execute(
    "INSERT INTO memory_vectors (memory_id, embedding) VALUES (?, ?)",
    (memory_id, vec_json)
)
```

### Binary (Recommended)

```python
import struct

vec_bytes = struct.pack(f"<{len(embedding)}f", *embedding)
conn.execute(
    "INSERT INTO memory_vectors (memory_id, embedding) VALUES (?, ?)",
    (memory_id, vec_bytes)
)
```

Binary format is ~4× smaller in transit and avoids JSON parsing overhead. It uses little-endian IEEE 754 float32 encoding, matching sqlite-vec's internal storage format.

## Appendix B: Embedding Quality Validation

To verify embeddings are working correctly, use these sanity checks:

```python
import numpy as np

def cosine_similarity(a: list[float], b: list[float]) -> float:
    """Cosine similarity between two vectors."""
    a_arr, b_arr = np.array(a), np.array(b)
    return float(np.dot(a_arr, b_arr) / (np.linalg.norm(a_arr) * np.linalg.norm(b_arr)))

# Sanity check 1: Same text → identical embedding
emb1 = await provider.embed_text("User prefers Python for backend services")
emb2 = await provider.embed_text("User prefers Python for backend services")
assert cosine_similarity(emb1.embedding, emb2.embedding) > 0.999

# Sanity check 2: Similar texts → high similarity
emb_a = await provider.embed_text("User prefers Python for backend services")
emb_b = await provider.embed_text("The user likes using Python for server-side code")
sim = cosine_similarity(emb_a.embedding, emb_b.embedding)
assert sim > 0.80, f"Expected > 0.80, got {sim}"

# Sanity check 3: Unrelated texts → low similarity
emb_x = await provider.embed_text("User prefers Python for backend services")
emb_y = await provider.embed_text("The weather in Tokyo is warm in July")
sim = cosine_similarity(emb_x.embedding, emb_y.embedding)
assert sim < 0.40, f"Expected < 0.40, got {sim}"
```
