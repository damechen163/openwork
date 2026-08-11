//! The `openwork run` scheduler: drives a runtime and a sandbox backend
//! through the frozen execution state machine.
//!
//! The runtime (Claude Code / Codex) executes on the host with network access
//! and produces an analysis script; the script then runs inside the isolated
//! sandbox, whose outputs become the run's artifacts.

use openwork_core::{ErrorCode, OpenWorkError};
use openwork_execution::{
    ActorId, ApprovedMountDirectory, Artifact, DigestPinnedImageRef, Run, RunId, RunStatus,
    SandboxBackend, SandboxCommand, SandboxLimits, SandboxRequest, SandboxTermination, SandboxUser,
    UtcTimestamp,
    orchestrator::ExecutionOrchestrator,
    store::{ExecutionStore, InMemoryExecutionStore},
};
use openwork_runtime::{AgentRuntime, CancellationToken, RuntimeEventKind, RuntimeRunRequest};
use serde::Serialize;
use std::path::Path;
use std::time::Duration;

/// Actor recorded in the audit chain for runs launched by the CLI.
pub const RUN_ACTOR: &str = "user:openwork-cli";
/// Script name the runtime must produce in the workspace.
pub const ANALYZE_SCRIPT: &str = "analyze.py";
/// Artifacts required for the run to succeed.
pub const EXPECTED_ARTIFACTS: [&str; 2] = ["sales-analysis.csv", "summary.md"];
/// Default runtime timeout when the plan does not override it.
pub const DEFAULT_RUNTIME_TIMEOUT_SECONDS: u64 = 300;

/// Everything `run_loop` needs to execute one task.
pub struct RunPlan {
    pub workspace: std::path::PathBuf,
    pub prompt: String,
    pub runtime_id: String,
    pub runtime_timeout: Duration,
    pub image: DigestPinnedImageRef,
    pub user: SandboxUser,
    pub limits: SandboxLimits,
}

/// Human- and machine-readable outcome of a run.
#[derive(Debug, Serialize)]
pub struct RunReport {
    pub run_id: RunId,
    pub status: RunStatus,
    pub artifacts: Vec<Artifact>,
}

/// Drives one run through the full state machine and returns the final report.
///
/// Cancellation is owned by the caller: cancel `token` (or call
/// `backend.cancel(run_id)` from a signal handler) to abort the current phase;
/// the run then lands in `Cancelled`. `on_run_id` fires once the run exists so
/// signal handlers can target its container.
///
/// # Errors
///
/// Returns an error for pre-flight failures (unhealthy runtime, unavailable
/// sandbox backend, inaccessible workspace) and for any state-machine
/// violation; execution-phase failures are folded into the returned
/// [`RunReport`] status instead.
pub fn run_loop(
    orchestrator: &ExecutionOrchestrator<InMemoryExecutionStore>,
    runtime: &dyn AgentRuntime,
    backend: &dyn SandboxBackend,
    token: &CancellationToken,
    plan: &RunPlan,
    on_run_id: &dyn Fn(&RunId),
) -> Result<RunReport, OpenWorkError> {
    let actor = ActorId::parse(RUN_ACTOR)?;

    // Pre-flight before creating a run record: an unusable runtime or backend
    // is a caller error, not a failed run.
    let detection = runtime.detect()?;
    if detection.state != openwork_runtime::DetectionState::Healthy {
        return Err(OpenWorkError::new(
            ErrorCode::RuntimeNotFound,
            format!("runtime `{}` is not healthy on this host", plan.runtime_id),
        )
        .with_remediation("Run `openwork doctor` or `openwork runtime info` first."));
    }
    if !runtime.capabilities().run {
        return Err(OpenWorkError::new(
            ErrorCode::RuntimeUnhealthy,
            format!("runtime `{}` does not support execution", plan.runtime_id),
        ));
    }
    backend.health()?;

    let workspace = std::fs::canonicalize(&plan.workspace).map_err(|error| {
        OpenWorkError::new(
            ErrorCode::InvalidArguments,
            format!("workspace is not accessible: {error}"),
        )
    })?;
    if !workspace.is_dir() {
        return Err(OpenWorkError::new(
            ErrorCode::InvalidArguments,
            format!("workspace `{}` is not a directory", workspace.display()),
        ));
    }

    let mut run = orchestrator.create_run(
        &plan.runtime_id,
        &workspace,
        actor.clone(),
        &plan.prompt,
        UtcTimestamp::now(),
    )?;
    on_run_id(&run.id);
    let cancel_guard = CancellationGuard::new(run.id.clone(), backend);
    run = orchestrator.transition(
        &run.id,
        run.revision,
        RunStatus::Planning,
        None,
        actor.clone(),
        UtcTimestamp::now(),
    )?;
    run = orchestrator.transition(
        &run.id,
        run.revision,
        RunStatus::Running,
        None,
        actor.clone(),
        UtcTimestamp::now(),
    )?;

    let outcome = execute_phases(
        orchestrator,
        runtime,
        backend,
        token,
        plan,
        &run,
        &actor,
        &workspace,
    );
    drop(cancel_guard);

    let (status, reason) = match outcome {
        Ok(()) => (RunStatus::Succeeded, None),
        Err(RunFailure::Cancelled(reason)) => (RunStatus::Cancelled, Some(reason)),
        Err(RunFailure::TimedOut(reason)) => (RunStatus::TimedOut, Some(reason)),
        Err(RunFailure::Failed(reason)) => (RunStatus::Failed, Some(reason)),
    };
    let final_run = finalize_run(
        orchestrator,
        &run.id,
        run.revision,
        status,
        reason.as_deref(),
        actor.clone(),
    )?;

    let artifacts = orchestrator.store().artifacts(&final_run.id)?;
    Ok(RunReport {
        run_id: final_run.id.clone(),
        status: final_run.status,
        artifacts,
    })
}

