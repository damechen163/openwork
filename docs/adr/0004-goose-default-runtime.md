# Goose default runtime

## Status

Accepted — 2026-08-10

## Context

General employee tasks need a maintained runtime with MCP support.

## Decision

Adapt `aaif-goose/goose` through the public `AgentRuntime` contract; do not fork its kernel.

## Consequences

OpenWork owns policy and sandbox boundaries, not Goose internals.

## Alternatives

Writing a runtime and making a proprietary runtime mandatory were rejected.

## Security implications

Runtime requests still pass through OpenWork authorization and sandbox controls.

## License implications

Goose v1.45.0 is Apache-2.0.

## Revisit trigger

Upstream contract, maintenance, license, or sandbox compatibility changes.
