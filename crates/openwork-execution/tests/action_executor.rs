use openwork_core::ErrorCode;
use openwork_execution::action_executor::{ActionExecutor, ClaimedAction, MockActionExecutor};
use openwork_execution::approval::{ActionClaim, ApprovalRepository};
use openwork_execution::audit::AuditAppend;
use openwork_execution::store::{ExecutionStore, InMemoryExecutionStore};
use openwork_execution::{
    ActionId, ActionRequest, ActorId, ApprovalDecision, ApprovalId, ApprovalRequest,
    ApprovalStatus, EXECUTION_SCHEMA_VERSION, Run, RunId, RunStatus, UtcTimestamp, sha256_bytes,
};
use serde_json::json;
use std::path::PathBuf;

#[test]
fn consumed_claim_allows_exact_action_once() {
    let fixture = approved_fixture();
    let executor = MockActionExecutor::default();
    let first = ClaimedAction::verify(
        &fixture.store,
        fixture.action.clone(),
        fixture.claim.clone(),
    )
    .expect("consumed exact claim");
    let replay = ClaimedAction::verify(
        &fixture.store,
        fixture.action.clone(),
        fixture.claim.clone(),
    )
    .expect("durable claim remains exact");

    let receipt = executor.execute(first).expect("first execution");

    assert_eq!(&receipt.run_id, &fixture.action.run_id);
    assert_eq!(&receipt.action_id, &fixture.action.id);
    assert_eq!(&receipt.parameter_hash, fixture.action.parameter_hash());
    assert!(executor.was_executed(&fixture.action.id).expect("state"));
    assert_eq!(
        executor.execute(replay).expect_err("replay rejected").code,
        ErrorCode::ApprovalInvalid
    );
}

#[test]
fn tampered_action_is_rejected_before_executor() {
    let fixture = approved_fixture();
    let tampered = ActionRequest::new(
        fixture.action.id.clone(),
        fixture.action.run_id.clone(),
        fixture.action.action.clone(),
        fixture.action.resource.clone(),
        json!({"amount": 999_999}),
    )
    .expect("tampered request is internally valid");

    let error = ClaimedAction::verify(&fixture.store, tampered, fixture.claim)
        .err()
        .expect("different parameter binding rejected");

    assert_eq!(error.code, ErrorCode::ApprovalInvalid);
}

#[test]
fn denied_approval_cannot_produce_executable_action() {
    let fixture = pending_fixture("01890f3e-a5f1-7cc2-98c0-5f9c6f5e7b01");
    fixture
        .store
        .decide_approval(
            &fixture.approval.id,
            fixture.approval.revision,
            ApprovalDecision::Denied,
            actor(),
            Some("unsafe recipient"),
            timestamp("2026-08-10T00:00:04Z"),
        )
        .expect("deny");

    let error = ClaimedAction::verify(
        &fixture.store,
        fixture.action.clone(),
        forged_claim(&fixture.action),
    )
    .err()
    .expect("denied approval has no consumed claim");

    assert_eq!(error.code, ErrorCode::ApprovalInvalid);
    assert!(
        fixture
            .store
            .get_action_claim(&fixture.action.id)
            .expect("claim read")
            .is_none()
    );
}

#[test]
fn expired_approval_cannot_produce_executable_action() {
    let fixture = pending_fixture("01890f3e-a5f1-7cc2-98c0-5f9c6f5e7c01");
    let approved = fixture
        .store
        .decide_approval(
            &fixture.approval.id,
            fixture.approval.revision,
            ApprovalDecision::Approved,
            actor(),
            None,
            timestamp("2026-08-10T00:00:04Z"),
        )
        .expect("approve");
    fixture
        .store
        .expire_approval(
            &approved.id,
            approved.revision,
            actor(),
            timestamp("2026-08-10T00:10:00Z"),
        )
        .expect("expire");

    let error = ClaimedAction::verify(
        &fixture.store,
        fixture.action.clone(),
        forged_claim(&fixture.action),
    )
    .err()
    .expect("expired approval has no consumed claim");

    assert_eq!(error.code, ErrorCode::ApprovalInvalid);
    assert!(
        fixture
            .store
            .get_action_claim(&fixture.action.id)
            .expect("claim read")
            .is_none()
    );
}

