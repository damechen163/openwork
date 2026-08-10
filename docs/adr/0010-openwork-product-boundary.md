# OpenWork bootstrap product boundary

Status: Accepted

## Context

OpenWork has a broad workspace vision, while the current milestone must produce a
safe cross-platform bootstrap runtime that can be independently validated.

## Decision

The `v0.1-bootstrap` scope is one local `openwork` binary: host inspection,
structured diagnostics, runtime discovery, managed installation, provenance, and
rollback. A web UI, server API, database, multi-tenancy, billing, and workflow
canvas are outside this milestone.

## Consequences

Every Bootstrap change must advance an executable command or its safety boundary.
Broader Phase 0 documents remain direction-setting rather than implementation scope.

## Security and license implications

The smaller boundary reduces exposed services and keeps OpenWork code Apache-2.0;
external runtime terms remain separate.

## Revisit trigger

Revisit after the Bootstrap release gates are met on Tier 1 platforms.
