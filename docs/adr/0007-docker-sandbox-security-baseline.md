# Docker sandbox security baseline

## Status

Accepted — 2026-08-10

## Context

Office and data tasks execute untrusted generated code.

## Decision

Use disposable non-root OCI containers with read-only rootfs, isolated workspace, limits,
timeout, cleanup, and network denied by default. Never mount Docker socket or use privileged mode.

## Consequences

Compatibility is narrower by design; gVisor remains an optional hardened profile.

## Alternatives

Host execution and persistent shells were rejected.

## Security implications

The listed controls are release blockers and require adversarial integration tests.

## License implications

Docker/OCI and optional gVisor notices must remain current.

## Revisit trigger

A stronger maintained sandbox meets compatibility and installation targets.
