//! Deterministic control-plane proof for the sales demo.
//!
//! The external action uses `MockActionExecutor`, so this test proves policy,
//! approval, claim, replay, and audit semantics without claiming that an email
//! was delivered. Real Docker analysis is covered by `real_docker_sales.rs`.

use openwork_core::ErrorCode;
use openwork_e2e::scenario::ScenarioFixture;
use openwork_execution::action_executor::{ClaimedAction, MockActionExecutor, execute_with_audit};
use openwork_execution::approval::ApprovalRepository;
use openwork_execution::audit::AuditAppend;
use openwork_execution::store::{ExecutionStore, InMemoryExecutionStore};
use openwork_execution::{
    ActionId, ActionRequest, ActorId, ApprovalDecision, ApprovalId, ApprovalRequest,
    ApprovalStatus, AuditEvent, AuditEventType, EXECUTION_SCHEMA_VERSION, PolicyDecision, Run,
    RunId, RunStatus, UtcTimestamp, sha256_bytes,
};
use openwork_policy::engine::PolicyEngine;
use serde_json::json;
use std::path::PathBuf;

const POLICY: &str = include_str!("../../../samples/sales/policy.yaml");
const RISKY_EMAIL: &str = include_str!("../../../samples/sales/scenarios/risky-email-send.json");
const DESTRUCTIVE_DATABASE: &str =
    include_str!("../../../samples/sales/scenarios/destructive-database-delete.json");
const GOLDEN_ANALYSIS: &str = include_str!("../../../samples/sales/golden/sales-analysis.csv");

#[test]
fn sales_control_plane_requires_an_exact_single_use_external_action_claim() {
    let fixture = approved_email_fixture();
    let changed_recipient = reject_changed_recipient(&fixture);
    consume_execute_once_and_reject_replay(&fixture, changed_recipient);
    complete_and_verify_sales_run(&fixture);
}

struct ApprovedEmailFixture {
    policy: PolicyEngine,
    store: InMemoryExecutionStore,
    actor: ActorId,
    run_id: RunId,
    email: ActionRequest,
    approved: ApprovalRequest,
}

fn approved_email_fixture() -> ApprovedEmailFixture {
    let policy = PolicyEngine::from_yaml(POLICY).expect("sales policy");
    let risky = ScenarioFixture::from_json(RISKY_EMAIL).expect("risky email fixture");
    let store = InMemoryExecutionStore::default();
    let actor = actor();
    let run = create_run(&store, risky.action_request().run_id.clone());
    let run = transition(
        &store,
        &run,
        RunStatus::Planning,
        "2026-08-10T00:00:01Z",
        None,
    );
    allow_safe_filesystem_actions(&policy, &store, &run, &actor);
    let email = risky.action_request().clone();
    let email_evaluation = policy.evaluate_at(&email, timestamp("2026-08-10T00:00:03Z"));
    assert_eq!(email_evaluation.decision, PolicyDecision::RequireApproval);
    assert_eq!(&email_evaluation.parameter_hash, email.parameter_hash());
    let awaiting = transition(
        &store,
        &run,
        RunStatus::AwaitingApproval,
        "2026-08-10T00:00:04Z",
        None,
    );
    let approval = store
        .create_approval(
            pending_approval(&email, "2026-08-10T00:00:05Z"),
            actor.clone(),
            timestamp("2026-08-10T00:00:05Z"),
        )
        .expect("create exact email approval");
    assert_eq!(approval.status, ApprovalStatus::Pending);
    assert_eq!(approval.revision, 0);
    assert_eq!(awaiting.status, RunStatus::AwaitingApproval);
    let approved = store
        .decide_approval(
            &approval.id,
            approval.revision,
            ApprovalDecision::Approved,
            actor.clone(),
            Some("sales report recipient and attachment reviewed"),
            timestamp("2026-08-10T00:00:06Z"),
        )
        .expect("approve exact pending revision");
    assert_eq!(approved.status, ApprovalStatus::Approved);
    assert_eq!(approved.revision, 1);
    ApprovedEmailFixture {
        policy,
        store,
        actor,
        run_id: run.id,
        email,
        approved,
    }
}

