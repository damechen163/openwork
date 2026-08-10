# Single company first

## Status

Accepted — 2026-08-10

## Context

V0.1 needs a dependable self-hosted installation, not SaaS billing complexity.

## Decision

One deployment serves one company. Keep only a lightweight installation/company identity.

## Consequences

Authorization and operations stay simpler; SaaS tenancy is deferred.

## Alternatives

Shared multi-tenant SaaS was rejected for v0.1.

## Security implications

Company boundaries rely on deployment isolation, while user/group boundaries remain explicit.

## License implications

None.

## Revisit trigger

A validated multi-company product requirement with an isolation threat model.
