# Adapters

Adapters isolate OpenWork contracts from upstream-specific APIs and CLIs. Business code
must not query an upstream component’s private database. Each adapter requires health,
credential validation, compatibility tests, license notes, and a pinned upstream version.
