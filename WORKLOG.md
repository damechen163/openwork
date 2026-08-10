# Work log

This file is updated before each development session ends.

## 2026-08-10 — Phase 0 bootstrap

- Read the full 2,252-line construction prompt and limited implementation to Phase 0.
- Created the public `shichenghaoshu/openwork` repository, required labels, four milestones,
  and canonical Issues #1–#30. Closed three transient duplicate issues created from a stale
  list response.
- Verified 12 requested upstreams from official repositories, documentation, releases, and
  registries; recorded exact commits, licenses, candidate image digests, and integration status.
- Marked LibreChat integration blocked because its official compose still uses an image that
  cannot be correlated with v0.8.7; recorded Goose's AAIF repository migration; opened Issue #34
  for MinerU's additional license terms.
- Added Apache-2.0 governance, bilingual README skeletons, security/support/contribution docs,
  nine ADRs, third-party notices, version lock, GitHub templates, Project bootstrap instructions,
  and CI skeleton.
- Implemented the TypeScript installer skeleton with `version`, `doctor [--json]`, and the
  non-mutating `install --dry-run [--json]` plan.
- Added tests first, observed the expected missing-implementation failure, then reached 6/6 tests
  passing. Verified formatting, lint, typecheck, unit, integration, build, Markdown links, YAML,
  dependency audit, secret patterns, and the exact Apache license text.
- GitHub Project remains pending because the authenticated token lacks `project`/`read:project`;
  `.github/PROJECT_SETUP.md` and `scripts/bootstrap-github.sh` contain the exact continuation.
- First pull request: pending creation after this log is committed.
