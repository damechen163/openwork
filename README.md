# OpenWork

The open-source AI workspace installer for small businesses.

Install once. Give every employee a private AI assistant with company knowledge,
business tools, and safe execution.

[中文](README.zh-CN.md) · [Getting started](docs/getting-started.md) ·
[Deploy for a client](docs/deploy-for-client.md) · [Build a pack](docs/packs/build-your-first-pack.md)

> Status: `v0.1.0-alpha.0` Phase 0. The installer currently supports `version`,
> `doctor`, and non-mutating `install --dry-run`. It does not yet deploy services.

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

## Phase 0 developer quick start

```bash
corepack enable
pnpm install --frozen-lockfile
pnpm lint
pnpm typecheck
pnpm test
pnpm test:integration
pnpm build
node installer/cli/dist/cli.js version
node installer/cli/dist/cli.js doctor --json
node installer/cli/dist/cli.js install --dry-run --json
```

The current supported installation-host contract is Linux on `amd64` or `arm64`.
Resource thresholds remain initial targets until public benchmarks exist.
