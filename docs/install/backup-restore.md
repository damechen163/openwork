# Backup and restore

Implementation is tracked in Issue #24. Backups will include database dumps, manifest,
controlled secrets strategy, uploads, pack versions, and image/version metadata, but never
temporary sandbox directories. Restore must check compatibility before mutation.
