//! Deterministic vertical-slice integration test that exercises the full
//! orchestrator execution chain with an in-memory store and a fake sandbox
//! backend. No network, no Docker, no external process.

use openwork_core::{ErrorCode, OpenWorkError};
use openwork_execution::{
    ActorId, ApprovedMountDirectory, AuditEventType, DigestPinnedImageRef,
    EXECUTION_SCHEMA_VERSION, RelativeArtifactPath, RunStatus, SandboxBackend,
    SandboxCleanupStatus, SandboxLimits, SandboxRequest, SandboxResult, SandboxTermination,
    SandboxUser, SandboxWorkingDirectory, UtcTimestamp,
    artifact::ArtifactScanner,
    orchestrator::ExecutionOrchestrator,
    sha256_bytes,
    store::{ExecutionStore, InMemoryExecutionStore},
};
use openwork_runtime::task::{
    CLAUDE_RUNTIME_ID, ClaudeTaskAdapter, RuntimeTaskAdapter, into_sandbox_request,
};
use std::fs;
use std::path::PathBuf;
use std::sync::Mutex;

// ---------------------------------------------------------------------------
// Golden fixtures
// ---------------------------------------------------------------------------

const JULY_CSV: &str = include_str!("../../../samples/sales/sales_july.csv");
const AUGUST_CSV: &str = include_str!("../../../samples/sales/sales_august.csv");
const GOLDEN_ANALYSIS: &str = include_str!("../../../samples/sales/golden/sales-analysis.csv");
const GOLDEN_SUMMARY: &str = include_str!("../../../samples/sales/golden/summary.md");

// ---------------------------------------------------------------------------
// Fake sandbox backend
// ---------------------------------------------------------------------------

/// A deterministic [`SandboxBackend`] that writes pre-supplied files into the
/// output directory and returns a successful result.  This replaces a real
/// Docker container for offline CI runs.
struct FakeSandbox {
    /// (`relative_path`, `file_content`) pairs to write into the output mount.
    output_files: Vec<(String, String)>,
    /// Call log for test assertions.
    calls: Mutex<Vec<String>>,
}

impl FakeSandbox {
    fn new(output_files: Vec<(String, String)>) -> Self {
        Self {
            output_files,
            calls: Mutex::new(Vec::new()),
        }
    }

    fn call_log(&self) -> Vec<String> {
        self.calls.lock().expect("calls lock").clone()
    }
}

impl SandboxBackend for FakeSandbox {
    fn health(&self) -> Result<(), OpenWorkError> {
        self.calls
            .lock()
            .expect("calls lock")
            .push("health".to_owned());
        Ok(())
    }

    fn execute(&self, request: &SandboxRequest) -> Result<SandboxResult, OpenWorkError> {
        self.calls
            .lock()
            .expect("calls lock")
            .push("execute".to_owned());

        // Write every pre-supplied output file into the output mount.
        let mut output_paths = Vec::new();
        for (relative, content) in &self.output_files {
            let full = request.output_directory.as_path().join(relative);
            if let Some(parent) = full.parent() {
                fs::create_dir_all(parent).expect("fake output parent");
            }
            fs::write(&full, content).expect("fake output write");
            output_paths.push(
                RelativeArtifactPath::parse(relative.as_str())
                    .expect("fake output path must be valid"),
            );
        }

        let started_at = UtcTimestamp::now();
        let completed_at = UtcTimestamp::now();
        let result = SandboxResult {
            schema_version: EXECUTION_SCHEMA_VERSION,
            run_id: request.run_id.clone(),
            sandbox_id: "fake-sandbox-1".to_owned(),
            termination: SandboxTermination::Exited,
            exit_code: Some(0),
            stdout: String::new(),
            stderr: String::new(),
            truncated: false,
            started_at,
            completed_at,
            output_paths,
            cleanup: SandboxCleanupStatus::Succeeded,
        };
        // Panic on validation failure so the test fails fast.
        result
            .validate()
            .expect("fake sandbox result must validate");
        Ok(result)
    }

    fn cancel(&self, _run_id: &openwork_execution::RunId) -> Result<(), OpenWorkError> {
        self.calls
            .lock()
            .expect("calls lock")
            .push("cancel".to_owned());
        Ok(())
    }