enum RunFailure {
    Cancelled(String),
    TimedOut(String),
    Failed(String),
}

/// Best-effort container cleanup on every exit path of a run.
struct CancellationGuard<'a> {
    run_id: RunId,
    backend: &'a dyn SandboxBackend,
}

impl<'a> CancellationGuard<'a> {
    fn new(run_id: RunId, backend: &'a dyn SandboxBackend) -> Self {
        Self { run_id, backend }
    }
}

impl Drop for CancellationGuard<'_> {
    fn drop(&mut self) {
        let _ = self.backend.cleanup(&self.run_id);
    }
}

#[allow(clippy::too_many_arguments, clippy::too_many_lines)]
fn execute_phases(
    orchestrator: &ExecutionOrchestrator<InMemoryExecutionStore>,
    runtime: &dyn AgentRuntime,
    backend: &dyn SandboxBackend,
    token: &CancellationToken,
    plan: &RunPlan,
    run: &Run,
    actor: &ActorId,
    workspace: &Path,
) -> Result<(), RunFailure> {
    scaffold_claude_settings(workspace);

    let events = runtime
        .run(
            &RuntimeRunRequest {
                prompt: plan.prompt.clone(),
                working_directory: Some(workspace.to_path_buf()),
                timeout_seconds: Some({
                    let seconds = plan.runtime_timeout.as_secs();
                    if seconds == 0 {
                        DEFAULT_RUNTIME_TIMEOUT_SECONDS
                    } else {
                        seconds
                    }
                }),
            },
            token,
        )
        .map_err(|error| classify_runtime_error(&error))?;
    if events
        .last()
        .is_some_and(|event| event.kind == RuntimeEventKind::Cancelled)
    {
        return Err(RunFailure::Cancelled("cancelled before start".to_owned()));
    }
    if token.is_cancelled() {
        return Err(RunFailure::Cancelled("cancelled by user".to_owned()));
    }

    let script = workspace.join(ANALYZE_SCRIPT);
    if !script.is_file() {
        return Err(RunFailure::Failed(format!(
            "{ANALYZE_SCRIPT} was not produced in the workspace"
        )));
    }

    let output_dir = workspace.join("output");
    std::fs::create_dir_all(&output_dir).map_err(|error| {
        RunFailure::Failed(format!("could not prepare output directory: {error}"))
    })?;
    let approved_root = workspace
        .parent()
        .ok_or_else(|| RunFailure::Failed("workspace has no parent directory".to_owned()))?;
    let input_mount = ApprovedMountDirectory::under_root(workspace, approved_root)
        .map_err(|error| RunFailure::Failed(format!("input mount rejected: {error}")))?;
    let output_mount = ApprovedMountDirectory::under_root(&output_dir, approved_root)
        .map_err(|error| RunFailure::Failed(format!("output mount rejected: {error}")))?;

    let request = SandboxRequest::new(
        run.id.clone(),
        plan.image.clone(),
        SandboxCommand::new(
            "/usr/local/bin/python3",
            vec![
                format!("/workspace/{ANALYZE_SCRIPT}"),
                "/workspace/output".to_owned(),
            ],
            std::collections::BTreeMap::new(),
        )
        .map_err(|error| RunFailure::Failed(format!("sandbox command rejected: {error}")))?,
        plan.user,
        input_mount,
        output_mount,
        plan.limits,
    )
    .map_err(|error| RunFailure::Failed(format!("sandbox request rejected: {error}")))?;

    let result = backend
        .execute(&request)
        .map_err(|error| RunFailure::Failed(format!("sandbox execution failed: {error}")))?;
    if token.is_cancelled() || result.termination == SandboxTermination::Cancelled {
        return Err(RunFailure::Cancelled("cancelled during sandbox".to_owned()));
    }
    match result.termination {
        SandboxTermination::Exited => {
            if result.exit_code != Some(0) {
                return Err(RunFailure::Failed(format!(
                    "sandbox script exited with code {}: {}",
                    result.exit_code.unwrap_or_default(),
                    result.stderr.trim()
                )));
            }
        }
        SandboxTermination::TimedOut => {
            return Err(RunFailure::TimedOut(format!(
                "sandbox exceeded {}s",
                plan.limits.timeout_seconds()
            )));
        }
        SandboxTermination::OutOfMemory => {
            return Err(RunFailure::Failed("sandbox ran out of memory".to_owned()));
        }
        SandboxTermination::Cancelled => {
            return Err(RunFailure::Cancelled("cancelled during sandbox".to_owned()));
        }
        SandboxTermination::Failed => {
            return Err(RunFailure::Failed("sandbox failed to start".to_owned()));
        }
    }

    let missing: Vec<String> = EXPECTED_ARTIFACTS
        .iter()
        .filter(|expected| {
            let expected: &str = expected;
            !result
                .output_paths
                .iter()
                .any(|path| path.as_str() == expected)
        })
        .map(|expected| (*expected).to_owned())
        .collect();
    if !missing.is_empty() {
        return Err(RunFailure::Failed(format!(
            "sandbox produced no {}",
            missing.join(", ")
        )));
    }

    let artifacts = orchestrator
        .record_artifacts(
            &run.id,
            &output_dir,
            &result.output_paths,
            actor.clone(),
            UtcTimestamp::now(),
        )
        .map_err(|error| RunFailure::Failed(format!("artifact scan failed: {error}")))?;
    if artifacts.is_empty() {
        return Err(RunFailure::Failed(
            "no artifacts were recorded for the run".to_owned(),
        ));
    }
    Ok(())
}

