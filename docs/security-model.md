# Security model

The intended baseline is deny-by-default execution: non-root ephemeral containers,
read-only root filesystems, an isolated task workspace, resource/time limits, no Docker
socket, no privileged mode, and no network unless a pack declares a reviewed allowlist.

Knowledge authorization must filter candidates in SQL before retrieval. External or
persistent actions must pass the Action Gateway. L4 destructive or financial actions are
denied in v0.1; L3 requires an exact-action, exact-parameter, run-bound, expiring approval.

These are release requirements, not claims that Phase 0 already implements them.
