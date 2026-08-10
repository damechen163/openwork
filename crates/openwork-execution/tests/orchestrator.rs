use openwork_execution::artifact::ArtifactScanner;
use openwork_execution::orchestrator::ExecutionOrchestrator;
use openwork_execution::store::{ExecutionStore, InMemoryExecutionStore};
use openwork_execution::{
    ActorId, AuditEventType, RelativeArtifactPath, UtcTimestamp, sha256_bytes,
};
use std::fs;

#[test]
fn run_creation_persists_only_the_prompt_digest() {
    let workspace = tempfile::tempdir().expect("workspace");
    let orchestrator = ExecutionOrchestrator::new(
        InMemoryExecutionStore::default(),
        ArtifactScanner::new(1024).expect("scanner"),
    );
    let run = orchestrator
        .create_run(
            "mock",
            workspace.path(),
            ActorId::parse("user:test").expect("actor"),
            "private enterprise prompt",
            UtcTimestamp::parse("2026-08-10T00:00:00Z").expect("timestamp"),
        )
        .expect("create run");
    assert_eq!(
        run.prompt_sha256,
        sha256_bytes(b"private enterprise prompt")
    );
    assert!(
        !serde_json::to_string(&run)
            .expect("serialize")
            .contains("private enterprise prompt")
    );
}

#[test]
fn invalid_artifact_batch_is_not_partially_persisted() {
    let workspace = tempfile::tempdir().expect("workspace");
    fs::write(workspace.path().join("valid.txt"), b"valid").expect("output");
    let orchestrator = ExecutionOrchestrator::new(
        InMemoryExecutionStore::default(),
        ArtifactScanner::new(1024).expect("scanner"),
    );
    let now = UtcTimestamp::parse("2026-08-10T00:00:00Z").expect("timestamp");
    let run = orchestrator
        .create_run(
            "mock",
            workspace.path(),
            ActorId::parse("user:test").expect("actor"),
            "prompt",
            now,
        )
        .expect("run");
    let paths = [
        RelativeArtifactPath::parse("valid.txt").expect("valid path"),
        RelativeArtifactPath::parse("missing.txt").expect("missing path"),
    ];
    assert!(
        orchestrator
            .record_artifacts(
                &run.id,
                workspace.path(),
                &paths,
                ActorId::parse("system:scanner").expect("actor"),
                now,
            )
            .is_err()
    );
    assert!(
        orchestrator
            .store()
            .artifacts(&run.id)
            .expect("artifacts")
            .is_empty()
    );
    assert_eq!(
        orchestrator
            .store()
            .audit_events(&run.id)
            .expect("audit after rejection")
            .len(),
        1
    );

    orchestrator
        .record_artifacts(
            &run.id,
            workspace.path(),
            std::slice::from_ref(&paths[0]),
            ActorId::parse("system:scanner").expect("actor"),
            now,
        )
        .expect("record artifact");
    let events = orchestrator.store().audit_events(&run.id).expect("audit");
    assert_eq!(
        events.last().expect("artifact event").event_type,
        AuditEventType::ArtifactCreated
    );
}
