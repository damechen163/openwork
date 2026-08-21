# OpenWork roadmap

OpenWork is an enterprise AI agent execution control plane. Roadmap work is
accepted only when it advances a verifiable path through identity, policy,
runtime, sandbox, approval, action execution, artifacts, and audit.

## v0.1 — M1: Safe Agent Execution

- One repeatable sales-analysis vertical slice through a real container engine.
- Claude Code and Codex remain external managed runtimes; prompts travel by
  bounded stdin and are not persisted in plaintext.
- Postgres-backed run, approval, action-claim, artifact, and audit state.
- L0-L2 automatic actions, exact-bound L3 approval, and fail-closed L4 denial.
- Single-use `ActionClaim` enforcement through a mock external-action executor.
- Deterministic crash recovery, Control API mutations, CLI demo, and real
  Docker/Postgres CI gates.

M1.1 may add a thin API-driven employee/admin web UI only after the backend
vertical slice above is repeatably green.

## v0.2 — MCP Gateway, Identity, and Connectors

- OpenWork-controlled MCP gateway contracts and credential-broker boundary.
- Enterprise identity integration and a small set of policy-enforced
  connectors. No connector marketplace in this release.

## v0.3 — Knowledge and Office Capability Packs

- Replaceable knowledge, office, and data-analysis capability packs built on
  the stable execution-control contracts.

## v0.4 — Enterprise Messaging and Admin UX

- Feishu/WeCom approval surfaces and production-oriented operator workflows.

## v1.0 — Stable enterprise control-plane contracts

- Versioned public contracts, compatibility policy, upgrade/rollback support,
  and evidence-backed production readiness.

Multi-tenant SaaS, billing, subscriptions, workflow canvases, mobile clients,
large RAG platforms, and custom-model training are outside this roadmap until
the M1 execution slice is complete.