fn allow_safe_filesystem_actions(
    policy: &PolicyEngine,
    store: &InMemoryExecutionStore,
    run: &Run,
    actor: &ActorId,
) {
    for safe_action in safe_filesystem_actions(&run.id) {
        let evaluation = policy.evaluate_at(&safe_action, timestamp("2026-08-10T00:00:02Z"));
        assert_eq!(evaluation.decision, PolicyDecision::Allow);
        assert_eq!(&evaluation.parameter_hash, safe_action.parameter_hash());
        store
            .append_audit(
                &run.id,
                AuditEventType::PolicyAllowed,
                AuditAppend::new(actor.clone(), evaluation.evaluated_at),
            )
            .expect("audit safe policy decision");
        assert!(
            store
                .get_action_claim(&safe_action.id)
                .expect("safe action claim lookup")
                .is_none(),
            "safe policy decisions do not manufacture approval claims"
        );
    }
}

fn reject_changed_recipient(fixture: &ApprovedEmailFixture) -> ActionRequest {
    let changed_recipient = ActionRequest::new(
        fixture.email.id.clone(),
        fixture.email.run_id.clone(),
        fixture.email.action.clone(),
        "mailto:ceo@example.invalid",
        json!({
            "attachment": "sales-analysis.csv",
            "recipient": "ceo@example.invalid",
            "subject": "August sales analysis"
        }),
    )
    .expect("changed email is a valid but differently bound action");
    assert_eq!(
        fixture
            .policy
            .evaluate_at(&changed_recipient, timestamp("2026-08-10T00:00:07Z"))
            .decision,
        PolicyDecision::Deny
    );
    let mismatch = fixture
        .store
        .consume_approval(
            &fixture.approved.id,
            fixture.approved.revision,
            &changed_recipient,
            fixture.actor.clone(),
            timestamp("2026-08-10T00:00:07Z"),
        )
        .expect_err("old approval must reject a changed recipient");
    assert_eq!(mismatch.code, ErrorCode::ApprovalInvalid);
    changed_recipient
}

fn consume_execute_once_and_reject_replay(
    fixture: &ApprovedEmailFixture,
    changed_recipient: ActionRequest,
) {
    let consumption = fixture
        .store
        .consume_approval(
            &fixture.approved.id,
            fixture.approved.revision,
            &fixture.email,
            fixture.actor.clone(),
            timestamp("2026-08-10T00:00:08Z"),
        )
        .expect("atomically consume exact approval and claim action");
    assert_eq!(consumption.approval.status, ApprovalStatus::Consumed);
    assert_eq!(consumption.approval.revision, 2);
    assert_eq!(consumption.action_claim.action_id, fixture.email.id);
    assert_eq!(
        consumption.action_claim.parameter_hash,
        fixture.email.parameter_hash().clone()
    );
    let changed_claim_error = ClaimedAction::verify(
        &fixture.store,
        changed_recipient,
        consumption.action_claim.clone(),
    )
    .err()
    .expect("old claim must reject a changed recipient");
    assert_eq!(changed_claim_error.code, ErrorCode::ApprovalInvalid);
    let first = ClaimedAction::verify(
        &fixture.store,
        fixture.email.clone(),
        consumption.action_claim.clone(),
    )
    .expect("exact durable claim");
    let replay = ClaimedAction::verify(
        &fixture.store,
        fixture.email.clone(),
        consumption.action_claim.clone(),
    )
    .expect("claim remains durably verifiable before executor replay guard");
    let executor = MockActionExecutor::default();
    let receipt = execute_with_audit(
        &fixture.store,
        &executor,
        first,
        fixture.actor.clone(),
        timestamp("2026-08-10T00:00:09Z"),
    )
    .expect("execute and audit mock action once");
    assert_eq!(receipt.action_id, fixture.email.id);
    assert_eq!(
        receipt.parameter_hash,
        fixture.email.parameter_hash().clone()
    );
    assert!(
        executor
            .was_executed(&fixture.email.id)
            .expect("mock state")
    );
    assert_eq!(
        execute_with_audit(
            &fixture.store,
            &executor,
            replay,
            fixture.actor.clone(),
            timestamp("2026-08-10T00:00:09Z"),
        )
        .expect_err("executor replay denied")
        .code,
        ErrorCode::ApprovalInvalid
    );
    assert_eq!(
        fixture
            .store
            .consume_approval(
                &fixture.approved.id,
                consumption.approval.revision,
                &fixture.email,
                fixture.actor.clone(),
                timestamp("2026-08-10T00:00:09Z"),
            )
            .expect_err("approval replay denied")
            .code,
        ErrorCode::ApprovalInvalid
    );
}

