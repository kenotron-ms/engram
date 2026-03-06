"""Shared types for the retrieval layer."""

from __future__ import annotations

from dataclasses import dataclass, field


@dataclass
class RetrievalResult:
    """A single memory returned by any retrieval route."""

    memory_id: str
    summary: str
    domain: str
    tags: list[str] = field(default_factory=list)
    content_type: str = "fact"
    importance: str = "medium"
    confidence: float = 0.7
    score: float = 0.0  # higher = more relevant

    def to_dict(self) -> dict:
        return {
            "memory_id": self.memory_id,
            "summary": self.summary,
            "domain": self.domain,
            "tags": self.tags,
            "content_type": self.content_type,
            "importance": self.importance,
            "confidence": self.confidence,
            "score": self.score,
        }
