---
created: 2026-02-17T19:57:00Z
contributors: [ken]
tags: [hipaa, compliance, healthcare, encryption]
keywords: [TLS, AES-256, encryption-at-rest, audit-logs, PHI, protected-health-information]
relates-to: []
---

# HIPAA Compliance Requirements

## What We Learned
This project handles patient data and requires HIPAA compliance.

**Core Requirements:**
- Encrypted transmission: TLS 1.2 or higher
- Encrypted storage: AES-256 encryption at rest
- Audit logging: Track all data access
- Access controls: Role-based authentication

## Why It Matters
Non-compliance can result in:
- Legal penalties ($100-$50,000 per violation)
- Loss of healthcare partnerships
- Reputation damage

## Context
Discussed during project kickoff (2026-02-17). User emphasized this as non-negotiable for healthcare clients.

## Related
- Links to security-architecture decisions
- Informs database selection (needs encryption support)
