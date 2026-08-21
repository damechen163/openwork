# OpenWork-controlled MCP gateway boundary

Status: Accepted for v0.2 design; implementation deferred until M1 is complete

## Context

External agent runtimes can discover and call MCP tools, but an enterprise must
not delegate identity, policy, approval, credentials, or audit decisions to the
runtime. Implementing connectors before the M1 execution path is repeatable
would create a second, ungoverned side-effect path.

## Decision

MCP servers sit behind an OpenWork-controlled gateway. A runtime submits a tool
request to the gateway; OpenWork derives the authenticated actor, evaluates the
same action policy used by the Action Gateway, obtains an exact-bound approval
when required, brokers credentials only for the authorized call, executes the
connector, and records a redacted result and audit event.

The v1 design has four provider-neutral contracts:

- `McpToolDescriptor`: stable server/tool identity, input schema digest,
  declared action/resource mapping, risk ceiling, and required capabilities.
- `McpToolRequest`: run and action IDs, descriptor identity, canonical input,
  canonical input hash, requested resource, and trusted actor context supplied
  by the gateway rather than the runtime payload.
- `McpExecutionPolicy`: allow, deny, or require-approval decision plus the exact
  action/resource/parameter binding, expiry, and policy revision.
- `McpExecutionResult`: request and claim IDs, bounded redacted status metadata,
  artifact references, external receipt digest, and timestamps. It never
  contains credentials or arbitrary provider response bodies in audit storage.

The gateway must consume a valid single-use `ActionClaim` before any L3 side
effect. L4 and unknown tools fail closed. Connectors receive short-lived scoped
credentials after authorization; agent runtimes never receive the underlying
credential. Transport and connector adapters may be replaced without changing
policy or approval semantics.

No MCP runtime, marketplace, credential store, or connector is implemented as
part of M1. This ADR defines the boundary only and cannot block the sales demo.

## Consequences

Claude, Codex, Goose, and future runtimes use one governed tool path. Feishu,
WeCom, GitHub, Postgres, ERP, CRM, and Google Workspace remain replaceable
connectors rather than privileged runtime plugins. v0.2 must version wire
schemas before enabling a connector.

## Alternatives

Direct runtime-to-MCP access was rejected because it bypasses OpenWork approval,
credential, and audit controls. Building a marketplace in M1 was rejected as
scope expansion.

## Security implications

The gateway rejects duplicate JSON keys, non-canonical or oversized input,
descriptor drift, parameter/resource/action mutation, expired claims, replay,
unknown tools, and credential requests broader than the approved descriptor.
Secrets, authorization headers, raw prompts, private reasoning, and unbounded
tool responses are excluded from logs and audit events.

## License implications

Each future connector requires its own upstream, license, and distribution
review. This ADR adds no dependency.

## Revisit trigger

Revisit after M1 is repeatably verified and before the first production MCP
connector or credential broker is implemented.
