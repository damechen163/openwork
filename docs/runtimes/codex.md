# Codex runtime

Codex is an **external-managed** Apache-2.0 runtime. OpenWork detects native and
WSL installations separately, preserves healthy or broken installations, and does
not silently repair, replace, uninstall, or perform login for the user.

Managed plans use only OpenAI's documented endpoints:

- macOS, Linux, and WSL: `https://chatgpt.com/codex/install.sh`
- Windows: `https://chatgpt.com/codex/install.ps1`

The downloaded bootstrap script receives an observed checksum. The official script
then resolves release metadata and verifies the released binary checksum; those two
authorities are recorded separately by the lockfile.

A wrapper on `PATH` is not enough to be healthy. A nonzero version check, missing
vendor executable, timeout, or similar launch failure is `broken` with redacted
details. OpenWork reports the problem before offering an explicit managed update.
