//! Scheduler tests for `openwork run`: mock runtime + mock sandbox through
//! the real execution state machine.

use openwork_cli::run::{RunPlan, run_loop};
use openwork_core::{ErrorCode, OpenWorkError};
use openwork_execution::{
    AuditEventType, DigestPinnedImageRef, RunStatus, SandboxLimits, SandboxResult,
    SandboxTermination, SandboxUser, SchemaVersion, UtcTimestamp,
    artifact::ArtifactScanner,
    orchestrator::ExecutionOrchestrator,
    store::{ExecutionStore, InMemoryExecutionStore},
};
use openwork_runtime::{
    AgentRuntime, AuthStatus, CancellationToken, DetectionState, DistributionModel,
    RuntimeCapabilities, RuntimeDetection, RuntimeDoctorCheck, RuntimeEvent, RuntimeEventKind,
    RuntimeId, RuntimeInstallOutcome, RuntimeInstallPlan, RuntimeMetadata, RuntimeResult,
    RuntimeRunRequest,
};
use openwork_sandbox::mock::MockSandboxBackend;
use std::collections::VecDeque;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::Duration;

struct FakeRuntime {
    events: Mutex<VecDeque<Result<Vec<RuntimeEvent>, OpenWorkError>>>,
}

impl FakeRuntime {
    fn new(outcomes: Vec<Result<Vec<RuntimeEvent>, OpenWorkError>>) -> Self {
        Self {
            events: Mutex::new(outcomes.into()),
        }
    }
}

fn completed() -> Vec<RuntimeEvent> {
    vec![
        RuntimeEvent {
            kind: RuntimeEventKind::Started,
            message: "started".to_owned(),
        },
        RuntimeEvent {
            kind: RuntimeEventKind::Completed,
            message: "completed".to_owned(),
        },
    ]
}

impl AgentRuntime for FakeRuntime {
    fn metadata(&self) -> RuntimeMetadata {
        RuntimeMetadata {
            id: RuntimeId::from("fake"),
            name: "Fake".to_owned(),
            upstream: "fixture".to_owned(),
            license: "fixture".to_owned(),
            distribution: DistributionModel::ExternalManaged,
        }
    }
    fn detect(&self) -> RuntimeResult<RuntimeDetection> {
        Ok(RuntimeDetection {
            state: DetectionState::Healthy,
            executable: Some(PathBuf::from("/fixture/runtime")),
            details: None,
        })
    }
    fn install_plan(&self, _: Option<&str>) -> RuntimeResult<RuntimeInstallPlan> {
        Err(OpenWorkError::new(ErrorCode::Internal, "not supported"))
    }
    fn install(&self, _: &RuntimeInstallPlan) -> RuntimeResult<RuntimeInstallOutcome> {
        Err(OpenWorkError::new(ErrorCode::Internal, "not supported"))
    }
    fn uninstall(&self) -> RuntimeResult<()> {
        Err(OpenWorkError::new(ErrorCode::Internal, "not supported"))
    }
    fn version(&self) -> RuntimeResult<Option<String>> {
        Ok(None)
    }
    fn update(&self, _: Option<&str>) -> RuntimeResult<RuntimeInstallOutcome> {
        Err(OpenWorkError::new(ErrorCode::Internal, "not supported"))
    }
    fn doctor(&self) -> RuntimeResult<Vec<RuntimeDoctorCheck>> {
        Ok(Vec::new())
    }
    fn auth_status(&self) -> RuntimeResult<AuthStatus> {
        Ok(AuthStatus::Unknown)
    }
    fn capabilities(&self) -> RuntimeCapabilities {
        RuntimeCapabilities {
            install: false,
            uninstall: false,
            update: false,
            authenticate: false,
            run: true,
            cancel: true,
        }
    }
    fn run(
        &self,
        _: &RuntimeRunRequest,
        _: &CancellationToken,
    ) -> RuntimeResult<Vec<RuntimeEvent>> {
        self.events.lock().unwrap().pop_front().unwrap_or_else(|| {
            Err(OpenWorkError::new(
                ErrorCode::Internal,
                "no scripted events",
            ))
        })
    }
    fn cancel(&self, _: &CancellationToken) -> RuntimeResult<()> {
        Ok(())
    }
}

fn fixture_plan(workspace: &std::path::Path) -> RunPlan {
    RunPlan {
        workspace: workspace.to_path_buf(),
        prompt: "analyze the sales data".to_owned(),
        runtime_id: "fake".to_owned(),
        runtime_timeout: Duration::from_mins(1),
        image: DigestPinnedImageRef::parse(format!(
            "docker.io/library/python@sha256:{}",
            "a".repeat(64)
        ))
        .unwrap(),
        user: SandboxUser::new(1000, 1000).unwrap(),
        limits: SandboxLimits::new(30_000, 512 * 1024 * 1024, 512, 60, 16 * 1024 * 1024).unwrap(),
    }
}

fn orchestrator() -> ExecutionOrchestrator<InMemoryExecutionStore> {
    ExecutionOrchestrator::new(
        InMemoryExecutionStore::default(),
        ArtifactScanner::new(1024 * 1024).unwrap(),
    )
}

fn sandbox_result(termination: SandboxTermination, exit_code: Option<i32>) -> SandboxResult {
    let now = UtcTimestamp::now();
    SandboxResult {
        schema_version: SchemaVersion,
        run_id: openwork_execution::RunId::generate(),
        sandbox_id: "mock".to_owned(),
        termination,
        exit_code,
        stdout: String::new(),
        stderr: String::new(),
        truncated: false,
        started_at: now,
        completed_at: now,
        output_paths: Vec::new(),
        cleanup: openwork_execution::SandboxCleanupStatus::Succeeded,
    }
}

