-- Add missing audit event types that were omitted from the initial migration.
ALTER TABLE audit_events DROP CONSTRAINT IF EXISTS audit_events_event_type_check;
ALTER TABLE audit_events ADD CONSTRAINT audit_events_event_type_check CHECK (event_type IN (
    'run_created', 'runtime_selected', 'sandbox_created', 'action_requested',
    'policy_allowed', 'policy_denied', 'approval_requested', 'approval_approved',
    'approval_denied', 'approval_expired', 'approval_consumed', 'runtime_started',
    'runtime_output', 'artifact_created', 'runtime_completed', 'sandbox_destroyed',
    'run_completed', 'run_failed', 'approval_binding_mismatch'
));

-- Durable proof that one exact approved action was claimed for execution.
-- Unique constraints serve as the final defense against replay.
CREATE TABLE action_claims (
    approval_id UUID PRIMARY KEY REFERENCES approval_requests(id),
    run_id UUID NOT NULL REFERENCES runs(id),
    action_id UUID NOT NULL UNIQUE,
    parameter_hash CHAR(64) NOT NULL CHECK (parameter_hash ~ '^[0-9a-f]{64}$'),
    actor_id TEXT NOT NULL CHECK (length(actor_id) BETWEEN 1 AND 256 AND btrim(actor_id) <> ''),
    claimed_at TIMESTAMPTZ NOT NULL
);
