//! Transactional execution persistence boundary and deterministic memory store.

use crate::audit::AuditAppend;
use crate::{Artifact, AuditEvent, AuditEventType, Run, RunId, RunStatus};
use openwork_core::{ErrorCode, OpenWorkError, redact_text};
use std::collections::BTreeMap;
use std::sync::Mutex;

/// Storage transaction boundary implemented by memory storage now and Postgres later.
pub trait ExecutionStore: Send + Sync {
    /// Creates a queued run and its genesis audit event atomically.
    ///
    /// # Errors
    ///
    /// Returns an error for invalid state, duplicate identity, or storage failure.
    fn create_run(&self, run: Run, audit: AuditAppend) -> Result<Run, OpenWorkError>;
    /// Applies a revision-checked transition and its audit event atomically.
    ///
    /// # Errors
    ///
    /// Returns an error for a stale revision, illegal transition, or storage failure.
    fn transition_run(
        &self,
        run_id: &RunId,
        expected_revision: u64,
        next: RunStatus,
        reason: Option<&str>,
        audit: AuditAppend,
    ) -> Result<Run, OpenWorkError>;
    /// Appends one centrally redacted event at the next per-run sequence.
    ///
    /// # Errors
    ///
    /// Returns an error when the run is absent or persistence fails.
    fn append_audit(
        &self,
        run_id: &RunId,
        event_type: AuditEventType,
        audit: AuditAppend,
    ) -> Result<AuditEvent, OpenWorkError>;
    /// Persists a complete artifact batch or none of it.
    ///
    /// # Errors
    ///
    /// Returns an error for a missing run, duplicate path, mismatch, or storage failure.
    fn record_artifacts(
        &self,
        run_id: &RunId,
        artifacts: Vec<Artifact>,
    ) -> Result<(), OpenWorkError>;
    /// Reads one run.
    ///
    /// # Errors
    ///
    /// Returns an error when storage cannot be read.
    fn get_run(&self, run_id: &RunId) -> Result<Option<Run>, OpenWorkError>;
    /// Reads a run's ordered audit chain.
    ///
    /// # Errors
    ///
    /// Returns an error when storage cannot be read.
    fn audit_events(&self, run_id: &RunId) -> Result<Vec<AuditEvent>, OpenWorkError>;
    /// Reads a run's artifacts.
    ///
    /// # Errors
    ///
    /// Returns an error when storage cannot be read.
    fn artifacts(&self, run_id: &RunId) -> Result<Vec<Artifact>, OpenWorkError>;
}

/// Deterministic single-process store used by local mode and tests.
#[derive(Default)]
pub struct InMemoryExecutionStore {
    state: Mutex<State>,
}

#[derive(Default)]
struct State {
    runs: BTreeMap<RunId, Run>,
    audits: BTreeMap<RunId, Vec<AuditEvent>>,
    artifacts: BTreeMap<RunId, Vec<Artifact>>,
}

impl ExecutionStore for InMemoryExecutionStore {
    fn create_run(&self, run: Run, audit: AuditAppend) -> Result<Run, OpenWorkError> {
        validate_new_run(&run)?;
        if audit.timestamp != run.created_at {
            return Err(state_error(
                "genesis audit timestamp must match run creation",
            ));
        }
        let mut state = self.lock()?;
        if state.runs.contains_key(&run.id) {
            return Err(state_error("run already exists"));
        }
        let event = audit.build(run.id.clone(), 1, AuditEventType::RunCreated, None)?;
        state.audits.insert(run.id.clone(), vec![event]);
        state.runs.insert(run.id.clone(), run.clone());
        Ok(run)
    }

    fn transition_run(
        &self,
        run_id: &RunId,
        expected_revision: u64,
        next: RunStatus,
        reason: Option<&str>,
        audit: AuditAppend,
    ) -> Result<Run, OpenWorkError> {
        let mut state = self.lock()?;
        let current = state.runs.get(run_id).ok_or_else(run_missing)?.clone();
        if current.revision != expected_revision || !current.status.can_transition_to(next) {
            return Err(state_error(
                "run revision is stale or transition is illegal",
            ));
        }
        if audit.timestamp < current.updated_at {
            return Err(state_error("run timestamps cannot move backwards"));
        }
        let events = state.audits.get(run_id).ok_or_else(audit_missing)?;
        if events
            .last()
            .is_some_and(|event| audit.timestamp < event.timestamp)
        {
            return Err(state_error("audit timestamps cannot move backwards"));
        }
        let sequence = u64::try_from(events.len())
            .map_err(|_| internal_error("audit sequence overflow"))?
            .checked_add(1)
            .ok_or_else(|| internal_error("audit sequence overflow"))?;
        let previous = events.last().map(|event| event.event_hash().clone());
        let event = audit.build(run_id.clone(), sequence, transition_event(next), previous)?;

        let mut updated = current;
        updated.status = next;
        updated.revision = updated
            .revision
            .checked_add(1)
            .ok_or_else(|| state_error("run revision overflow"))?;
        updated.updated_at = audit.timestamp;
        if next == RunStatus::Running && updated.started_at.is_none() {
            updated.started_at = Some(audit.timestamp);
        }
        if next.is_terminal() {
            updated.completed_at = Some(audit.timestamp);
            updated.terminal_reason = (next != RunStatus::Succeeded)
                .then(|| reason.map_or_else(|| "unspecified".to_owned(), redact_text));
        }
        state.runs.insert(run_id.clone(), updated.clone());
        state
            .audits
            .get_mut(run_id)
            .ok_or_else(audit_missing)?
            .push(event);
        Ok(updated)
    }

