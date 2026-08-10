use openwork_execution::approval::ApprovalRepository;
use openwork_execution::audit::AuditAppend;
use openwork_execution::store::{ExecutionStore, InMemoryExecutionStore};
use openwork_execution::{
    ActionId, ActionRequest, ActorId, ApprovalDecision, ApprovalId, ApprovalRequest,
    ApprovalStatus, EXECUTION_SCHEMA_VERSION, Run, RunId, RunStatus, UtcTimestamp, sha256_bytes,
};
use serde_json::json;
use std::path::PathBuf;
use std::sync::{Arc, Barrier};

#[test]
fn approval_is_single_use_and_claims_the_exact_action() {
    let store = seeded_store();
    let action = action("finance@example.com");
    let approval = pending_approval(&action);
    assert!(
        store
            .create_approval(approval.clone(), actor("attacker"), time(0))
            .is_err()
    );
    assert!(store.get_approval(&approval.id).unwrap().is_none());
    store
        .create_approval(approval.clone(), actor("requester"), time(0))
        .expect("create approval");
    let approved = store
        .decide_approval(
            &approval.id,
            0,
            ApprovalDecision::Approved,
            actor("admin"),
            Some("token=must-not-persist"),
            time(1),
        )
        .expect("approve");
    assert_eq!(approved.status, ApprovalStatus::Approved);
    assert_eq!(
        approved.decision.as_ref().expect("decision").actor,
        actor("admin")
    );
    assert!(
        !serde_json::to_string(&approved)
            .unwrap()
            .contains("must-not-persist")
    );

    let consumed = store
        .consume_approval(&approval.id, 1, &action, actor("executor"), time(2))
        .expect("consume once");
    assert_eq!(consumed.approval.status, ApprovalStatus::Consumed);
    assert_eq!(
        consumed.action_claim.parameter_hash,
        *action.parameter_hash()
    );
    assert!(
        store
            .consume_approval(&approval.id, 2, &action, actor("executor"), time(3))
            .is_err()
    );
    assert_eq!(
        store.get_action_claim(&action.id).unwrap(),
        Some(consumed.action_claim)
    );
    assert_eq!(
        store
            .audit_events(&run_id())
            .unwrap()
            .last()
            .unwrap()
            .event_type,
        openwork_execution::AuditEventType::ApprovalConsumed
    );
    verify_audit(&store, 4);
}

#[test]
fn expiry_uses_now_greater_than_or_equal_to_deadline() {
    let store = seeded_store();
    let action = action("internal@example.com");
    let approval = pending_approval(&action);
    store
        .create_approval(approval.clone(), actor("requester"), time(0))
        .expect("create approval");
    assert!(
        store
            .decide_approval(
                &approval.id,
                0,
                ApprovalDecision::Approved,
                actor("admin"),
                None,
                time(5),
            )
            .is_err()
    );
    let expired = store
        .expire_approval(&approval.id, 0, actor("system"), time(5))
        .expect("expire at deadline");
    assert_eq!(expired.status, ApprovalStatus::Expired);
    assert_eq!(
        store
            .audit_events(&run_id())
            .unwrap()
            .last()
            .unwrap()
            .event_type,
        openwork_execution::AuditEventType::ApprovalExpired
    );
    assert!(
        store
            .consume_approval(&approval.id, 1, &action, actor("executor"), time(5))
            .is_err()
    );
    verify_audit(&store, 3);

    let approved_store = seeded_store();
    let approved_request = pending_approval(&action);
    approved_store
        .create_approval(approved_request.clone(), actor("requester"), time(0))
        .unwrap();
    approved_store
        .decide_approval(
            &approved_request.id,
            0,
            ApprovalDecision::Approved,
            actor("admin"),
            None,
            time(1),
        )
        .unwrap();
    let expired_approved = approved_store
        .expire_approval(&approved_request.id, 1, actor("system"), time(5))
        .unwrap();
    assert_eq!(expired_approved.status, ApprovalStatus::Expired);
    assert!(expired_approved.decision.is_some());
    verify_audit(&approved_store, 4);
}

#[test]
fn tampered_action_is_rejected_and_safely_audited() {
    let store = seeded_store();
    let action = action("finance@example.com");
    let approval = pending_approval(&action);
    store
        .create_approval(approval.clone(), actor("requester"), time(0))
        .unwrap();
    store
        .decide_approval(
            &approval.id,
            0,
            ApprovalDecision::Approved,
            actor("admin"),
            None,
            time(1),
        )
        .unwrap();
    let mut tampered = action.clone();
    tampered.resource = "attacker@example.net".to_owned();
    assert!(
        store
            .consume_approval(&approval.id, 1, &tampered, actor("executor"), time(2))
            .is_err()
    );
    assert!(store.get_action_claim(&action.id).unwrap().is_none());
    assert_eq!(
        store.get_approval(&approval.id).unwrap().unwrap().revision,
        1
    );
    let audit = store.audit_events(&run_id()).unwrap();
    assert_eq!(
        audit.last().unwrap().event_type,
        openwork_execution::AuditEventType::ApprovalBindingMismatch
    );
    assert!(
        !serde_json::to_string(&audit)
            .unwrap()
            .contains("attacker@example.net")
    );
    verify_audit(&store, 4);
}

