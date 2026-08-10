# Agent runtime abstraction

Status: Accepted

## Context

Claude Code, Codex, Goose, and test doubles expose different commands and state.
Provider branching in the CLI would make diagnostics and installation inconsistent.

## Decision

Define one typed `AgentRuntime` contract covering identity, metadata, detection,
installation lifecycle, version, doctor, authentication, capabilities, execution,
and cancellation. A registry owns discovery; a shared compatibility suite owns
behavioral expectations.

## Consequences

Unsupported operations are explicit capabilities, not late command failures.
Renderers consume provider-neutral results.

## Security and license implications

Secrets cross only approved interfaces and are centrally redacted. The abstraction
does not imply that runtime licenses or distribution policies are interchangeable.

## Revisit trigger

Revisit when two implemented providers cannot express a required operation safely.
