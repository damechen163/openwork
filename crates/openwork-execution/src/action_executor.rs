//! Execution boundary for actions backed by an exact, consumed approval claim.

use crate::approval::{ActionClaim, ApprovalRepository};
use crate::audit::AuditAppend;
use crate::store::ExecutionStore;
use crate::{ActionId, ActionRequest, ActorId, AuditEventType, RunId, Sha256Digest, UtcTimestamp};
use openwork_core::{ErrorCode, OpenWorkError};
use std::collections::BTreeSet;
use std::sync::Mutex;

/// An action whose exact run, action identity, and parameter hash match a
/// durably persisted claim created by [`ApprovalRepository::consume_approval`].
///
/// Fields are private so an executor cannot accidentally accept a model-authored
/// [`ActionRequest`] without first checking the durable consumed claim.
pub struct ClaimedAction {
    action: ActionRequest,
    claim: ActionClaim,
}

impl ClaimedAction {
    /// Verifies an action against the durable claim produced by approval
    /// consumption.
    ///
    /// # Errors
    ///
    /// Returns [`ErrorCode::ApprovalInvalid`] when the action is tampered, the
    /// supplied claim is not exact, or no consumed claim exists. Storage errors
    /// are returned unchanged.
    pub fn verify<R>(
        repository: &R,
        action: ActionRequest,
        claim: ActionClaim,
    ) -> Result<Self, OpenWorkError>
    where
        R: ApprovalRepository + ?Sized,
    {
        if !action.parameters_match_hash()
            || action.run_id != claim.run_id
            || action.id != claim.action_id
            || action.parameter_hash() != &claim.parameter_hash
        {
            return Err(invalid_claim());
        }

        let persisted = repository.get_action_claim(&action.id)?;
        if persisted.as_ref() != Some(&claim) {
            return Err(invalid_claim());
        }

        Ok(Self { action, claim })
    }

    #[must_use]
    pub const fn action(&self) -> &ActionRequest {
        &self.action
    }

    #[must_use]
    pub const fn claim(&self) -> &ActionClaim {
        &self.claim
    }
}

/// Non-secret proof that an authorized action reached an executor once.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ActionExecutionReceipt {
    pub run_id: RunId,
    pub action_id: ActionId,
    pub parameter_hash: Sha256Digest,
}

/// Side-effect boundary. Implementations can receive only a verified claimed
/// action, never a bare model-authored request.
pub trait ActionExecutor: Send + Sync {
    /// Executes one previously verified action.
    ///
    /// # Errors
    ///
    /// Returns an error when the exact claim has already executed or when the
    /// implementation cannot safely perform the action.
    fn execute(&self, action: ClaimedAction) -> Result<ActionExecutionReceipt, OpenWorkError>;
}

/// Executes an exact claimed action and appends a typed, content-free audit
/// receipt to the same run.
///
/// Production executors must durably deduplicate the action before the side
/// effect so a transient audit failure can be retried safely.
///
/// # Errors
///
/// Returns an executor or audit persistence error. A bare [`ActionRequest`]
/// cannot enter this boundary.
pub fn execute_with_audit<S, E>(
    store: &S,
    executor: &E,
    claimed: ClaimedAction,
    actor: ActorId,
    timestamp: UtcTimestamp,
) -> Result<ActionExecutionReceipt, OpenWorkError>
where
    S: ExecutionStore + ?Sized,
    E: ActionExecutor + ?Sized,
{
    let run_id = claimed.action().run_id.clone();
    let receipt = executor.execute(claimed)?;
    store.append_audit(
        &run_id,
        AuditEventType::ActionExecuted,
        AuditAppend::new(actor, timestamp)
            .with_action_execution(receipt.action_id.clone(), receipt.parameter_hash.clone()),
    )?;
    Ok(receipt)
}

/// Deterministic no-side-effect executor for contract tests and local dry runs.
///
/// Replay protection is atomic for the lifetime of this instance. Production
/// executors must use a durable idempotency record before external side effects.
#[derive(Default)]
pub struct MockActionExecutor {
    executed: Mutex<BTreeSet<ActionId>>,
}

impl MockActionExecutor {
    /// Returns whether this instance accepted the action.
    ///
    /// # Errors
    ///
    /// Returns an internal error if the executor state lock is poisoned.
    pub fn was_executed(&self, action_id: &ActionId) -> Result<bool, OpenWorkError> {
        Ok(self
            .executed
            .lock()
            .map_err(|_| executor_state_error())?
            .contains(action_id))
    }
}

impl ActionExecutor for MockActionExecutor {
    fn execute(&self, claimed: ClaimedAction) -> Result<ActionExecutionReceipt, OpenWorkError> {
        let action = claimed.action();
        let mut executed = self.executed.lock().map_err(|_| executor_state_error())?;
        if !executed.insert(action.id.clone()) {
            return Err(OpenWorkError::new(
                ErrorCode::ApprovalInvalid,
                "consumed action claim was already executed",
            ));
        }

        Ok(ActionExecutionReceipt {
            run_id: action.run_id.clone(),
            action_id: action.id.clone(),
            parameter_hash: claimed.claim().parameter_hash.clone(),
        })
    }
}

fn invalid_claim() -> OpenWorkError {
    OpenWorkError::new(
        ErrorCode::ApprovalInvalid,
        "action does not match an exact consumed approval claim",
    )
}

fn executor_state_error() -> OpenWorkError {
    OpenWorkError::new(ErrorCode::Internal, "action executor state lock poisoned")
}
