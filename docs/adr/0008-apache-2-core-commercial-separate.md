# Apache-2.0 core and separate commercial products

## Status

Accepted — 2026-08-10

## Context

Community self-hosting must remain useful while implementers can charge for services.

## Decision

License OpenWork-authored Community code under Apache-2.0. Keep hosted, fleet, premium,
and SLA products in separate repositories connected through public interfaces.

## Consequences

Community code contains no artificial payment failure paths.

## Alternatives

Source-available core and mixed private directories were rejected.

## Security implications

Public extension interfaces must authenticate and authorize commercial providers.

## License implications

Third-party licenses remain independent and may narrow specific profiles.

## Revisit trigger

Contributor or foundation governance adopts a compatible long-term license policy.
