# Getting started

Phase 0 is a developer preview and does not install services yet.

## Developer requirements

- Node.js 24 LTS-compatible runtime
- Corepack and pnpm 10.21.0
- Docker Engine and Docker Compose for diagnostics

Run the commands in the root README. On Linux `amd64` or `arm64`, `doctor` validates the
current host contract. On other operating systems it intentionally reports a blocked
installation while remaining usable for development and tests.

Never put provider keys on a command line. The mutating installer will introduce a
permission-restricted secret file in Issue #5.
