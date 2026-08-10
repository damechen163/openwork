# LiteLLM gateway adapter

## Status

Accepted — 2026-08-10

## Context

OpenWork needs an OpenAI-compatible multi-provider gateway without creating one.

## Decision

Use LiteLLM OSS behind `GatewayAdapter`; never query its private database.

## Consequences

Credentials, routing, and usage remain adapter-mediated.

## Alternatives

A custom gateway and enterprise-only functionality were rejected.

## Security implications

Secrets must not enter logs; image signatures and digests are verified.

## License implications

Only MIT-licensed paths outside `enterprise/` may be relied upon.

## Revisit trigger

OSS license boundary, stable API, maintenance, or security posture materially changes.
