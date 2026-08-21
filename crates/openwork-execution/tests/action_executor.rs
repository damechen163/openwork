use openwork_core::{ErrorCode, OpenWorkError};
use openwork_execution::action_executor::{
    ActionExecutionOutcome, ActionExecutor, ClaimedAction, MockActionExecutor, execute_with_audit,
};
use openwork_execution::approval::{ActionClaim, ApprovalRepository};
use openwork_execution::audit::AuditAppend;
use openwork_execution::store::{ExecutionStore, InMemoryExecutionStore};
use openwork_execution::{
    ActionId, ActionRequest, ActorId, ApprovalDecision, ApprovalId, ApprovalRequest,
    ApprovalStatus, Artifact, AuditEvent, AuditEventType, EXECUTION_SCHEMA_VERSION, Run, RunId,
    RunStatus, UtcTimestamp, sha256_bytes,
};
use serde_json::json;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Barrier};
use std::thread;

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

    let ActionExecutionOutcome::Performed(receipt) =
        executor.execute(first).expect("first execution")
    else {
        panic!("first execution must perform the action");
    };

    assert_eq!(&receipt.run_id, &fixture.action.run_id);
    assert_eq!(&receipt.action_id, &fixture.action.id);
    assert_eq!(&receipt.parameter_hash, fixture.action.parameter_hash());
    assert!(executor.was_executed(&fixture.action.id).expect("state"));
    let ActionExecutionOutcome::Recovered(recovered) =
        executor.execute(replay).expect("replay recovers receipt")
    else {
        panic!("replay must not perform the action again");
    };
    assert_eq!(recovered, receipt);
    assert_eq!(executor.execution_count().expect("count"), 1);
}

#[test]
fn transient_audit_failure_is_reconciled_without_repeating_side_effect() {
    let fixture = approved_fixture();
    let store = FailOnceActionAuditStore::new(fixture.store);
    let executor = MockActionExecutor::default();
    let first = claimed(&store.inner, &fixture.action, &fixture.claim);

    let error = execute_with_audit(
        &store,
        &executor,
        first,
        actor(),
        timestamp("2026-08-10T00:00:06Z"),
    )
    .expect_err("first audit append fails after the side effect");
    assert_eq!(error.code, ErrorCode::Internal);
    assert_eq!(executor.execution_count().expect("count"), 1);
    assert_eq!(action_executed_count(&store, &fixture.action.run_id), 0);

    let receipt = execute_with_audit(
        &store,
        &executor,
        claimed(&store.inner, &fixture.action, &fixture.claim),
        actor(),
        timestamp("2026-08-10T00:00:07Z"),
    )
    .expect("retry recovers the durable receipt and repairs the audit");
    assert_eq!(&receipt.action_id, &fixture.action.id);
    assert_eq!(executor.execution_count().expect("count"), 1);
    assert_eq!(action_executed_count(&store, &fixture.action.run_id), 1);

    let replay = execute_with_audit(
        &store,
        &executor,
        claimed(&store.inner, &fixture.action, &fixture.claim),
        actor(),
        timestamp("2026-08-10T00:00:08Z"),
    )
    .expect_err("a reconciled action remains replay protected");
    assert_eq!(replay.code, ErrorCode::ApprovalInvalid);
    assert_eq!(executor.execution_count().expect("count"), 1);
    assert_eq!(action_executed_count(&store, &fixture.action.run_id), 1);
}

