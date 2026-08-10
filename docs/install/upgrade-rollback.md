# Upgrade and rollback

Implementation is tracked in Issue #25. Upgrade will preflight, back up, record digests,
migrate, start, health-check, and smoke-test. Failure must stop the rollout and preserve a
clear rollback path; mixed silent versions are prohibited.