fn sha256_of(bytes: &[u8]) -> String {
    use sha2::{Digest, Sha256};
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let digest = Sha256::digest(bytes);
    let mut encoded = String::with_capacity(64);
    for byte in digest {
        encoded.push(HEX[(byte >> 4) as usize] as char);
        encoded.push(HEX[(byte & 0x0f) as usize] as char);
    }
    encoded
}

#[test]
fn happy_path_records_artifacts_and_succeeds() {
    let workspace = tempfile::tempdir().unwrap();
    std::fs::write(workspace.path().join("analyze.py"), "fixture").unwrap();
    let runtime = Arc::new(FakeRuntime::new(vec![Ok(completed())]));
    let backend = Arc::new(
        MockSandboxBackend::new()
            .with_output_file(
                "sales-analysis.csv",
                "region,units,revenue\nnorth,10,100.0\n",
            )
            .with_output_file("summary.md", "# Sales Summary\nTotal: 100.0\n"),
    );
    backend.enqueue(Ok(sandbox_result(SandboxTermination::Exited, Some(0))));
    let orchestrator = orchestrator();
    let plan = fixture_plan(workspace.path());

    let report = run_loop(
        &orchestrator,
        runtime.as_ref(),
        backend.as_ref(),
        &CancellationToken::new(),
        &plan,
        &|_| {},
        &|_| {},
    )
    .expect("run must succeed");

    assert_eq!(report.status, RunStatus::Succeeded);
    assert_eq!(report.artifacts.len(), 2);
    let analysis = report
        .artifacts
        .iter()
        .find(|artifact| artifact.path.as_str() == "sales-analysis.csv")
        .expect("analysis artifact");
    assert_eq!(
        analysis.sha256.as_str(),
        sha256_of(b"region,units,revenue\nnorth,10,100.0\n")
    );

    // Audit chain ends with the completed run.
    let audit = orchestrator.store().audit_events(&report.run_id).unwrap();
    assert_eq!(
        audit.last().expect("audit events").event_type,
        AuditEventType::RunCompleted
    );
}

#[test]
fn missing_script_fails_the_run() {
    let workspace = tempfile::tempdir().unwrap();
    let runtime = Arc::new(FakeRuntime::new(vec![Ok(completed())]));
    let backend = Arc::new(MockSandboxBackend::new());
    let orchestrator = orchestrator();
    let plan = fixture_plan(workspace.path());
    let run_id = Arc::new(Mutex::new(None));
    let register = {
        let run_id = run_id.clone();
        move |id: &openwork_execution::RunId| {
            *run_id.lock().unwrap() = Some(id.clone());
        }
    };

    let report = run_loop(
        &orchestrator,
        runtime.as_ref(),
        backend.as_ref(),
        &CancellationToken::new(),
        &plan,
        &register,
        &|_| {},
    )
    .expect("run must complete");

    assert_eq!(report.status, RunStatus::Failed);
    let run_id = run_id.lock().unwrap().clone().expect("run id");
    let run = orchestrator.store().get_run(&run_id).unwrap().unwrap();
    assert_eq!(run.status, RunStatus::Failed);
    assert!(
        run.terminal_reason
            .as_deref()
            .unwrap_or_default()
            .contains("analyze.py")
    );
}

#[test]
fn sandbox_nonzero_exit_fails_the_run() {
    let workspace = tempfile::tempdir().unwrap();
    std::fs::write(workspace.path().join("analyze.py"), "fixture").unwrap();
    let runtime = Arc::new(FakeRuntime::new(vec![Ok(completed())]));
    let backend = Arc::new(MockSandboxBackend::new());
    backend.enqueue(Ok(sandbox_result(SandboxTermination::Exited, Some(1))));
    let orchestrator = orchestrator();
    let plan = fixture_plan(workspace.path());

    let report = run_loop(
        &orchestrator,
        runtime.as_ref(),
        backend.as_ref(),
        &CancellationToken::new(),
        &plan,
        &|_| {},
        &|_| {},
    )
    .expect("run must complete");

    assert_eq!(report.status, RunStatus::Failed);
}

#[test]
fn sandbox_timeout_marks_the_run_timed_out() {
    let workspace = tempfile::tempdir().unwrap();
    std::fs::write(workspace.path().join("analyze.py"), "fixture").unwrap();
    let runtime = Arc::new(FakeRuntime::new(vec![Ok(completed())]));
    let backend = Arc::new(MockSandboxBackend::new());
    backend.enqueue(Ok(sandbox_result(SandboxTermination::TimedOut, None)));
    let orchestrator = orchestrator();
    let plan = fixture_plan(workspace.path());

    let report = run_loop(
        &orchestrator,
        runtime.as_ref(),
        backend.as_ref(),
        &CancellationToken::new(),
        &plan,
        &|_| {},
        &|_| {},
    )
    .expect("run must complete");

    assert_eq!(report.status, RunStatus::TimedOut);
}

#[test]
fn pre_cancelled_token_cancels_the_run() {
    let workspace = tempfile::tempdir().unwrap();
    let runtime = Arc::new(FakeRuntime::new(vec![Ok(completed())]));
    let backend = Arc::new(MockSandboxBackend::new());
    let orchestrator = orchestrator();
    let plan = fixture_plan(workspace.path());
    let token = CancellationToken::new();
    token.cancel();

    let report = run_loop(
        &orchestrator,
        runtime.as_ref(),
        backend.as_ref(),
        &token,
        &plan,
        &|_| {},
        &|_| {},
    )
    .expect("run must complete");

    assert_eq!(report.status, RunStatus::Cancelled);
}