#[test]
fn concurrent_consumers_have_one_cas_winner() {
    let store = Arc::new(seeded_store());
    let action = action("finance@example.com");
    let approval = pending_approval(&action);
    store
        .create_approval(approval.clone(), actor("requester"), time(0))
        .unwrap();
    store
        .decide_approval(
            &approval.id,
            0,
            ApprovalDecision::Approved,
            actor("admin"),
            None,
            time(1),
        )
        .unwrap();
    let barrier = Arc::new(Barrier::new(3));
    let attempts = ["executor-a", "executor-b"].map(|name| {
        let store = Arc::clone(&store);
        let barrier = Arc::clone(&barrier);
        let approval_id = approval.id.clone();
        let action = action.clone();
        std::thread::spawn(move || {
            barrier.wait();
            store.consume_approval(&approval_id, 1, &action, actor(name), time(2))
        })
    });
    barrier.wait();
    let results = attempts.map(|handle| handle.join().unwrap());
    assert_eq!(results.iter().filter(|result| result.is_ok()).count(), 1);
    assert!(store.get_action_claim(&action.id).unwrap().is_some());
    verify_audit(&store, 4);
}

#[test]
fn concurrent_approve_and_deny_have_one_cas_winner() {
    let store = Arc::new(seeded_store());
    let action = action("finance@example.com");
    let approval = pending_approval(&action);
    store
        .create_approval(approval.clone(), actor("requester"), time(0))
        .unwrap();
    let barrier = Arc::new(Barrier::new(3));
    let attempts = [ApprovalDecision::Approved, ApprovalDecision::Denied].map(|decision| {
        let store = Arc::clone(&store);
        let barrier = Arc::clone(&barrier);
        let approval_id = approval.id.clone();
        std::thread::spawn(move || {
            barrier.wait();
            store.decide_approval(&approval_id, 0, decision, actor("admin"), None, time(1))
        })
    });
    barrier.wait();
    let results = attempts.map(|handle| handle.join().unwrap());
    assert_eq!(results.iter().filter(|result| result.is_ok()).count(), 1);
    assert_eq!(
        store.get_approval(&approval.id).unwrap().unwrap().revision,
        1
    );
    verify_audit(&store, 3);
}

fn seeded_store() -> InMemoryExecutionStore {
    let store = InMemoryExecutionStore::default();
    store
        .create_run(
            Run {
                schema_version: EXECUTION_SCHEMA_VERSION,
                id: run_id(),
                runtime: "mock".to_owned(),
                workspace: PathBuf::from("/workspace"),
                status: RunStatus::Queued,
                revision: 0,
                actor_id: actor("requester"),
                prompt_sha256: sha256_bytes(b"prompt"),
                created_at: time(0),
                updated_at: time(0),
                started_at: None,
                completed_at: None,
                terminal_reason: None,
            },
            AuditAppend::new(actor("requester"), time(0)),
        )
        .unwrap();
    store
}

fn action(resource: &str) -> ActionRequest {
    ActionRequest::new(
        ActionId::parse("01890f3e-a5f1-7cc2-98c0-5f9c6f5e7a02").unwrap(),
        run_id(),
        "email.send",
        resource,
        json!({"attachment": "sales-analysis.csv"}),
    )
    .unwrap()
}

fn pending_approval(action: &ActionRequest) -> ApprovalRequest {
    ApprovalRequest {
        schema_version: EXECUTION_SCHEMA_VERSION,
        id: ApprovalId::parse("01890f3e-a5f1-7cc2-98c0-5f9c6f5e7a03").unwrap(),
        run_id: action.run_id.clone(),
        action_id: action.id.clone(),
        parameter_hash: action.parameter_hash().clone(),
        requested_by: actor("requester"),
        request_reason: "external email requires review".to_owned(),
        created_at: time(0),
        expires_at: time(5),
        status: ApprovalStatus::Pending,
        revision: 0,
        decision: None,
        consumed_at: None,
    }
}

fn verify_audit(store: &InMemoryExecutionStore, expected: usize) {
    let events = store.audit_events(&run_id()).unwrap();
    assert_eq!(events.len(), expected);
    for (index, event) in events.iter().enumerate() {
        let previous = index
            .checked_sub(1)
            .map(|offset| events[offset].event_hash());
        event
            .verify_integrity((index + 1) as u64, previous)
            .unwrap();
    }
}

fn run_id() -> RunId {
    RunId::parse("01890f3e-a5f1-7cc2-98c0-5f9c6f5e7a01").unwrap()
}

fn actor(name: &str) -> ActorId {
    ActorId::parse(format!("user:{name}")).unwrap()
}

fn time(minute: u8) -> UtcTimestamp {
    UtcTimestamp::parse(format!("2026-08-10T00:{minute:02}:00Z")).unwrap()
}