#[test]
fn concurrent_reconciliation_appends_exactly_one_action_audit() {
    let fixture = approved_fixture();
    let store = Arc::new(FailOnceActionAuditStore::new(fixture.store));
    let executor = Arc::new(MockActionExecutor::default());
    let first = claimed(&store.inner, &fixture.action, &fixture.claim);
    execute_with_audit(
        store.as_ref(),
        executor.as_ref(),
        first,
        actor(),
        timestamp("2026-08-10T00:00:06Z"),
    )
    .expect_err("establish performed receipt with missing audit");

    let barrier = Arc::new(Barrier::new(2));
    let mut workers = Vec::new();
    for retry in [
        claimed(&store.inner, &fixture.action, &fixture.claim),
        claimed(&store.inner, &fixture.action, &fixture.claim),
    ] {
        let worker_store = Arc::clone(&store);
        let worker_executor = Arc::clone(&executor);
        let worker_barrier = Arc::clone(&barrier);
        workers.push(thread::spawn(move || {
            worker_barrier.wait();
            execute_with_audit(
                worker_store.as_ref(),
                worker_executor.as_ref(),
                retry,
                actor(),
                timestamp("2026-08-10T00:00:07Z"),
            )
        }));
    }
    let results = workers
        .into_iter()
        .map(|worker| worker.join().expect("reconciliation worker"))
        .collect::<Vec<_>>();

    assert_eq!(results.iter().filter(|result| result.is_ok()).count(), 1);
    assert_eq!(
        results
            .iter()
            .filter_map(|result| result.as_ref().err())
            .filter(|error| error.code == ErrorCode::ApprovalInvalid)
            .count(),
        1
    );
    assert_eq!(executor.execution_count().expect("count"), 1);
    assert_eq!(
        action_executed_count(store.as_ref(), &fixture.action.run_id),
        1
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

struct FailOnceActionAuditStore {
    inner: InMemoryExecutionStore,
    fail_next_action_audit: AtomicBool,
}

impl FailOnceActionAuditStore {
    fn new(inner: InMemoryExecutionStore) -> Self {
        Self {
            inner,
            fail_next_action_audit: AtomicBool::new(true),
        }
    }
}

impl ExecutionStore for FailOnceActionAuditStore {
    fn create_run(&self, run: Run, audit: AuditAppend) -> Result<Run, OpenWorkError> {
        self.inner.create_run(run, audit)
    }

    fn transition_run(
        &self,
        run_id: &RunId,
        expected_revision: u64,
        next: RunStatus,
        reason: Option<&str>,
        audit: AuditAppend,
    ) -> Result<Run, OpenWorkError> {
        self.inner
            .transition_run(run_id, expected_revision, next, reason, audit)
    }

    fn append_audit(
        &self,
        run_id: &RunId,
        event_type: AuditEventType,
        audit: AuditAppend,
    ) -> Result<AuditEvent, OpenWorkError> {
        self.inner.append_audit(run_id, event_type, audit)
    }

    fn reconcile_action_execution(
        &self,
        receipt: &openwork_execution::action_executor::ActionExecutionReceipt,
        audit: AuditAppend,
    ) -> Result<bool, OpenWorkError> {
        if self.fail_next_action_audit.swap(false, Ordering::AcqRel) {
            return Err(OpenWorkError::new(
                ErrorCode::Internal,
                "simulated transient action audit failure",
            ));
        }
        self.inner.reconcile_action_execution(receipt, audit)
    }

    fn record_artifacts(
        &self,
        run_id: &RunId,
        artifacts: Vec<Artifact>,
        audit: AuditAppend,
    ) -> Result<(), OpenWorkError> {
        self.inner.record_artifacts(run_id, artifacts, audit)
    }

    fn get_run(&self, run_id: &RunId) -> Result<Option<Run>, OpenWorkError> {
        self.inner.get_run(run_id)
    }

    fn audit_events(&self, run_id: &RunId) -> Result<Vec<AuditEvent>, OpenWorkError> {
        self.inner.audit_events(run_id)
    }

    fn artifacts(&self, run_id: &RunId) -> Result<Vec<Artifact>, OpenWorkError> {
        self.inner.artifacts(run_id)
    }
}

fn claimed(
    store: &InMemoryExecutionStore,
    action: &ActionRequest,
    claim: &ActionClaim,
) -> ClaimedAction {
    ClaimedAction::verify(store, action.clone(), claim.clone()).expect("exact durable claim")
}

fn action_executed_count(store: &impl ExecutionStore, run_id: &RunId) -> usize {
    store
        .audit_events(run_id)
        .expect("audit events")
        .iter()
        .filter(|event| event.event_type == AuditEventType::ActionExecuted)
        .count()
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