fn complete_and_verify_sales_run(fixture: &ApprovedEmailFixture) {
    let running = fixture
        .store
        .get_run(&fixture.run_id)
        .expect("read running run")
        .expect("run exists");
    assert_eq!(running.status, RunStatus::Running);
    let succeeded = transition(
        &fixture.store,
        &running,
        RunStatus::Succeeded,
        "2026-08-10T00:00:11Z",
        None,
    );
    assert_eq!(succeeded.revision, 4);
    assert!(succeeded.started_at.is_some());
    assert!(succeeded.completed_at.is_some());
    assert!(succeeded.terminal_reason.is_none());
    let events = fixture
        .store
        .audit_events(&fixture.run_id)
        .expect("main audit chain");
    assert_event_types(
        &events,
        &[
            AuditEventType::PolicyAllowed,
            AuditEventType::ApprovalRequested,
            AuditEventType::ApprovalApproved,
            AuditEventType::ApprovalBindingMismatch,
            AuditEventType::ApprovalConsumed,
            AuditEventType::RuntimeStarted,
            AuditEventType::ActionExecuted,
            AuditEventType::RunCompleted,
        ],
    );
    let action_executed = events
        .iter()
        .find(|event| event.event_type == AuditEventType::ActionExecuted)
        .expect("typed action execution audit");
    assert_eq!(
        action_executed.metadata.as_map().get("action_id"),
        Some(&json!(&fixture.email.id))
    );
    assert_eq!(
        action_executed.metadata.as_map().get("parameter_hash"),
        Some(&json!(fixture.email.parameter_hash()))
    );
    assert_hash_chain(&events);
}

#[test]
fn destructive_sales_action_is_denied_without_an_approval_window() {
    let policy = PolicyEngine::from_yaml(POLICY).expect("sales policy");
    let destructive =
        ScenarioFixture::from_json(DESTRUCTIVE_DATABASE).expect("destructive fixture");
    let store = InMemoryExecutionStore::default();
    let run_id = RunId::parse("01890f3e-a5f1-7cc2-98c0-5f9c6f5e7d01").expect("UUIDv7");
    let run = create_run(&store, run_id);
    let run = transition(
        &store,
        &run,
        RunStatus::Planning,
        "2026-08-10T01:00:01Z",
        None,
    );
    let template = destructive.action_request();
    let action = ActionRequest::new(
        template.id.clone(),
        run.id.clone(),
        template.action.clone(),
        template.resource.clone(),
        template.parameters.clone(),
    )
    .expect("destructive action rebound to denial run");
    let evaluation = policy.evaluate_at(&action, timestamp("2026-08-10T01:00:02Z"));
    assert_eq!(evaluation.decision, PolicyDecision::Deny);
    assert_eq!(
        evaluation.effective_risk,
        openwork_execution::RiskLevel::DestructiveOrFinancial
    );
    store
        .append_audit(
            &run.id,
            AuditEventType::PolicyDenied,
            AuditAppend::new(actor(), evaluation.evaluated_at),
        )
        .expect("audit direct policy denial");
    let failed = transition(
        &store,
        &run,
        RunStatus::Failed,
        "2026-08-10T01:00:03Z",
        Some("policy denied database.delete"),
    );
    assert_eq!(failed.status, RunStatus::Failed);
    assert_eq!(failed.revision, 2);
    assert!(failed.started_at.is_none());
    assert!(failed.completed_at.is_some());
    assert_eq!(
        failed.terminal_reason.as_deref(),
        Some("policy denied database.delete")
    );
    assert!(
        store
            .get_action_claim(&action.id)
            .expect("destructive claim lookup")
            .is_none()
    );
    let events = store.audit_events(&run.id).expect("denial audit chain");
    assert_event_types(
        &events,
        &[AuditEventType::PolicyDenied, AuditEventType::RunFailed],
    );
    assert!(!events.iter().any(|event| matches!(
        event.event_type,
        AuditEventType::ApprovalRequested
            | AuditEventType::ApprovalApproved
            | AuditEventType::ApprovalConsumed
    )));
    assert_hash_chain(&events);
}

