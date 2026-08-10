# Capability Pack v1

## Status

Accepted — 2026-08-10

## Context

Implementers need a portable, reviewable unit for enterprise capabilities.

## Decision

Define a versioned schema before implementing packs. Installation validates dependencies,
license, permissions, explicit consent, registration, and self-tests.

## Consequences

Knowledge, Office, and data analysis use the same lifecycle and permission vocabulary.

## Alternatives

Unversioned plugin folders and a visual workflow canvas were rejected.

## Security implications

Permissions are declared, diffed, audited, and denied until accepted.

## License implications

Every pack declares a license and third-party dependencies.

## Revisit trigger

Backward-compatible v1 evolution is insufficient for a proven ecosystem requirement.
