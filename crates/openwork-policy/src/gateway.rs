//! Typed boundary for policy-checking OpenWork-managed actions.

use openwork_execution::{ActionPolicy, ActionRequest, PolicyEvaluation};

/// Explicit call-site marker for an action managed by `OpenWork`.
///
/// This marker is not runtime proof and does not claim interception of model
/// syscalls or provider-native tools. Controlled adapters create it only for
/// actions whose execution they own.
#[derive(Clone, Copy)]
pub struct ManagedAction<'a> {
    request: &'a ActionRequest,
}

impl<'a> ManagedAction<'a> {
    #[must_use]
    pub const fn new(request: &'a ActionRequest) -> Self {
        Self { request }
    }
}

/// Policy-only gateway. Execution remains in a separate controlled adapter.
pub struct ActionGateway<P> {
    policy: P,
}

impl<P: ActionPolicy> ActionGateway<P> {
    #[must_use]
    pub const fn new(policy: P) -> Self {
        Self { policy }
    }

    /// Returns a binding-preserving decision and never executes the action.
    #[must_use]
    pub fn decide(&self, action: ManagedAction<'_>) -> PolicyEvaluation {
        self.policy.evaluate(action.request)
    }
}
