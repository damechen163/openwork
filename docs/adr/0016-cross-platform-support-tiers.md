# Cross-platform support tiers

Status: Accepted

## Context

Compilation, fixture tests, CI smoke tests, and real-device validation provide
different evidence. Treating them as equivalent would mislead users.

## Decision

Tier 1 targets are macOS arm64/x64, Ubuntu Linux x64/arm64, Windows 11 x64, and
WSL2. Tier 2 targets are Windows native edge cases, Windows arm64, and Debian.
Documentation reports four evidence levels separately: compiled, fixture-tested,
CI-smoked, and real-host validated.

## Consequences

Release gates require the documented Tier 1 evidence. Unsupported hosts receive
actionable diagnostics rather than best-effort mutation. Docker remains optional.

## Security and license implications

Platform commands stay behind typed runners and least-privilege paths. Cross-build
tools and CI actions require provenance and license review.

## Revisit trigger

Revisit after support data shows a Tier 2 platform can meet Tier 1 gates.