struct PendingFixture {
    store: InMemoryExecutionStore,
    action: ActionRequest,
    approval: ApprovalRequest,
}

struct ApprovedFixture {
    store: InMemoryExecutionStore,
    action: ActionRequest,
    claim: ActionClaim,
}

fn approved_fixture() -> ApprovedFixture {
    let fixture = pending_fixture("01890f3e-a5f1-7cc2-98c0-5f9c6f5e7a01");
    let approved = fixture
        .store
        .decide_approval(
            &fixture.approval.id,
            fixture.approval.revision,
            ApprovalDecision::Approved,
            actor(),
            Some("reviewed"),
            timestamp("2026-08-10T00:00:04Z"),
        )
        .expect("approve");
    let consumption = fixture
        .store
        .consume_approval(
            &approved.id,
            approved.revision,
            &fixture.action,
            actor(),
            timestamp("2026-08-10T00:00:05Z"),
        )
        .expect("consume");

    ApprovedFixture {
        store: fixture.store,
        action: fixture.action,
        claim: consumption.action_claim,
    }
}

fn pending_fixture(run_id: &str) -> PendingFixture {
    let store = InMemoryExecutionStore::default();
    let run_id = RunId::parse(run_id).expect("UUIDv7");
    let created_at = timestamp("2026-08-10T00:00:00Z");
    let run = Run {
        schema_version: EXECUTION_SCHEMA_VERSION,
        id: run_id.clone(),
        runtime: "mock".to_owned(),
        workspace: PathBuf::from("/workspace"),
        status: RunStatus::Queued,
        revision: 0,
        actor_id: actor(),
        prompt_sha256: sha256_bytes(b"prompt"),
        created_at,
        updated_at: created_at,
        started_at: None,
        completed_at: None,
        terminal_reason: None,
    };
    store
        .create_run(run, AuditAppend::new(actor(), created_at))
        .expect("create run");
    store
        .transition_run(
            &run_id,
            0,
            RunStatus::Planning,
            None,
            AuditAppend::new(actor(), timestamp("2026-08-10T00:00:01Z")),
        )
        .expect("planning");
    store
        .transition_run(
            &run_id,
            1,
            RunStatus::AwaitingApproval,
            None,
            AuditAppend::new(actor(), timestamp("2026-08-10T00:00:02Z")),
        )
        .expect("awaiting approval");
    let action = ActionRequest::new(
        ActionId::generate(),
        run_id.clone(),
        "email.send",
        "sales-manager@example.invalid",
        json!({"amount": 10}),
    )
    .expect("action");
    let created_at = timestamp("2026-08-10T00:00:03Z");
    let approval = ApprovalRequest {
        schema_version: EXECUTION_SCHEMA_VERSION,
        id: ApprovalId::generate(),
        run_id,
        action_id: action.id.clone(),
        parameter_hash: action.parameter_hash().clone(),
        requested_by: actor(),
        request_reason: "external side effect".to_owned(),
        created_at,
        expires_at: timestamp("2026-08-10T00:10:00Z"),
        status: ApprovalStatus::Pending,
        revision: 0,
        decision: None,
        consumed_at: None,
    };
    let approval = store
        .create_approval(approval, actor(), created_at)
        .expect("create approval");

    PendingFixture {
        store,
        action,
        approval,
    }
}

fn forged_claim(action: &ActionRequest) -> ActionClaim {
    ActionClaim {
        run_id: action.run_id.clone(),
        action_id: action.id.clone(),
        parameter_hash: action.parameter_hash().clone(),
        actor: actor(),
        claimed_at: timestamp("2026-08-10T00:00:05Z"),
    }
}

fn actor() -> ActorId {
    ActorId::parse("user:test").expect("actor")
}

fn timestamp(value: &str) -> UtcTimestamp {
    UtcTimestamp::parse(value).expect("timestamp")
}
