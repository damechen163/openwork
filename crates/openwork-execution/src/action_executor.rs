//! Execution boundary for actions backed by an exact, consumed approval claim.

use crate::approval::{ActionClaim, ApprovalRepository};
use crate::audit::AuditAppend;
use crate::store::ExecutionStore;
use crate::{ActionId, ActionRequest, ActorId, RunId, Sha256Digest, UtcTimestamp};
use openwork_core::{ErrorCode, OpenWorkError};
use std::collections::BTreeMap;
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

/// Result of an idempotent executor call.
///
/// `Recovered` means the executor found the exact durable receipt written
/// before a prior successful side effect returned. Callers may reconcile a
/// missing audit record, but must not invoke the external side effect again.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ActionExecutionOutcome {
    Performed(ActionExecutionReceipt),
    Recovered(ActionExecutionReceipt),
}

impl ActionExecutionOutcome {
    fn into_receipt(self) -> (ActionExecutionReceipt, bool) {
        match self {
            Self::Performed(receipt) => (receipt, false),
            Self::Recovered(receipt) => (receipt, true),
        }
    }
}

/// Side-effect boundary. Implementations can receive only a verified claimed
/// action, never a bare model-authored request.
pub trait ActionExecutor: Send + Sync {
    /// Executes one previously verified action.
    ///
    /// # Errors
    ///
    /// Implementations must durably record the exact receipt before reporting
    /// a successful external side effect. A retry returns `Recovered` with that
    /// receipt and must not repeat the side effect.
    ///
    /// Returns an error when the implementation cannot safely perform or
    /// recover the exact action.
    fn execute(&self, action: ClaimedAction) -> Result<ActionExecutionOutcome, OpenWorkError>;
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
    let expected = ActionExecutionReceipt {
        run_id: claimed.action().run_id.clone(),
        action_id: claimed.action().id.clone(),
        parameter_hash: claimed.action().parameter_hash().clone(),
    };
    let (receipt, recovered) = executor.execute(claimed)?.into_receipt();
    if receipt != expected {
        return Err(invalid_receipt());
    }
    if !store.reconcile_action_execution(&receipt, AuditAppend::new(actor, timestamp))? {
        return Err(OpenWorkError::new(
            ErrorCode::ApprovalInvalid,
            if recovered {
                "executed action was already reconciled"
            } else {
                "executor performed an action with an existing audit receipt"
            },
        ));
    }
    Ok(receipt)
}

/// Deterministic no-side-effect executor for contract tests and local dry runs.
///
/// Replay protection is atomic for the lifetime of this instance. Production
/// executors must use a durable idempotency record before external side effects.
#[derive(Default)]
pub struct MockActionExecutor {
    receipts: Mutex<BTreeMap<ActionId, ActionExecutionReceipt>>,
}

impl MockActionExecutor {
    /// Returns whether this instance accepted the action.
    ///
    /// # Errors
    ///
    /// Returns an internal error if the executor state lock is poisoned.
    pub fn was_executed(&self, action_id: &ActionId) -> Result<bool, OpenWorkError> {
        Ok(self
            .receipts
            .lock()
            .map_err(|_| executor_state_error())?
            .contains_key(action_id))
    }

    /// Returns the number of side effects this mock accepted as newly performed.
    ///
    /// # Errors
    ///
    /// Returns an internal error if the executor state lock is poisoned.
    pub fn execution_count(&self) -> Result<usize, OpenWorkError> {
        Ok(self
            .receipts
            .lock()
            .map_err(|_| executor_state_error())?
            .len())
    }
}

impl ActionExecutor for MockActionExecutor {
    fn execute(&self, claimed: ClaimedAction) -> Result<ActionExecutionOutcome, OpenWorkError> {
        let action = claimed.action();
        let mut receipts = self.receipts.lock().map_err(|_| executor_state_error())?;
        if let Some(receipt) = receipts.get(&action.id) {
            if receipt.run_id != action.run_id || receipt.parameter_hash != *action.parameter_hash()
            {
                return Err(invalid_receipt());
            }
            return Ok(ActionExecutionOutcome::Recovered(receipt.clone()));
        }
        let receipt = ActionExecutionReceipt {
            run_id: action.run_id.clone(),
            action_id: action.id.clone(),
            parameter_hash: claimed.claim().parameter_hash.clone(),
        };
        receipts.insert(action.id.clone(), receipt.clone());
        Ok(ActionExecutionOutcome::Performed(receipt))
    }
}

fn invalid_claim() -> OpenWorkError {
    OpenWorkError::new(
        ErrorCode::ApprovalInvalid,
        "action does not match an exact consumed approval claim",
    )
}

fn invalid_receipt() -> OpenWorkError {
    OpenWorkError::new(
        ErrorCode::ApprovalInvalid,
        "action execution receipt is missing, duplicated, or does not match the exact claim",
    )
}

fn executor_state_error() -> OpenWorkError {
    OpenWorkError::new(ErrorCode::Internal, "action executor state lock poisoned")
}
