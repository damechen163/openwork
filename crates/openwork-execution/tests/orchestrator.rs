use openwork_execution::orchestrator::ExecutionOrchestrator;
use openwork_execution::store::InMemoryExecutionStore;
use openwork_execution::{ActorId, UtcTimestamp, sha256_bytes};

#[test]
fn run_creation_persists_only_the_prompt_digest() {
    let workspace = tempfile::tempdir().expect("workspace");
    let orchestrator = ExecutionOrchestrator::new(InMemoryExecutionStore::default());
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