fn safe_filesystem_actions(run_id: &RunId) -> [ActionRequest; 2] {
    [
        ActionRequest::new(
            ActionId::parse("01890f3e-a5f1-7cc2-98c0-5f9c6f5e7a10").expect("read action ID"),
            run_id.clone(),
            "filesystem.read",
            "/workspace/input/sales_july.csv",
            json!({"mode": "read_only"}),
        )
        .expect("safe read action"),
        ActionRequest::new(
            ActionId::parse("01890f3e-a5f1-7cc2-98c0-5f9c6f5e7a11").expect("write action ID"),
            run_id.clone(),
            "filesystem.write",
            "/workspace/output/sales-analysis.csv",
            json!({
                "content_sha256": sha256_bytes(GOLDEN_ANALYSIS.as_bytes()),
                "mode": "create"
            }),
        )
        .expect("safe write action"),
    ]
}

fn pending_approval(action: &ActionRequest, created_at: &str) -> ApprovalRequest {
    ApprovalRequest {
        schema_version: EXECUTION_SCHEMA_VERSION,
        id: ApprovalId::parse("01890f3e-a5f1-7cc2-98c0-5f9c6f5e7a20").expect("approval ID"),
        run_id: action.run_id.clone(),
        action_id: action.id.clone(),
        parameter_hash: action.parameter_hash().clone(),
        requested_by: actor(),
        request_reason: "email.send requires explicit external-effect approval".to_owned(),
        created_at: timestamp(created_at),
        expires_at: timestamp("2026-08-10T00:10:00Z"),
        status: ApprovalStatus::Pending,
        revision: 0,
        decision: None,
        consumed_at: None,
    }
}

fn create_run(store: &InMemoryExecutionStore, run_id: RunId) -> Run {
    let created_at = timestamp(if run_id.to_hyphenated().ends_with("7d01") {
        "2026-08-10T01:00:00Z"
    } else {
        "2026-08-10T00:00:00Z"
    });
    let run = Run {
        schema_version: EXECUTION_SCHEMA_VERSION,
        id: run_id,
        runtime: "sales-demo-control-plane".to_owned(),
        workspace: PathBuf::from("/workspace"),
        status: RunStatus::Queued,
        revision: 0,
        actor_id: actor(),
        prompt_sha256: sha256_bytes(b"analyze sales and prepare the approved report action"),
        created_at,
        updated_at: created_at,
        started_at: None,
        completed_at: None,
        terminal_reason: None,
    };
    store
        .create_run(run, AuditAppend::new(actor(), created_at))
        .expect("create queued run")
}

fn transition(
    store: &InMemoryExecutionStore,
    run: &Run,
    next: RunStatus,
    at: &str,
    reason: Option<&str>,
) -> Run {
    store
        .transition_run(
            &run.id,
            run.revision,
            next,
            reason,
            AuditAppend::new(actor(), timestamp(at)),
        )
        .expect("valid run transition")
}

fn assert_event_types(events: &[AuditEvent], required: &[AuditEventType]) {
    for event_type in required {
        assert!(
            events.iter().any(|event| &event.event_type == event_type),
            "missing audit event {event_type:?}"
        );
    }
}

fn assert_hash_chain(events: &[AuditEvent]) {
    let mut previous = None;
    for (index, event) in events.iter().enumerate() {
        let sequence = u64::try_from(index + 1).expect("bounded test audit sequence");
        event
            .verify_integrity(sequence, previous)
            .expect("audit hash chain integrity");
        previous = Some(event.event_hash());
    }
}

fn actor() -> ActorId {
    ActorId::parse("user:sales-approver").expect("actor")
}

fn timestamp(value: &str) -> UtcTimestamp {
    UtcTimestamp::parse(value).expect("UTC timestamp")
}
