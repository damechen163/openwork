# M1 Current State Matrix

**Compiled:** 2026-08-11  
**Main commit:** `670df60` feat(approval): add atomic single-use approval store (#86)  
**Published tag:** `v0.1.0-alpha.1`

---

## Main Branch — Already Merged

| Capability | Status | Evidence | Crate/File |
|---|---|---|---|
| Run State Machine (9 statuses) | ✅ merged | `RunStatus` enum + `can_transition_to()` | `openwork-execution/src/lib.rs` |
| CAS revision | ✅ merged | `revision: u64` on Run, ApprovalRequest | `openwork-execution/src/lib.rs` |
| Transactional InMemoryStore | ✅ merged | `ExecutionStore` trait + `InMemoryExecutionStore` | `openwork-execution/src/store/mod.rs` |
| ExecutionOrchestrator | ✅ merged | Create run, transition, record artifacts | `openwork-execution/src/orchestrator/mod.rs` |
| Audit hash chain | ✅ merged | `AuditEvent` with `previous_hash` chaining | `openwork-execution/src/lib.rs` |
| ArtifactScanner | ✅ merged | Bounded, symlink-safe, SHA-256 | `openwork-execution/src/artifact/mod.rs` |
| Artifact SHA256 | ✅ merged | `Sha256Digest` type, `artifact::scan()` | `openwork-execution/src/lib.rs` |
| Path traversal protection | ✅ merged | `RelativeArtifactPath`, `SandboxWorkingDirectory` validation | `openwork-execution/src/lib.rs` |
| Policy YAML | ✅ merged | Versioned YAML config + engine | `openwork-policy/src/` |
| Policy Engine | ✅ merged | L0-L4 Risk, fail-closed unknown action | `openwork-policy/src/engine.rs` |
| Action Gateway | ✅ merged | `ActionRequest` with parameter hash | `openwork-policy/src/gateway.rs` |
| L0-L4 Risk | ✅ merged | `RiskLevel` enum (Read, LocalWrite, InternalMutation, ExternalEffect, DestructiveOrFinancial) | `openwork-execution/src/lib.rs` |
| Exact parameter binding | ✅ merged | `parameter_hash` on ActionRequest | `openwork-execution/src/lib.rs` |
| Approval CAS | ✅ merged | `ApprovalRepository` trait with CAS operations | `openwork-execution/src/approval/mod.rs` |
| Approval expiry | ✅ merged | TTL-based expiry, `expire_approval()` | `openwork-execution/src/approval/mod.rs` |
| Approval single-use | ✅ merged | Atomic consume + ActionClaim creation | `openwork-execution/src/approval/mod.rs` |
| Replay protection | ✅ merged | `parameter_hash` binding, consume-once | `openwork-execution/src/approval/mod.rs` |
| ActionClaim | ✅ merged | Durable proof of approved execution | `openwork-execution/src/approval/mod.rs` |
| Approval audit events | ✅ merged | `ApprovalRequested`, `Approved`, `Denied`, `Expired`, `Consumed`, `BindingMismatch` | `openwork-execution/src/lib.rs` |
| SandboxBackend trait | ✅ merged | `execute()`, `cancel()`, `cleanup()`, `health()` | `openwork-execution/src/lib.rs` |
| SandboxRequest | ✅ merged | Full contract with image, command, mounts, limits | `openwork-execution/src/lib.rs` |
| SandboxResult | ✅ merged | Full response with termination, stdout/stderr, output paths | `openwork-execution/src/lib.rs` |
| SandboxLimits | ✅ merged | CPU, memory, PID, timeout, output bounds | `openwork-execution/src/lib.rs` |
| RuntimeTask | ✅ merged | Task contract with prompt hash, working dir, capabilities | `openwork-execution/src/lib.rs` |
| RuntimeEvent | ✅ merged | Started, Stdout, Stderr, Message, ToolCall, Completed, Failed | `openwork-execution/src/lib.rs` |
| Runtime adapter discovery | ✅ merged | Claude/Codex external adapters in runtime crate | `openwork-runtime/src/` |
| MockRuntime | ✅ merged | For deterministic testing | `openwork-runtime/src/mock.rs` |
| CLI (version, doctor, status, install) | ✅ merged | Full native Rust CLI | `openwork-cli/` |
| Platform Detection | ✅ merged | macOS/Ubuntu/Windows detection | `openwork-platform/` |
| Cross-platform CI | ✅ merged | macOS arm64/x64, Ubuntu arm64/x64, Windows x64 | `.github/workflows/` |
| Release + SBOM + attestation | ✅ merged | v0.1.0-alpha.1 published | Release workflow |
| Installer | ✅ merged | POSIX + PowerShell, checksum-verified | `installer/` |

---

## Sandbox Stack (PRs #70 → #73)

| PR | Branch | Capability | Status | Key Files | Line Count | Blocker |
|---|---|---|---|---|---|---|
| #70 | `opencat/m1-sandbox-primitives` | Docker CLI wrapper + filesystem safety | Draft | `sandbox/src/cli.rs` (176L), `sandbox/src/filesystem.rs` (353L), `sandbox/src/lib.rs` (16L) | 593+ | Needs merge |
| #71 | `opencat/m1-sandbox-lifecycle` | DockerSandbox implements SandboxBackend | Draft | `sandbox/src/lib.rs` (458L) — container create/start/poll/kill/cleanup | 906+ | Builds on #70 |
| #72 | `opencat/m1-sandbox-tests` | Docker lifecycle tests | Draft | `sandbox/tests/docker_sandbox.rs` (465L) | 1371+ | Builds on #71 |
| #73 | `opencat/m1-sandbox` | Cleanup + output exhaustion fixes | Draft | Fixes to filesystem/lib + new docs | 1478+ | Builds on #72 |

**Key observations:**
- `DockerSandbox<C: DockerCli>` implements `SandboxBackend` trait
- Uses `filesystem::validate_mount()` for path safety
- Creates temporary directories for environment files and container ID
- Polls container state with timeout, supports cancel via `AtomicBool`
- Collects output paths from the output directory after execution
- Uses `decode_output()` for stdout/stderr with UTF-8 validation
- **No stdin support yet** — this is critical for Claude/Codex

**Sandbox security baseline (from code):**
- `--network none`
- `--cap-drop ALL`
- `--no-new-privileges`
- Read-only rootfs
- Non-root user
- Memory/CPU/PID limits enforced
- Cleanup on drop (ContainerGuard)

---

## Runtime Task Stack (PRs #88 → #91)

| PR | Branch | Capability | Status | Key Files | Blocker |
|---|---|---|---|---|---|
| #88 | `opencat/m1-runtime-contract` | `RuntimeTaskAdapter` trait + JSONL decoder | Draft | `runtime/src/task/mod.rs` (354L) | Needs merge |
| #89 | `opencat/m1-runtime-run` | Runtime protocol tests + Claude/Codex decoders | Draft | `runtime/tests/runtime_task.rs` (372L), `runtime/src/task/claude.rs` (216L), `codex.rs` (206L) | Builds on #88 |
| #90 | `opencat/m1-runtime-claude` | Claude Code sandbox task preparation | Draft | `runtime/src/task/claude.rs` (213L) — `ClaudeTaskAdapter` | Builds on #88 |
| #91 | `opencat/m1-runtime-codex` | Codex sandbox task preparation | Draft | `runtime/src/task/codex.rs` (202L) — `CodexTaskAdapter` | Builds on #90 |

**Key observations:**
- `RuntimeTaskAdapter::prepare()` takes a `RuntimeTask` and returns `RuntimeInvocation`
- `RuntimeInvocation` has `command: SandboxCommand`, `working_directory`, `stdin: Vec<u8>`, `output_protocol`
- `RuntimeTaskAdapter::decoder()` returns a stateful `RuntimeEventDecoder` that consumes JSONL lines
- Claude adapter uses `--print --output-format stream-json --safe-mode --no-session-persistence`
- Codex adapter uses `codex exec --json` pattern
- Prompt goes to stdin (not argv)
- Provider binaries are referenced by container_executable path

**Critical gap: stdin on SandboxRequest**
- Current `SandboxRequest` has `command: SandboxCommand` but NO `stdin` field
- `RuntimeInvocation` has `stdin: Vec<u8>` but it's not wired to SandboxRequest
- This needs a contract extension or a workaround via environment/temp file

---

## Control API Stack (PRs #76 → #80)

| PR | Branch | Capability | Status | Key Files | Blocker |
|---|---|---|---|---|---|
| #76 | `opencat/m1-control-api` | Postgres API library (Axum handlers) | Draft | `control-api/src/lib.rs` (442L) | Needs merge |
| #77 | `opencat/m1-control-tests` | API fail-closed test coverage | Draft | Tests for routes | Builds on #76 |
| #78 | `opencat/m1-control` | API contract documentation | Draft | OpenAPI/docs | Docs |
| #79 | `opencat/m1-control-infra` | Postgres migrations + Compose | Open | `migrations/0001_m1_control.up.sql` (99L), `compose/compose.yaml` (59L) | Needs #76 |
| #80 | `opencat/m1-control-binary` | Control service binary + health | Draft | `control-api/src/main.rs` (43L), Dockerfile | Builds on #76 |

**Key observations:**
- Uses Axum + sqlx with Postgres
- Bearer token auth with constant-time comparison
- Routes: `POST /v1/runs`, `GET /v1/runs/:id`, `GET /v1/runs/:id/events`, `GET /v1/runs/:id/artifacts`, `POST /v1/runs/:id/cancel`, `GET /v1/approvals`, `GET /v1/approvals/:id`, `POST /v1/approvals/:id/approve`, `POST /v1/approvals/:id/deny`, `GET /health`
- Run creation: takes runtime, prompt (SHA-256 only stored), workspace
- Actor from authenticated context (Bearer token), not request body

**NOTE: PRs #77 and #78 are drafts with mostly overlapping content; the stacked pattern means they accumulate.**

---

## Approval Stack (PRs #84 → #85)

| PR | Branch | Capability | Status | Key Files | Blocker |
|---|---|---|---|---|---|
| #84 | `opencat/m1-approval` | Action-claim transaction spec + Postgres integration | Draft | `approval/POSTGRES.md` (76L), `approval/mod.rs` updates (110L), `store/mod.rs` updates (355L) | Needs merge |
| #85 | `opencat/m1-approval-tests` | Replay race + binding tampering tests | Draft | `execution/tests/approval_store.rs` (534L) | Builds on #84 |

**Key observations:**
- Approval core (trait + InMemoryStore impl) already merged in #86
- #84 adds Postgres implementation of ApprovalRepository
- #85 adds comprehensive tests: consume-once, parameter drift, TTL expiry, concurrent admin race
- The `ApprovalRepository` trait is in main; the Postgres impl is in #84

---

## E2E Stack (PRs #81 → #82)

| PR | Branch | Capability | Status | Key Files | Blocker |
|---|---|---|---|---|---|
| #81 | `opencat/m1-safe-e2e` | L3/L4 demo scenarios + E2E crate | Draft | `crates/openwork-e2e/src/lib.rs` (320L), `scenario.rs` (146L), test fixtures | Needs Sandbox+Runtime+Approval |
| #82 | `opencat/m1-sales-fixtures` | Sales golden fixtures | Open | `samples/sales/` (6 CSV files + golden outputs), golden tests | Low dependency |

**Key observations:**
- E2E crate uses MockRuntime + real Policy + real ExecutionStore
- Sales fixtures: `sales_july.csv`, `sales_august.csv`, golden `sales-analysis.csv` + `summary.md`
- Scenarios: safe sales analysis, risky email.send (L3), destructive database.delete (L4)
- Golden hash tests for deterministic output

---

## Integration Gaps (What Must Be Built)

| Gap | Description | Priority | Owner |
|---|---|---|---|
| **Sandbox stdin** | `SandboxRequest` has no stdin field; `RuntimeInvocation` does. Need contract extension. | 🔴 BLOCKER | Lead + Sandbox |
| **Orchestrator → Sandbox wiring** | `ExecutionOrchestrator` creates runs but doesn't execute them. Need to call `SandboxBackend::execute()`. | 🔴 BLOCKER | Lead + Runtime |
| **Runtime → Sandbox wiring** | `RuntimeTaskAdapter::prepare()` returns `RuntimeInvocation` but nothing creates a `SandboxRequest` from it. | 🔴 BLOCKER | Lead + Runtime |
| **Approval → Execution chain** | Approval flow works in isolation but isn't triggered during execution. | 🔴 BLOCKER | Lead + Approval |
| **Postgres persistence** | `ExecutionStore` trait has InMemory impl; need Postgres impl wired to Control API. | 🔴 BLOCKER | Control API |
| **Control API mutations** | API has read routes; mutation routes (create run, cancel, approve, deny) need real backend wiring. | 🔴 BLOCKER | Control API |
| **Real Docker CI** | Sandbox tests use FakeDocker; need real Docker daemon in CI. | 🟡 HIGH | CI + Sandbox |
| **Provider binary problem** | Claude/Codex binary availability in sandbox containers unsolved. | 🟡 HIGH | Runtime |
| **Run lifecycle** | Run goes Created → Queued → Planning → ??? . No actual execution path. | 🔴 BLOCKER | Lead |
| **Cancel/timeout wiring** | Cancel endpoint exists but doesn't propagate to sandbox. | 🟡 HIGH | Control + Sandbox |
| **Crash recovery** | No recovery for orphaned Running runs on restart. | 🟡 MEDIUM | Control API |

---

## Missing Capabilities (Not in Any PR)

| Capability | Notes |
|---|---|
| Orchestrator execution loop | The orchestrator creates runs but never progresses them to Running/Succeeded |
| SandboxRequest construction from RuntimeInvocation | The bridge between task adapter output and sandbox input |
| Approval trigger in execution flow | Policy evaluates, approval needed → but who creates the ApprovalRequest? |
| E2E vertical slice test | Tests exist for individual modules but not end-to-end |
| Admin UI | No web UI exists yet |
| Real Claude/Codex end-to-end | No real provider has run through the full pipeline |

---

## Dependency Graph (Merge Order)

```
Phase 1 — Foundations (can parallel):
  Sandbox (#70 → #71) ←→ Runtime contract (#88)
  
Phase 2 — Integration (serial after Phase 1):
  Sandbox tests (#72, #73)
  Runtime adapters (#89, #90, #91)
  Control API (#76, #79, #80)
  
Phase 3 — Wiring (Lead serial):
  Wire Sandbox + Runtime in orchestrator
  Wire Approval into execution chain
  Wire Control API ↔ Postgres persistence
  
Phase 4 — E2E + Review (can parallel):
  E2E tests (#81, #82)
  Security review
  Architecture review
  QA review
```