    fn cleanup(&self, _run_id: &openwork_execution::RunId) -> Result<(), OpenWorkError> {
        self.calls
            .lock()
            .expect("calls lock")
            .push("cleanup".to_owned());
        Ok(())
    }
}

struct FailingSandbox;

impl SandboxBackend for FailingSandbox {
    fn health(&self) -> Result<(), OpenWorkError> {
        Ok(())
    }

    fn execute(&self, _request: &SandboxRequest) -> Result<SandboxResult, OpenWorkError> {
        Err(OpenWorkError::new(
            ErrorCode::ExecutionFailed,
            "simulated sandbox failure",
        ))
    }

    fn cancel(&self, _run_id: &openwork_execution::RunId) -> Result<(), OpenWorkError> {
        Ok(())
    }

    fn cleanup(&self, _run_id: &openwork_execution::RunId) -> Result<(), OpenWorkError> {
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// Fixture helper
// ---------------------------------------------------------------------------

struct Fixture {
    _root: tempfile::TempDir,
    input_dir: PathBuf,
    output_dir: PathBuf,
    workspace: PathBuf,
}

impl Fixture {
    fn new() -> Self {
        let root = tempfile::tempdir().expect("fixture root");
        let workspace = root.path().join("workspace");
        let input_root = root.path().join("approved-inputs");
        let output_root = root.path().join("approved-outputs");
        let input_dir = input_root.join("run");
        let output_dir = output_root.join("run");
        for dir in [
            &workspace,
            &input_root,
            &output_root,
            &input_dir,
            &output_dir,
        ] {
            fs::create_dir(dir).expect("fixture directory");
        }
        // Write input CSV files into the sandbox input mount.
        fs::write(input_dir.join("july.csv"), JULY_CSV).expect("write july fixture");
        fs::write(input_dir.join("august.csv"), AUGUST_CSV).expect("write august fixture");
        Self {
            _root: root,
            input_dir,
            output_dir,
            workspace,
        }
    }
}

fn prepare_sandbox_request(
    run: &openwork_execution::Run,
    prompt: &str,
    fixture: &Fixture,
    image_digest_character: char,
    capabilities: &[&str],
) -> SandboxRequest {
    let task = openwork_execution::RuntimeTask {
        schema_version: EXECUTION_SCHEMA_VERSION,
        run_id: run.id.clone(),
        runtime: CLAUDE_RUNTIME_ID.to_owned(),
        prompt: prompt.to_owned(),
        prompt_hash: sha256_bytes(prompt.as_bytes()),
        working_directory: SandboxWorkingDirectory::parse("/workspace")
            .expect("valid working directory"),
        timeout_seconds: 300,
        capabilities: capabilities.iter().map(ToString::to_string).collect(),
    };
    task.validate().expect("task must be valid");

    let invocation = ClaudeTaskAdapter::new("/usr/bin/claude")
        .prepare(&task)
        .expect("prepare invocation");
    let image = DigestPinnedImageRef::parse(format!(
        "ghcr.io/openwork/sandbox@sha256:{}",
        image_digest_character.to_string().repeat(64)
    ))
    .expect("valid pinned image");
    let user = SandboxUser::new(65_532, 65_532).expect("non-root user");
    let input_mount = ApprovedMountDirectory::under_root(
        &fixture.input_dir,
        fixture.input_dir.parent().expect("input root"),
    )
    .expect("approved input mount");
    let output_mount = ApprovedMountDirectory::under_root(
        &fixture.output_dir,
        fixture.output_dir.parent().expect("output root"),
    )
    .expect("approved output mount");
    let limits =
        SandboxLimits::new(1000, 268_435_456, 128, 300, 1_048_576).expect("valid sandbox limits");

    into_sandbox_request(
        invocation,
        run.id.clone(),
        image,
        user,
        input_mount,
        output_mount,
        limits,
    )
    .expect("sandbox request")
}

fn assert_successful_run(final_run: &openwork_execution::Run) {
    assert_eq!(final_run.status, RunStatus::Succeeded);
    assert!(final_run.completed_at.is_some(), "completed_at must be set");
    assert!(
        final_run.started_at.is_some(),
        "started_at must be set on transition to Running"
    );
    assert!(
        final_run.terminal_reason.is_none(),
        "successful runs must not have a terminal reason"
    );
    assert_eq!(final_run.revision, 3, "expected three transitions");
}

fn assert_golden_artifacts(
    orchestrator: &ExecutionOrchestrator<InMemoryExecutionStore>,
    run_id: &openwork_execution::RunId,
) {
    let artifacts = orchestrator
        .store()
        .artifacts(run_id)
        .expect("read artifacts");
    assert_eq!(artifacts.len(), 2, "exactly two artifacts expected");

    let analysis = artifacts
        .iter()
        .find(|artifact| artifact.path.as_str() == "sales-analysis.csv")
        .expect("sales-analysis.csv artifact");
    let summary = artifacts
        .iter()
        .find(|artifact| artifact.path.as_str() == "summary.md")
        .expect("summary.md artifact");
    assert_eq!(analysis.media_type, "text/csv");
    assert_eq!(summary.media_type, "text/markdown");
    assert_eq!(analysis.sha256, sha256_bytes(GOLDEN_ANALYSIS.as_bytes()));
    assert_eq!(summary.sha256, sha256_bytes(GOLDEN_SUMMARY.as_bytes()));
    assert_eq!(analysis.size_bytes.get(), GOLDEN_ANALYSIS.len() as u64);
    assert_eq!(summary.size_bytes.get(), GOLDEN_SUMMARY.len() as u64);
}

fn assert_audit_chain(
    orchestrator: &ExecutionOrchestrator<InMemoryExecutionStore>,
    run_id: &openwork_execution::RunId,
) {
    let events = orchestrator
        .store()
        .audit_events(run_id)
        .expect("read audit events");
    assert!(
        events.len() >= 6,
        "expected the complete pipeline audit trail"
    );
    events[0]
        .verify_integrity(1, None)
        .expect("genesis event must start the chain");
    for window in events.windows(2) {
        window[1]
            .verify_integrity(window[1].sequence, Some(window[0].event_hash()))
            .expect("audit chain must be intact");
        assert!(window[1].timestamp >= window[0].timestamp);
    }
    assert_eq!(events[0].event_type, AuditEventType::RunCreated);
    assert!(matches!(
        events.last().expect("last event").event_type,
        AuditEventType::RunCompleted | AuditEventType::ArtifactCreated
    ));
}

// ---------------------------------------------------------------------------
// The pipeline test
// ---------------------------------------------------------------------------

#[test]
fn full_execution_pipeline_sales_analysis_vertical_slice() {
    // ---- Layer 1: setup ------------------------------------------------
    let fixture = Fixture::new();

    let store = InMemoryExecutionStore::default();
    let scanner =
        ArtifactScanner::new(100 * 1024 * 1024).expect("artifact scanner with 100 MiB limit");
    let orchestrator = ExecutionOrchestrator::new(store, scanner);

    let actor = ActorId::parse("test:runner").expect("valid actor");
    let prompt = "Analyze july.csv and august.csv in /workspace/input.\n\
                  Write the output CSV to /workspace/output/sales-analysis.csv\n\
                  and a summary to /workspace/output/summary.md.";

    // ---- Step 1: create run --------------------------------------------
    let now = UtcTimestamp::now();
    let run = orchestrator
        .create_run(
            CLAUDE_RUNTIME_ID,
            &fixture.workspace,
            actor.clone(),
            prompt,
            now,
        )
        .expect("create run");

    assert_eq!(run.status, RunStatus::Queued);
    assert_eq!(run.revision, 0);
    assert_eq!(run.runtime, CLAUDE_RUNTIME_ID);

    // Transition Queued → Planning (required before execute can proceed to Running).
    let run = orchestrator
        .transition(
            &run.id,
            run.revision,
            RunStatus::Planning,
            None,
            actor.clone(),
            UtcTimestamp::now(),
        )
        .expect("transition to Planning");
    assert_eq!(run.status, RunStatus::Planning);
    assert_eq!(run.revision, 1);

    let sandbox_request = prepare_sandbox_request(
        &run,
        prompt,
        &fixture,
        'a',
        &["filesystem.read", "filesystem.write"],
    );

    // Verify the request was assembled correctly.
    assert_eq!(sandbox_request.run_id, run.id);
    assert!(
        !sandbox_request.command.stdin().is_empty(),
        "stdin must carry the prompt"
    );
    assert_eq!(
        sandbox_request.command.program(),
        "/usr/bin/claude",
        "program comes from the adapter"
    );

    // ---- Step 4: execute via test sandbox ------------------------------
    let sandbox = FakeSandbox::new(vec![
        ("sales-analysis.csv".to_owned(), GOLDEN_ANALYSIS.to_owned()),
        ("summary.md".to_owned(), GOLDEN_SUMMARY.to_owned()),
    ]);

    let final_run = orchestrator
        .execute(
            &run,
            &sandbox,
            &sandbox_request,
            actor.clone(),
            UtcTimestamp::now(),
        )
        .expect("orchestrator execute must succeed");
    assert_successful_run(&final_run);
    assert_golden_artifacts(&orchestrator, &run.id);
    assert_audit_chain(&orchestrator, &run.id);
    assert!(sandbox.call_log().contains(&"execute".to_owned()));

    // Verify the original run is retrievable by id.
    let stored = orchestrator
        .store()
        .get_run(&run.id)
        .expect("get stored run")
        .expect("run must exist in store");
    assert_eq!(stored.status, RunStatus::Succeeded);
    assert_eq!(stored.id, run.id);
}

// ---------------------------------------------------------------------------
// Additional pipeline tests
// ---------------------------------------------------------------------------

/// Exercises the full chain end-to-end with a minimal single-artifact output
/// and confirms the audit chain is 1-based, hash-linked, and timestamp-ordered.
#[test]
fn pipeline_single_artifact_minimal_chain() {
    let root = tempfile::tempdir().expect("root");
    let workspace = root.path().join("workspace");
    let input_root = root.path().join("inputs");
    let output_root = root.path().join("outputs");
    let input_dir = input_root.join("run");
    let output_dir = output_root.join("run");
    for dir in [
        &workspace,
        &input_root,
        &output_root,
        &input_dir,
        &output_dir,
    ] {
        fs::create_dir(dir).expect("dir");
    }

    let store = InMemoryExecutionStore::default();
    let scanner = ArtifactScanner::new(1024 * 1024).expect("scanner");
    let orchestrator = ExecutionOrchestrator::new(store, scanner);

    let actor = ActorId::parse("test:minimal").expect("actor");
    let prompt = "Write one file.";

    let now = UtcTimestamp::now();
    let run = orchestrator
        .create_run("claude-code", &workspace, actor.clone(), prompt, now)
        .expect("create run");

    let task = openwork_execution::RuntimeTask {
        schema_version: EXECUTION_SCHEMA_VERSION,
        run_id: run.id.clone(),
        runtime: "claude-code".to_owned(),
        prompt: prompt.to_owned(),
        prompt_hash: sha256_bytes(prompt.as_bytes()),
        working_directory: SandboxWorkingDirectory::parse("/workspace")
            .expect("valid working directory"),
        timeout_seconds: 300,
        capabilities: vec!["filesystem.read".to_owned()],
    };
    task.validate().expect("task valid");

    let adapter = ClaudeTaskAdapter::new("/usr/bin/claude");
    let invocation = adapter.prepare(&task).expect("prepare");

    let image = DigestPinnedImageRef::parse(format!(
        "ghcr.io/openwork/sandbox@sha256:{}",
        "b".repeat(64)
    ))
    .expect("image");

    let user = SandboxUser::new(65_532, 65_532).expect("user");

    let input_mount =
        ApprovedMountDirectory::under_root(&input_dir, input_dir.parent().expect("input root"))
            .expect("input mount");

    let output_mount =
        ApprovedMountDirectory::under_root(&output_dir, output_dir.parent().expect("output root"))
            .expect("output mount");

    let limits = SandboxLimits::new(1000, 268_435_456, 128, 300, 1_048_576).expect("limits");

    let sandbox_request = into_sandbox_request(
        invocation,
        run.id.clone(),
        image,
        user,
        input_mount,
        output_mount,
        limits,
    )
    .expect("sandbox request");

    // Single output file.
    let sandbox = FakeSandbox::new(vec![("result.txt".to_owned(), "done\n".to_owned())]);

    let final_run = orchestrator
        .execute(
            &run,
            &sandbox,
            &sandbox_request,
            actor.clone(),
            UtcTimestamp::now(),
        )
        .expect("execute");

    assert_eq!(final_run.status, RunStatus::Succeeded);

    let artifacts = orchestrator.store().artifacts(&run.id).expect("artifacts");
    assert_eq!(artifacts.len(), 1);
    assert_eq!(artifacts[0].path.as_str(), "result.txt");
    assert_eq!(artifacts[0].media_type, "text/plain");

    let events = orchestrator
        .store()
        .audit_events(&run.id)
        .expect("audit events");
    // Genesis event at sequence 1.
    assert_eq!(events[0].sequence, 1);
    assert!(events[0].previous_hash.is_none());
}

/// Ensures that sandbox execution failures propagate correctly through the
/// orchestrator and the run lands in Failed status.
#[test]
fn pipeline_sandbox_failure_records_failed_state() {
    let root = tempfile::tempdir().expect("root");
    let workspace = root.path().join("workspace");
    let input_root = root.path().join("inputs");
    let output_root = root.path().join("outputs");
    let input_dir = input_root.join("run");
    let output_dir = output_root.join("run");
    for dir in [
        &workspace,
        &input_root,
        &output_root,
        &input_dir,
        &output_dir,
    ] {
        fs::create_dir(dir).expect("dir");
    }

    let store = InMemoryExecutionStore::default();
    let scanner = ArtifactScanner::new(1024 * 1024).expect("scanner");
    let orchestrator = ExecutionOrchestrator::new(store, scanner);

    let actor = ActorId::parse("test:failure").expect("actor");
    let prompt = "This will fail.";

    let now = UtcTimestamp::now();
    let run = orchestrator
        .create_run("claude-code", &workspace, actor.clone(), prompt, now)
        .expect("create run");

    // Transition to Planning.
    let run = orchestrator
        .transition(
            &run.id,
            run.revision,
            RunStatus::Planning,
            None,
            actor.clone(),
            UtcTimestamp::now(),
        )
        .expect("transition to Planning");

    let task = openwork_execution::RuntimeTask {
        schema_version: EXECUTION_SCHEMA_VERSION,
        run_id: run.id.clone(),
        runtime: "claude-code".to_owned(),
        prompt: prompt.to_owned(),
        prompt_hash: sha256_bytes(prompt.as_bytes()),
        working_directory: SandboxWorkingDirectory::parse("/workspace")
            .expect("valid working directory"),
        timeout_seconds: 300,
        capabilities: vec!["filesystem.read".to_owned()],
    };
    task.validate().expect("task valid");

    let adapter = ClaudeTaskAdapter::new("/usr/bin/claude");
    let invocation = adapter.prepare(&task).expect("prepare");

    let image = DigestPinnedImageRef::parse(format!(
        "ghcr.io/openwork/sandbox@sha256:{}",
        "c".repeat(64)
    ))
    .expect("image");

    let user = SandboxUser::new(65_532, 65_532).expect("user");

    let input_mount =
        ApprovedMountDirectory::under_root(&input_dir, input_dir.parent().expect("input root"))
            .expect("input mount");

    let output_mount =
        ApprovedMountDirectory::under_root(&output_dir, output_dir.parent().expect("output root"))
            .expect("output mount");

    let limits = SandboxLimits::new(1000, 268_435_456, 128, 300, 1_048_576).expect("limits");

    let sandbox_request = into_sandbox_request(
        invocation,
        run.id.clone(),
        image,
        user,
        input_mount,
        output_mount,
        limits,
    )
    .expect("sandbox request");

    let failing_sandbox = FailingSandbox;
    let result = orchestrator.execute(
        &run,
        &failing_sandbox,
        &sandbox_request,
        actor.clone(),
        UtcTimestamp::now(),
    );

    assert!(
        result.is_err(),
        "orchestrator must propagate sandbox failure"
    );

    // The orchestrator's error handler records a best-effort transition to
    // Failed so the run is terminal even when the sandbox fails.
    let stored_run = orchestrator
        .store()
        .get_run(&run.id)
        .expect("get run")
        .expect("run must exist");
    assert_eq!(stored_run.status, RunStatus::Failed);
    assert!(stored_run.terminal_reason.is_some());
    assert!(stored_run.completed_at.is_some());
}
