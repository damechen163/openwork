# Node.js development toolchain

Phase 0 uses Node.js, pnpm, TypeScript, ESLint, Prettier, typescript-eslint, Vitest,
and `@types/node` only for development/build/test. Exact package versions are locked in
`package.json` and `pnpm-lock.yaml`; transitive licenses are resolved from the lockfile.

- Redistribution: source package metadata and lockfile only; release bundles must produce an SBOM and license report
- Modifications: none
- Purpose: TypeScript compilation, formatting, linting, and tests
- Review date: 2026-08-10
