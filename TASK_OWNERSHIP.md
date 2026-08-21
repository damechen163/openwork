# M1 task ownership

Contract-breaking changes require Lead approval. Agents work in separate Git
worktrees and do not edit another owner's paths.

| Agent | Issues | Branch | Owned paths | Dependencies | Status |
| --- | --- | --- | --- | --- | --- |
| Lead | #64 | `opencat/m1-vertical-slice` | root Cargo files, `openwork-core`, `openwork-execution` orchestrator, ADRs, ownership, WORKLOG, shared schemas, README, global workflows | M0 main | **Integration in progress** |
| A Infrastructure | #3, #6 | `opencat/m1-control` | `crates/openwork-control-api/**`, `compose/**`, `migrations/**`, `contracts/openapi/**`, `docs/control-api/**` | #64 | **PR #76-#80 ready, pending merge** |
| B Sandbox | #12 | `opencat/m1-sandbox` | `crates/openwork-sandbox/**`, `tests/sandbox/**`, `docs/security/sandbox.md` | #64 | **✅ Merged into m1-vertical-slice** |
| C Execution | #13 | — | implementation modules below `crates/openwork-execution/src/` | #64 | **✅ Orchestrator wired with execute()** |
| D Policy | #14, #15 | — | `crates/openwork-policy/**`, `contracts/schemas/policy/**`, `tests/policy/**`, `docs/admin/approvals.md` | #64 | **✅ Merged (already on main)** |
| E Runtime | #63 | `opencat/m1-runtime-run` | runtime provider modules, `tests/runtime-execution/**` | #12, #13, #64 | **✅ Merged into m1-vertical-slice** |
| F QA | #65 | `opencat/m1-safe-e2e` | `tests/safe-execution/**`, `samples/sales/**`, `docs/demo/safe-execution.md` | #3, #12–#15, #63–#64 | **✅ Fixtures merged, pipeline test pending** |

## Current Integration State (2026-08-11)

### Merged into `opencat/m1-vertical-slice`
- Sandbox stack (#70-#73): DockerSandbox + FakeDockerCli tests
- Runtime stack (#88-#89): Claude/Codex adapters + 9 protocol tests
- E2E fixtures (#81-#82): Sales golden data + L3/L4 scenarios
- **New integration work:**
  - Stdin contract extension (`SandboxCommand` with 1 MiB stdin)
  - Docker CLI stdin piping (`SystemDockerCli`)
  - `into_sandbox_request()` bridge function
  - `ExecutionOrchestrator::execute()` lifecycle method

### Still pending
- Control API merge (#76-#80): Cargo.lock conflict
- Postgres ExecutionStore + ApprovalRepository implementation
- Control API POST mutation wiring (currently 503 stubs)
- Claude/Codex CLI flag verification
- Real Docker CI validation
- Full E2E pipeline test
- Postgres `action_claims` migration table

## Shared-file rule

Root `Cargo.toml`, `Cargo.lock`, bilingual READMEs, `ROADMAP.md`, `WORKLOG.md`,
schema indexes, release files, and global CI workflows are Lead-owned. Subagents
report required changes instead of editing those files.
