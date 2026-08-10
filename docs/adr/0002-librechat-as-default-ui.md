# LibreChat as default UI candidate

## Status

Accepted with upstream blocker — 2026-08-10

## Context

Employees need a mature multi-user chat UI without a new UI platform.

## Decision

Integrate LibreChat through configuration and stable extension points; do not vendor it.

## Consequences

Issue #4 remains blocked until a release-correlated image is pinned by digest.

## Alternatives

Building a new chat UI and invasive source forks were rejected.

## Security implications

Identity mapping and upstream security updates require compatibility tests.

## License implications

Reviewed v0.8.7 is MIT; transitive components still require scanning.

## Revisit trigger

License incompatibility, unmaintained upstream, or absence of a defensible release artifact.