    fn append_audit(
        &self,
        run_id: &RunId,
        event_type: AuditEventType,
        audit: AuditAppend,
    ) -> Result<AuditEvent, OpenWorkError> {
        let mut state = self.lock()?;
        if !state.runs.contains_key(run_id) {
            return Err(run_missing());
        }
        let events = state.audits.get(run_id).ok_or_else(audit_missing)?;
        if events
            .last()
            .is_some_and(|event| audit.timestamp < event.timestamp)
        {
            return Err(state_error("audit timestamps cannot move backwards"));
        }
        let sequence = u64::try_from(events.len())
            .map_err(|_| internal_error("audit sequence overflow"))?
            .checked_add(1)
            .ok_or_else(|| internal_error("audit sequence overflow"))?;
        let previous = events.last().map(|event| event.event_hash().clone());
        let event = audit.build(run_id.clone(), sequence, event_type, previous)?;
        state
            .audits
            .get_mut(run_id)
            .ok_or_else(audit_missing)?
            .push(event.clone());
        Ok(event)
    }

    fn record_artifacts(
        &self,
        run_id: &RunId,
        artifacts: Vec<Artifact>,
    ) -> Result<(), OpenWorkError> {
        let mut state = self.lock()?;
        if !state.runs.contains_key(run_id)
            || artifacts.iter().any(|artifact| &artifact.run_id != run_id)
        {
            return Err(OpenWorkError::new(
                ErrorCode::ArtifactInvalid,
                "artifact run mismatch",
            ));
        }
        let existing = state.artifacts.entry(run_id.clone()).or_default();
        if artifacts.iter().any(|candidate| {
            existing.iter().any(|stored| stored.path == candidate.path)
                || artifacts
                    .iter()
                    .filter(|item| item.path == candidate.path)
                    .count()
                    > 1
        }) {
            return Err(OpenWorkError::new(
                ErrorCode::ArtifactInvalid,
                "duplicate artifact path",
            ));
        }
        existing.extend(artifacts);
        Ok(())
    }

    fn get_run(&self, run_id: &RunId) -> Result<Option<Run>, OpenWorkError> {
        Ok(self.lock()?.runs.get(run_id).cloned())
    }

    fn audit_events(&self, run_id: &RunId) -> Result<Vec<AuditEvent>, OpenWorkError> {
        Ok(self.lock()?.audits.get(run_id).cloned().unwrap_or_default())
    }

    fn artifacts(&self, run_id: &RunId) -> Result<Vec<Artifact>, OpenWorkError> {
        Ok(self
            .lock()?
            .artifacts
            .get(run_id)
            .cloned()
            .unwrap_or_default())
    }
}

impl InMemoryExecutionStore {
    fn lock(&self) -> Result<std::sync::MutexGuard<'_, State>, OpenWorkError> {
        self.state
            .lock()
            .map_err(|_| internal_error("execution store lock poisoned"))
    }
}

fn validate_new_run(run: &Run) -> Result<(), OpenWorkError> {
    if run.status != RunStatus::Queued
        || run.revision != 0
        || run.runtime.trim().is_empty()
        || run.workspace.as_os_str().is_empty()
        || run.updated_at != run.created_at
        || run.started_at.is_some()
        || run.completed_at.is_some()
        || run.terminal_reason.is_some()
    {
        return Err(state_error("new run invariants are invalid"));
    }
    Ok(())
}

const fn transition_event(status: RunStatus) -> AuditEventType {
    match status {
        RunStatus::Planning => AuditEventType::RuntimeSelected,
        RunStatus::AwaitingApproval => AuditEventType::ApprovalRequested,
        RunStatus::Running => AuditEventType::RuntimeStarted,
        RunStatus::Succeeded => AuditEventType::RunCompleted,
        RunStatus::Failed | RunStatus::Cancelled | RunStatus::TimedOut | RunStatus::Queued => {
            AuditEventType::RunFailed
        }
    }
}

fn state_error(message: &str) -> OpenWorkError {
    OpenWorkError::new(ErrorCode::InvalidStateTransition, message)
}

fn run_missing() -> OpenWorkError {
    state_error("run does not exist")
}

fn audit_missing() -> OpenWorkError {
    internal_error("run audit chain does not exist")
}

fn internal_error(message: &str) -> OpenWorkError {
    OpenWorkError::new(ErrorCode::Internal, message)
}
