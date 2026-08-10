# Runtime provider separation

Status: Accepted

## Context

Provider installation, authentication, health, and execution change independently.

## Decision

Keep provider adapters in separate modules behind the runtime contract. The CLI,
doctor, and installer may depend on the registry, never on provider internals.
Command execution and downloads use injected interfaces with deterministic fakes.

## Consequences

An adapter can be disabled or rolled back without changing other runtimes. Fixture
tests do not count as proof of real provider execution.

## Security and license implications

Executable plus argument arrays replace interpolated shell strings. Each provider
records its own upstream, license, source allowlist, and verification policy.

## Revisit trigger

Revisit if a shared behavior appears in three independent provider adapters.