fn classify_runtime_error(error: &OpenWorkError) -> RunFailure {
    match error.code {
        ErrorCode::RunCancelled => RunFailure::Cancelled("cancelled by user".to_owned()),
        ErrorCode::RunTimedOut => RunFailure::TimedOut("runtime exceeded its timeout".to_owned()),
        _ => RunFailure::Failed(format!("runtime failed: {error}")),
    }
}

fn finalize_run(
    orchestrator: &ExecutionOrchestrator<InMemoryExecutionStore>,
    run_id: &RunId,
    expected_revision: u64,
    status: RunStatus,
    reason: Option<&str>,
    actor: ActorId,
) -> Result<Run, OpenWorkError> {
    let current = orchestrator
        .store()
        .get_run(run_id)?
        .ok_or_else(|| OpenWorkError::new(ErrorCode::Internal, "run vanished from the store"))?;
    if current.status.is_terminal() {
        return Ok(current);
    }
    orchestrator.transition(
        run_id,
        expected_revision,
        status,
        reason,
        actor,
        UtcTimestamp::now(),
    )
}

fn scaffold_claude_settings(workspace: &Path) {
    let settings_dir = workspace.join(".claude");
    let settings_file = settings_dir.join("settings.json");
    if settings_file.exists() {
        return;
    }
    if std::fs::create_dir_all(&settings_dir).is_err() {
        return;
    }
    let settings = serde_json::json!({
        "permissions": {
            "write": ["**"],
            "edit": ["**"],
            "bash": []
        }
    });
    if let Ok(content) = serde_json::to_string_pretty(&settings) {
        let _ = std::fs::write(settings_file, content);
    }
}
