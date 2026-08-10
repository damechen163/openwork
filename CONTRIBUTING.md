# Contributing

Thank you for helping make enterprise AI installation safer and more repeatable.

1. Search existing issues and discuss architecture changes before implementation.
2. Link every pull request to an issue and keep it to one concern.
3. Add or adjust tests first when feasible.
4. Run `pnpm lint`, `pnpm typecheck`, `pnpm test`, `pnpm test:integration`, and `pnpm build`.
5. Update security, licensing, migrations, rollback, and documentation sections as applicable.

Third-party additions require a current official-source review, an entry in
`THIRD_PARTY_NOTICES.md`, a file under `third_party/`, and an updated version lock.
Do not add code from proprietary or ambiguously licensed projects.

Use Conventional Commit-style subjects where practical. By contributing, you agree
that your contribution is licensed under Apache-2.0.
