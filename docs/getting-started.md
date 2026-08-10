# Getting started

The Bootstrap Runtime is a native developer preview. It does not install the
broader OpenWork service stack.

## Developer requirements

- Rust 1.85 or newer for source builds
- Git for source checkout and selected runtime workflows
- Docker is optional and reported as `SKIP` when absent

```bash
cargo test --workspace
cargo run -p openwork-cli -- --version
cargo run -p openwork-cli -- status
cargo run -p openwork-cli -- doctor --json
cargo run -p openwork-cli -- install --dry-run --json
```

Dry-run does not create directories, download files, or execute subprocesses.
Never put provider keys on a command line or in the runtime lockfile.
