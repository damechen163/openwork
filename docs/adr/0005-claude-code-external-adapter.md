# Claude Code external adapter

## Status

Accepted — 2026-08-10

## Context

Some users may lawfully install Claude Code, but it is proprietary.

## Decision

An optional adapter may detect and call supported user-installed interfaces. Never redistribute,
reverse engineer, or imitate private protocols.

## Consequences

The adapter is unavailable when the lawful external installation is absent.

## Alternatives

Bundling Claude Code or making OpenWork Claude-only were rejected.

## Security implications

Credentials remain user-controlled and subprocess access is scoped.

## License implications

Use is subject to Anthropic’s commercial terms and trademark rules.

## Revisit trigger

Anthropic publishes a materially different supported license or integration contract.
