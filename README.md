# OpenWork

The open-source enterprise AI agent execution control plane.

Install once. Give every employee a private AI assistant with company knowledge,
business tools, and safe execution.

[中文](README.zh-CN.md) · [Getting started](docs/getting-started.md) ·
[Deployment guide](DEPLOYMENT.md) · [Deploy for a client](docs/deploy-for-client.md) ·
[Build a pack](docs/packs/build-your-first-pack.md)

> Status: M1 safe execution is complete on the real-host CLI path. A
> real-container sales demo, Postgres control state, policy/approval/action
> controls, artifacts, and hash-chain audit are implemented; the generic
> worker execution loop (openwork run) and secure prompt delivery are also
> implemented and verified on a clean host. Durable worker leases and
> fail-closed cancellation intent are implemented. See the evidence-scoped
> [current state](CURRENT_STATE.md).
=======

## What employees will be able to do

- Ask questions using authorized company knowledge.
- Analyze spreadsheets and generate documents in an isolated sandbox.
- Query explicitly allowed business data with read-only credentials.
- Run business tools only when policy and approvals permit them.

## Built for AI service providers

- Deploy one isolated installation for one client company.
- Add versioned capability packs and adapters without forking the control plane.
- Diagnose, back up, upgrade, roll back, and support installations consistently.

Apache-2.0 Community code permits commercial implementation services, subject to
the licenses of third-party components. See [licensing](docs/licensing.md).

## Developer quick start

```bash
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
cargo build --release
./scripts/demo-m1.sh
./target/release/openwork --version
./target/release/openwork status --json
./target/release/openwork doctor --json
./target/release/openwork install --dry-run --json
```

## Real AI task end to end (safe execution)

```bash
rm -rf /tmp/ow-demo && mkdir -p /tmp/ow-demo && cp -r samples/sales /tmp/ow-demo/
./target/release/openwork run \
  --workspace /tmp/ow-demo/sales \
  "Read README.md and implement analyze.py exactly per the task contract. Do not run any commands."
```

One `openwork run`: Claude Code writes `analyze.py` on the host (networked),
OpenWork executes it in a podman sandbox (`--network=none --read-only
--user 1000:1000`), and the outputs (`sales-analysis.csv` / `summary.md`) are
recorded with SHA-256 into the audit chain — final status `Succeeded`.
Requires podman ≥ 6 and an authenticated Claude Code; full steps in the
[deployment guide](DEPLOYMENT.md) and the [real-host demo](docs/demo/safe-execution.md).

## Admin Web dashboard

[apps/admin-web/](apps/admin-web/README.md) is the Electron + React + TypeScript
dashboard: Dashboard, Run Task (live progress + artifact table), Install,
Doctor, and Runtimes. See its [README](apps/admin-web/README.md) to launch.

Release archives for the five native Tier 1 build targets are installed by the
checksum-verifying [POSIX](scripts/install.sh) and [PowerShell](scripts/install.ps1)
scripts. Existing binaries are refused
unless an explicit force option creates a backup first. See the
[release checklist](docs/release/checklist.md) and reproducible
[Bootstrap demo](docs/demo/bootstrap-runtime.md). See the
[alpha release notes](docs/release/v0.1.0-alpha.1.md) for delivered scope and
known limitations. The newer M1 source workflow is described in
[Getting started](docs/getting-started.md); release artifacts remain at the
published bootstrap alpha until the M1 integration is merged and released.

See the [platform evidence matrix](docs/platform-support.md) for the difference
between fixtures, CI smoke tests, and real-host validation.
