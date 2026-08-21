//! Execution-path proof that malformed provider output fails the persisted run.

use openwork_core::{ErrorCode, OpenWorkError};
use openwork_execution::artifact::ArtifactScanner;
use openwork_execution::orchestrator::ExecutionOrchestrator;
use openwork_execution::store::{ExecutionStore, InMemoryExecutionStore};
use openwork_execution::{
    ActorId, ApprovedMountDirectory, DigestPinnedImageRef, EXECUTION_SCHEMA_VERSION, RunId,
    RunStatus, SandboxBackend, SandboxCleanupStatus, SandboxCommand, SandboxLimits, SandboxRequest,
    SandboxResult, SandboxTermination, SandboxUser, UtcTimestamp,
};
use openwork_runtime::task::{ClaudeTaskAdapter, RuntimeTaskAdapter, decode_sandbox_result};
use std::collections::BTreeMap;
use std::fs;

struct MalformedOutputSandbox;

impl SandboxBackend for MalformedOutputSandbox {
    fn health(&self) -> Result<(), OpenWorkError> {
        Ok(())
    }

    fn execute(&self, request: &SandboxRequest) -> Result<SandboxResult, OpenWorkError> {
        Ok(SandboxResult {
            schema_version: EXECUTION_SCHEMA_VERSION,
            run_id: request.run_id.clone(),
            sandbox_id: "malformed-provider".to_owned(),
            termination: SandboxTermination::Exited,
            exit_code: Some(0),
            stdout: "not-json\n".to_owned(),
            stderr: String::new(),
            truncated: false,
            started_at: UtcTimestamp::now(),
            completed_at: UtcTimestamp::now(),
            output_paths: Vec::new(),
            cleanup: SandboxCleanupStatus::Succeeded,
        })
    }

    fn cancel(&self, _run_id: &RunId) -> Result<(), OpenWorkError> {
        Ok(())
    }

    fn cleanup(&self, _run_id: &RunId) -> Result<(), OpenWorkError> {
        Ok(())
    }
}

#[test]
fn malformed_provider_output_fails_the_persisted_run() {
    let fixture = tempfile::tempdir().expect("fixture");
    let workspace = directory(fixture.path(), "workspace");
    let input_root = directory(fixture.path(), "inputs");
    let input = directory(&input_root, "run");
    let output_root = directory(fixture.path(), "outputs");
    let output = directory(&output_root, "run");
    let actor = ActorId::parse("test:runtime-output").expect("actor");
    let orchestrator = ExecutionOrchestrator::new(
        InMemoryExecutionStore::default(),
        ArtifactScanner::new(1024 * 1024).expect("scanner"),
    );
    let run = orchestrator
        .create_run(
            "claude-code",
            &workspace,
            actor.clone(),
            "private prompt sentinel",
            UtcTimestamp::now(),
        )
        .expect("create run");
    let request = sandbox_request(run.id.clone(), &input_root, &input, &output_root, &output);
    let mut decoder = ClaudeTaskAdapter::new("/usr/bin/claude").decoder(run.id.clone());

    let error = orchestrator
        .execute_with_output_processor(
            &run,
            &MalformedOutputSandbox,
            &request,
            actor,
            UtcTimestamp::now(),
            |result| decode_sandbox_result(result, decoder.as_mut()).map(|_| ()),
        )
        .expect_err("malformed provider output must fail closed");

    assert_eq!(error.code, ErrorCode::ExecutionFailed);
    let stored = orchestrator
        .store()
        .get_run(&run.id)
        .expect("read run")
        .expect("stored run");
    assert_eq!(stored.status, RunStatus::Failed);
    assert_eq!(
        stored.terminal_reason.as_deref(),
        Some("runtime output validation failed")
    );
}

fn sandbox_request(
    run_id: RunId,
    input_root: &std::path::Path,
    input: &std::path::Path,
    output_root: &std::path::Path,
    output: &std::path::Path,
) -> SandboxRequest {
    SandboxRequest::new(
        run_id,
        DigestPinnedImageRef::parse(format!(
            "docker.io/library/busybox@sha256:{}",
            "a".repeat(64)
        ))
        .expect("image"),
        SandboxCommand::new("/usr/bin/claude", Vec::new(), BTreeMap::new()).expect("command"),
        SandboxUser::new(65_532, 65_532).expect("user"),
        ApprovedMountDirectory::under_root(input, input_root).expect("input"),
        ApprovedMountDirectory::under_root(output, output_root).expect("output"),
        SandboxLimits::new(500, 64 * 1024 * 1024, 32, 30, 256 * 1024).expect("limits"),
    )
    .expect("request")
}

fn directory(root: &std::path::Path, name: &str) -> std::path::PathBuf {
    let path = root.join(name);
    fs::create_dir(&path).expect("directory");
    path
}
