//! Thin orchestration layer that never persists raw prompts or artifact contents.

use crate::artifact::ArtifactScanner;
use crate::audit::AuditAppend;
use crate::store::ExecutionStore;
use crate::{
    ActorId, Artifact, EXECUTION_SCHEMA_VERSION, RelativeArtifactPath, Run, RunId, RunStatus,
    SandboxBackend, SandboxRequest, UtcTimestamp, sha256_bytes,
};
use openwork_core::{ErrorCode, OpenWorkError};
use std::fs;
use std::path::Path;

/// Coordinates run persistence while delegating execution to later M1 integrations.
pub struct ExecutionOrchestrator<S> {
    store: S,
    scanner: ArtifactScanner,
}

impl<S: ExecutionStore> ExecutionOrchestrator<S> {
    #[must_use]
    pub const fn new(store: S, scanner: ArtifactScanner) -> Self {
        Self { store, scanner }
    }

    /// Creates a queued run from a trusted actor; only the prompt digest is persisted.
    ///
    /// # Errors
    ///
    /// Returns an error for an invalid runtime/workspace or a storage failure.
    pub fn create_run(
        &self,
        runtime: &str,
        workspace: &Path,
        actor: ActorId,
        prompt: &str,
        now: UtcTimestamp,
    ) -> Result<Run, OpenWorkError> {
        if runtime.trim().is_empty() || runtime.len() > 128 {
            return Err(OpenWorkError::new(
                ErrorCode::InvalidArguments,
                "invalid runtime name",
            ));
        }
        let workspace = fs::canonicalize(workspace).map_err(|_| {
            OpenWorkError::new(ErrorCode::InvalidArguments, "workspace unavailable")
        })?;
        if !fs::metadata(&workspace).is_ok_and(|metadata| metadata.is_dir()) {
            return Err(OpenWorkError::new(
                ErrorCode::InvalidArguments,
                "workspace is not a directory",
            ));
        }
        let run = Run {
            schema_version: EXECUTION_SCHEMA_VERSION,
            id: RunId::generate(),
            runtime: runtime.to_owned(),
            workspace,
            status: RunStatus::Queued,
            revision: 0,
            actor_id: actor.clone(),
            prompt_sha256: sha256_bytes(prompt.as_bytes()),
            created_at: now,
            updated_at: now,
            started_at: None,
            completed_at: None,
            terminal_reason: None,
        };
        self.store.create_run(run, audit(actor, now))
    }

    /// Revision-checks one state change and records the corresponding event atomically.
    ///
    /// # Errors
    ///
    /// Returns an error for a stale revision, illegal transition, or storage failure.
    pub fn transition(
        &self,
        run_id: &RunId,
        expected_revision: u64,
        next: RunStatus,
        reason: Option<&str>,
        actor: ActorId,
        now: UtcTimestamp,
    ) -> Result<Run, OpenWorkError> {
        self.store
            .transition_run(run_id, expected_revision, next, reason, audit(actor, now))
    }

    /// Validates every claimed output before atomically recording the artifact batch.
    ///
    /// # Errors
    ///
    /// Returns an error for unsafe output or an atomic persistence failure.
    pub fn record_artifacts(
        &self,
        run_id: &RunId,
        output_root: &Path,
        paths: &[RelativeArtifactPath],
        actor: ActorId,
        created_at: UtcTimestamp,
    ) -> Result<Vec<Artifact>, OpenWorkError> {
        let artifacts = self.scanner.scan(run_id, output_root, paths, created_at)?;
        self.store
            .record_artifacts(run_id, artifacts.clone(), audit(actor, created_at))?;
        Ok(artifacts)
    }

    /// Executes a prepared sandbox request within the run lifecycle.
    ///
    /// Transitions the run to Running, delegates to the sandbox backend,
    /// scans validated output artifacts, and records the terminal status
    /// with full audit trail. This is the primary execution path for M1.
    ///
    /// # Errors
    ///
    /// Returns an error for a stale run revision, illegal transition,
    /// sandbox failure, invalid output, or persistence failure.
    pub fn execute(
        &self,
        run: &Run,
        sandbox: &dyn SandboxBackend,
        request: &SandboxRequest,
        actor: ActorId,
        now: UtcTimestamp,
    ) -> Result<Run, OpenWorkError> {
        // Transition Queued/Planning → Running
        let run = self.transition(
            &run.id,
            run.revision,
            RunStatus::Running,
            None,
            actor.clone(),
            now,
        )?;

        // Execute in sandbox
        let result = sandbox.execute(request).map_err(|error| {
            // Best-effort: record the failure as a terminal event
            let _ = self.store.transition_run(
                &run.id,
                run.revision + 1,
                RunStatus::Failed,
                Some("sandbox execution failed"),
                audit(actor.clone(), UtcTimestamp::now()),
            );
            error
        })?;

        // Scan output artifacts (only when the container exited cleanly)
        let completed_at = UtcTimestamp::now();
        let exited_clean = matches!(
            result.termination,
            crate::SandboxTermination::Exited
        );
        let _artifacts = if exited_clean {
            match self.record_artifacts(
                &run.id,
                request.output_directory.as_path(),
                &result.output_paths,
                actor.clone(),
                completed_at,
            ) {
                Ok(artifacts) => artifacts,
                Err(error) => {
                    let _ = self.store.transition_run(
                        &run.id,
                        run.revision + 1,
                        RunStatus::Failed,
                        Some("artifact validation failed"),
                        audit(actor, completed_at),
                    );
                    return Err(error);
                }
            }
        } else {
            Vec::new()
        };

        // Determine terminal status
        let (terminal, reason) = match result.termination {
            crate::SandboxTermination::Exited if result.exit_code == Some(0) => {
                (RunStatus::Succeeded, None)
            }
            crate::SandboxTermination::Exited => (
                RunStatus::Failed,
                Some("provider exited with non-zero code"),
            ),
            crate::SandboxTermination::Cancelled => (
                RunStatus::Cancelled,
                Some("sandbox was cancelled"),
            ),
            crate::SandboxTermination::TimedOut => (
                RunStatus::TimedOut,
                Some("sandbox timed out"),
            ),
            crate::SandboxTermination::OutOfMemory => (
                RunStatus::Failed,
                Some("sandbox out of memory"),
            ),
            crate::SandboxTermination::Failed => (
                RunStatus::Failed,
                Some("sandbox execution failed"),
            ),
        };

        let reason_str: Option<&str> = reason;
        self.transition(
            &run.id,
            run.revision + 1,
            terminal,
            reason_str,
            actor,
            completed_at,
        )
    }

    #[must_use]
    pub const fn store(&self) -> &S {
        &self.store
    }
}

fn audit(actor: ActorId, timestamp: UtcTimestamp) -> AuditAppend {
    AuditAppend::new(actor, timestamp)
}
