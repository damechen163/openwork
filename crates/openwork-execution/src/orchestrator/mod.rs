//! Thin orchestration layer that never persists raw prompts or artifact contents.

use crate::artifact::ArtifactScanner;
use crate::audit::AuditAppend;
use crate::store::ExecutionStore;
use crate::{
    ActorId, Artifact, EXECUTION_SCHEMA_VERSION, RelativeArtifactPath, Run, RunId, RunStatus,
    UtcTimestamp, sha256_bytes,
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

    #[must_use]
    pub const fn store(&self) -> &S {
        &self.store
    }
}

fn audit(actor: ActorId, timestamp: UtcTimestamp) -> AuditAppend {
    AuditAppend::new(actor, timestamp)
}
