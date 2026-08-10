use openwork_execution::audit::AuditAppend;
use openwork_execution::store::{ExecutionStore, InMemoryExecutionStore};
use openwork_execution::{
    ActorId, Artifact, ArtifactId, ArtifactSizeBytes, AuditEventType, EXECUTION_SCHEMA_VERSION,
    RelativeArtifactPath, Run, RunId, RunStatus, UtcTimestamp, sha256_bytes,
};
use std::path::PathBuf;
use std::sync::{Arc, Barrier};

#[test]
fn illegal_transition_is_atomic_and_audit_is_redacted() {
    let store = InMemoryExecutionStore::default();
    let run = queued_run();
    store
        .create_run(run.clone(), audit("2026-08-10T00:00:00Z"))
        .expect("create run");
    let illegal = store.transition_run(
        &run.id,
        0,
        RunStatus::Succeeded,
        None,
        audit("2026-08-10T00:00:01Z"),
    );
    assert!(illegal.is_err());
    assert_eq!(store.get_run(&run.id).expect("read run"), Some(run.clone()));

    let event = store
        .append_audit(
            &run.id,
            AuditEventType::RuntimeOutput,
            AuditAppend::new(actor(), timestamp("2026-08-10T00:00:02Z")),
        )
        .expect("append event");
    let encoded = serde_json::to_string(&event).expect("serialize event");
    assert_eq!(event.metadata.as_map().len(), 0);
    assert!(!encoded.contains("Authorization"));

    let events = store.audit_events(&run.id).expect("read audit");
    events[0].verify_integrity(1, None).expect("genesis");
    events[1]
        .verify_integrity(2, Some(events[0].event_hash()))
        .expect("second event");
}

#[test]
fn persistence_rechecks_public_contract_fields() {
    let store = InMemoryExecutionStore::default();
    let mut invalid_run = queued_run();
    invalid_run.runtime = "x".repeat(129);
    assert!(
        store
            .create_run(invalid_run, audit("2026-08-10T00:00:00Z"))
            .is_err()
    );

    let run = queued_run();
    store
        .create_run(run.clone(), audit("2026-08-10T00:00:00Z"))
        .expect("create valid run");
    let invalid_artifact = Artifact {
        schema_version: EXECUTION_SCHEMA_VERSION,
        id: ArtifactId::parse("01890f3e-a5f1-7cc2-98c0-5f9c6f5e7a02").expect("UUIDv7"),
        run_id: run.id.clone(),
        path: RelativeArtifactPath::parse("report.txt").expect("path"),
        media_type: String::new(),
        size_bytes: ArtifactSizeBytes::new(1).expect("size"),
        sha256: sha256_bytes(b"x"),
        created_at: timestamp("2026-08-10T00:00:01Z"),
    };
    assert!(
        store
            .record_artifacts(&run.id, vec![invalid_artifact])
            .is_err()
    );
    assert!(store.artifacts(&run.id).expect("artifacts").is_empty());
}

#[test]
fn concurrent_cancel_and_complete_have_one_cas_winner() {
    let store = Arc::new(InMemoryExecutionStore::default());
    let run = queued_run();
    store
        .create_run(run.clone(), audit("2026-08-10T00:00:00Z"))
        .expect("create run");
    store
        .transition_run(
            &run.id,
            0,
            RunStatus::Planning,
            None,
            audit("2026-08-10T00:00:01Z"),
        )
        .expect("plan");
    store
        .transition_run(
            &run.id,
            1,
            RunStatus::Running,
            None,
            audit("2026-08-10T00:00:02Z"),
        )
        .expect("start");

    let barrier = Arc::new(Barrier::new(3));
    let attempts = [RunStatus::Cancelled, RunStatus::Succeeded].map(|status| {
        let store = Arc::clone(&store);
        let barrier = Arc::clone(&barrier);
        let run_id = run.id.clone();
        std::thread::spawn(move || {
            barrier.wait();
            store.transition_run(
                &run_id,
                2,
                status,
                Some("token=must-not-persist"),
                audit("2026-08-10T00:00:03Z"),
            )
        })
    });
    barrier.wait();
    let results = attempts.map(|handle| handle.join().expect("thread"));
    assert_eq!(results.iter().filter(|result| result.is_ok()).count(), 1);
    let stored = store
        .get_run(&run.id)
        .expect("read run")
        .expect("stored run");
    assert!(stored.status.is_terminal());
    assert_eq!(stored.revision, 3);
    let events = store.audit_events(&run.id).expect("audit");
    assert_eq!(events.len(), 4);
    assert_eq!(
        events.last().expect("terminal event").metadata.as_map()["run_status"],
        serde_json::to_value(stored.status).expect("status")
    );
    assert!(
        !stored
            .terminal_reason
            .as_deref()
            .unwrap_or_default()
            .contains("must-not-persist")
    );
}

fn queued_run() -> Run {
    let now = timestamp("2026-08-10T00:00:00Z");
    Run {
        schema_version: EXECUTION_SCHEMA_VERSION,
        id: RunId::parse("01890f3e-a5f1-7cc2-98c0-5f9c6f5e7a01").expect("UUIDv7"),
        runtime: "mock".to_owned(),
        workspace: PathBuf::from("/workspace"),
        status: RunStatus::Queued,
        revision: 0,
        actor_id: actor(),
        prompt_sha256: sha256_bytes(b"enterprise prompt"),
        created_at: now,
        updated_at: now,
        started_at: None,
        completed_at: None,
        terminal_reason: None,
    }
}

fn audit(value: &str) -> AuditAppend {
    AuditAppend::new(actor(), timestamp(value))
}

fn actor() -> ActorId {
    ActorId::parse("user:test").expect("actor")
}

fn timestamp(value: &str) -> UtcTimestamp {
    UtcTimestamp::parse(value).expect("timestamp")
}
