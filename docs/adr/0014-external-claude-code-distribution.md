# External Claude Code distribution

Status: Accepted

## Context

Claude Code is an Anthropic product with its own official installers, release
cadence, authentication, terms, and distribution policy.

## Decision

Treat Claude Code as an external-managed runtime. Detect and preserve an existing
installation. When absent, plan only official Anthropic install channels. Do not
vendor, mirror, reverse engineer, or silently replace it. `claude-code-rev` may
inform research but is never a shipped dependency or installer source.

## Consequences

OpenWork records observed version and provenance but does not claim ownership of
the runtime. Login remains an explicit user action.

## Security and license implications

Official HTTPS endpoints are allowlisted. Anthropic terms remain distinct from
OpenWork's Apache-2.0 license, and credentials are never persisted in the lockfile.

## Revisit trigger

Revisit when Anthropic changes an official installer or distribution policy.
