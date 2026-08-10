# Runtime lockfile and provenance

Status: Accepted

## Context

Installation state must be reproducible and auditable without mixing user choices,
resolved artifacts, and credentials.

## Decision

Separate user configuration, machine-managed lockfile, and secret storage. The
versioned lockfile records requested and resolved versions, source, checksum and
its authority, installed path, timestamps, and status. Writes use atomic replace
and preserve the previous valid file on failure.

## Consequences

Unknown future schema versions fail with remediation. Migrations are explicit and
tested. A missing official checksum is represented honestly rather than invented.

## Security and license implications

Tokens and authorization material are prohibited. Files use least-privilege modes;
runtime license identity is provenance, not copied code.

## Revisit trigger

Revisit when the first backward-incompatible schema migration is required.
