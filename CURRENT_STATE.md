# OpenWork current state

This document is the single status source for the repository. Claims below are
scoped by evidence level and were last refreshed on 2026-08-21.

## Working

- `openwork demo sales` runs the deterministic July/August sales analysis in a
  digest-pinned, hardened container, scans the two artifacts, verifies their
  golden bytes and hashes, records the audit chain, and reports `Succeeded`.
- The reusable sales runner uses `SystemDockerCli`, `DockerSandbox`, and
  `ExecutionOrchestrator`; input mounts are read-only and the analyzer is
  invoked as an executable plus fixed arguments without a shell.
- Docker execution starts without blocking on provider stdin. A separate
  attach worker delivers bounded stdin while timeout and cancellation polling
  remain active; timeout, cancel, and transport failure kill and clean up the
  container.
- Runtime task adapters prepare Claude Code and Codex invocations with the
  prompt on stdin rather than argv. Strict JSONL decoding validates run IDs,
  event sequences, output bounds, terminal events, truncation, and exit state.
- The execution store has in-memory and Postgres implementations. Postgres
  persists runs, approvals, single-use action claims, artifacts, and hash-chain
  audit events with CAS revisions and transactional state changes.
- The authenticated Control API persists run creation, reads runs/events/
  artifacts/approvals, and performs approval or denial with trusted server time
  and actor identity. Cancellation remains deliberately unavailable until a
  durable worker can prove that the runtime and sandbox stopped.
- Startup recovery deterministically transitions persisted `Planning` and
  `Running` runs to `Failed` with an audit event. M1 has no `Cancelling` state.
- Policy tests cover automatic filesystem read/write, exact-bound L3
  `email.send` approval, single-use claim consumption, replay and parameter
  tampering rejection, and direct L4 `database.delete` denial.
- `ActionExecutor` accepts only a repository-verified `ClaimedAction`.
  `MockActionExecutor` provides side-effect-free, action-ID-idempotent M1
  execution and records an `action_executed` audit event.
- Docker and Podman share one hardened container policy builder through a
  sealed engine adapter. Docker remains the real M1 backend; Podman reports its
  host-dependent capabilities rather than implying parity.
- Compose starts a non-root, read-only Control API with a read-only workspace
  mount and a digest-pinned Postgres service. The service runs migrations and
  recovery before listening.

## Tested

- Workspace formatting, locked checking, strict Clippy, all-target tests, and
  release build pass locally.
- The default workspace suite exercises runtime decoding, sandbox lifecycle,
  artifact/path safety, policy, approval binding/replay, audit integrity,
  orchestrator terminal states, Control API fail-closed behavior, and the M1
  control-plane scenario.
- Real-Postgres tests exercise approve-versus-deny, consume-versus-consume,
  cancel-versus-complete, revision races, and selective/idempotent crash
  recovery.
- CI includes a real Docker daemon sales test and a real Postgres concurrency
  job. Compose CI builds and starts the deployed services, checks health,
  creates an authenticated run, verifies prompt omission, and reads its genesis
  audit event.
- CI now runs actual CodeQL analysis and scans the built Control API image for
  critical vulnerabilities instead of using placeholder or duplicate checks.

## Real-host verified

- macOS arm64 with Docker Server 29.2.0: the digest-pinned BusyBox sales
  container completed, produced byte-identical CSV/Markdown artifacts, passed
  artifact hashing and audit verification, and cleaned up.
- macOS arm64 with a real PostgreSQL 17.6 container: all five transaction-race
  and recovery tests passed.
- macOS arm64 Compose: migrations 1 through 3 applied; Postgres and the Control
  API became healthy; an authenticated queued run persisted; its response did
  not contain the prompt; its `run_created` audit event round-tripped and
  verified from Postgres.
- `scripts/demo-m1.sh` completed Doctor, the real-container sales demo, and both
  policy/approval/action control-plane scenarios without sending external
  email.

## Fixture only

- Claude Code and Codex adapter commands, stdin routing, and provider event
  decoders are fixture-tested. No credential-gated provider task has completed
  inside the production sandbox image.
- Podman command routing and hardened-argument equivalence are fixture-tested;
  no real Podman host lifecycle has been run.
- Tier-1 macOS x64, Linux x64/arm64, and Windows Server 2025 x64 compatibility
  is CI evidence. It is not evidence for Windows 11, WSL, or arbitrary desktop
  distributions.
- The external-action path ends at `MockActionExecutor`; no email, ERP, CRM, or
  other connector is enabled.

## Missing

- A durable worker/dispatcher that claims queued Control API runs and drives
  the generic `RuntimeTask -> adapter -> sandbox -> events -> artifacts ->
  terminal state` path. Until it exists, `openwork run` fails clearly and does
  not create a misleading queued run.
- End-to-end cancellation from `POST /v1/runs/:id/cancel` through a durable
  worker to runtime and sandbox termination. The route currently returns 503.
- A credential-gated real Claude Code or Codex execution image and optional
  provider E2E job.
- Real Podman host validation and durable production idempotency for a real
  external-action executor.
- The thin employee/admin web UI, intentionally deferred to M1.1.

## Blockers

- The deterministic M1 demo is repeatable, but the generic API/CLI execution
  product is not production-ready until queued runs have an owned durable
  worker and cancellation protocol.
- Enterprise pilot readiness additionally requires provider-image provenance,
  real provider validation, operational credential brokering, deployment
  observability, backup/restore, and an external security review.
