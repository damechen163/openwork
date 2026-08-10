# Platform support evidence

Support claims use four distinct evidence levels. A fixture or successful compile
is not presented as real-host validation.

| Target | Tier | Compile | Fixtures | CI smoke | Real host |
| --- | --- | --- | --- | --- | --- |
| macOS arm64 | 1 | yes | yes | pending matrix | macOS 15 arm64, 2026-08-10 |
| macOS x64 | 1 | planned | yes | pending matrix | not yet |
| Ubuntu x64 | 1 | planned | yes | pending matrix | not yet |
| Ubuntu arm64 | 1 | planned | yes | pending matrix | not yet |
| Windows 11 x64 | 1 | planned | yes | pending matrix | not yet |
| WSL2 | 1 | planned | yes | pending manual smoke | not yet |
| Windows arm64 | 2 | planned | yes | not required | not yet |
| Debian | 2 | planned | Linux fixtures | not required | not yet |

The platform detector is read-only. Docker is reported when present but is not a
Bootstrap prerequisite. Unsupported operating systems or architectures fail before
installation planning with an actionable error.
