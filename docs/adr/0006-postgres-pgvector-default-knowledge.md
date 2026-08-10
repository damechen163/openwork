# PostgreSQL and pgvector for default knowledge

## Status

Accepted — 2026-08-10

## Context

V0.1 needs ACL-filtered relational and vector retrieval without a large RAG platform.

## Decision

Use supported PostgreSQL plus pgvector and SQL-layer ACL filtering before retrieval.

## Consequences

Schema, migrations, FTS, vectors, and backup share one operational database.

## Alternatives

Default RAGFlow and prompt-only access control were rejected.

## Security implications

Unauthorized chunks must never enter candidate retrieval; integration tests are release gates.

## License implications

Both use the permissive PostgreSQL License at reviewed versions.

## Revisit trigger

Measured scale or retrieval quality cannot meet published targets.
