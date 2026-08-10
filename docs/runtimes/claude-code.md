# Claude Code runtime

Claude Code is an **external-managed** runtime. OpenWork detects and diagnoses an
existing executable but does not vendor, mirror, reverse engineer, silently replace,
or uninstall it. Login remains an explicit user action.

Managed plans use only Anthropic's documented endpoints:

- macOS, Linux, and WSL: `https://claude.ai/install.sh`
- Windows: `https://claude.ai/install.ps1`

The bootstrap endpoints do not have an adjacent published checksum, so the runtime
manifest records verification as `unavailable` instead of inventing authority.
Downloaded bytes and observed provenance will be recorded by the installer lockfile.

`claude-code-rev` is research-only and is not a dependency, installer source, or
distributed artifact. OpenWork never emits auth payloads; it reduces
`claude auth status --json` to authenticated, unauthenticated, or unknown.
