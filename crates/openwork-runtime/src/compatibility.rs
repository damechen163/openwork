//! Shared behavioral checks for isolated runtime test doubles.

use crate::{
    AgentRuntime, AuthStatus, CancellationToken, DetectionState, RuntimeEventKind, RuntimeResult,
    RuntimeRunRequest,
};
use openwork_core::{ErrorCode, OpenWorkError};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CompatibilityReport {
    pub checks_passed: usize,
}

/// Exercises the complete runtime lifecycle on an isolated test double.
///
/// This function installs, updates, runs, cancels, and uninstalls. Never pass a
/// real external-managed runtime; provider adapters use non-mutating fixture suites.
///
/// # Errors
///
/// Returns the first runtime error or contract invariant violation.
pub fn exercise_isolated_runtime(runtime: &dyn AgentRuntime) -> RuntimeResult<CompatibilityReport> {
    let mut checks = 0;
    ensure(!runtime.metadata().id.0.is_empty(), "runtime id is empty")?;
    checks += 1;
    ensure(
        runtime.detect()?.state == DetectionState::Missing,
        "isolated runtime must begin missing",
    )?;
    checks += 1;

    let capabilities = runtime.capabilities();
    ensure(
        capabilities.install && capabilities.run && capabilities.cancel,
        "required test capabilities are absent",
    )?;
    checks += 1;
    let plan = runtime.install_plan(Some("1.2.3"))?;
    ensure(
        plan.source_url.starts_with("https://"),
        "install source is not HTTPS",
    )?;
    runtime.install(&plan)?;
    ensure(
        runtime.detect()?.state == DetectionState::Healthy,
        "install did not become healthy",
    )?;
    ensure(
        runtime.version()?.as_deref() == Some("1.2.3"),
        "installed version did not round-trip",
    )?;
    checks += 3;

    runtime.doctor()?;
    ensure(
        runtime.auth_status()? == AuthStatus::Unauthenticated,
        "unexpected initial auth state",
    )?;
    checks += 2;
    let request = RuntimeRunRequest {
        prompt: "TOKEN=synthetic-secret say hello".to_owned(),
        working_directory: None,
    };
    let events = runtime.run(&request, &CancellationToken::new())?;
    ensure(
        events
            .last()
            .is_some_and(|event| event.kind == RuntimeEventKind::Completed),
        "run did not complete",
    )?;
    ensure(
        events
            .iter()
            .all(|event| !event.message.contains("synthetic-secret")),
        "run leaked a synthetic secret",
    )?;
    checks += 2;

    let cancellation = CancellationToken::new();
    runtime.cancel(&cancellation)?;
    let events = runtime.run(&request, &cancellation)?;
    ensure(
        events
            .iter()
            .any(|event| event.kind == RuntimeEventKind::Cancelled),
        "cancelled run lacked event",
    )?;
    checks += 1;
    runtime.update(Some("1.2.4"))?;
    ensure(
        runtime.version()?.as_deref() == Some("1.2.4"),
        "update version did not round-trip",
    )?;
    checks += 1;
    runtime.uninstall()?;
    ensure(
        runtime.detect()?.state == DetectionState::Missing,
        "uninstall did not return to missing",
    )?;
    checks += 1;
    Ok(CompatibilityReport {
        checks_passed: checks,
    })
}

fn ensure(condition: bool, message: &str) -> RuntimeResult<()> {
    if condition {
        Ok(())
    } else {
        Err(OpenWorkError::new(
            ErrorCode::Internal,
            format!("runtime compatibility failed: {message}"),
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{MockRuntime, RuntimeId, RuntimeRegistry};
    use std::sync::Arc;

    #[test]
    fn mock_runtime_passes_full_compatibility_suite() {
        let report = exercise_isolated_runtime(&MockRuntime::default()).unwrap();
        assert_eq!(report.checks_passed, 13);
    }

    #[test]
    fn registry_is_sorted_and_rejects_duplicates() {
        let mut registry = RuntimeRegistry::new();
        registry
            .register(Arc::new(MockRuntime::new("zeta")))
            .unwrap();
        registry
            .register(Arc::new(MockRuntime::new("alpha")))
            .unwrap();
        let ids: Vec<RuntimeId> = registry
            .metadata()
            .into_iter()
            .map(|item| item.id)
            .collect();
        assert_eq!(ids, vec![RuntimeId::from("alpha"), RuntimeId::from("zeta")]);
        assert!(
            registry
                .register(Arc::new(MockRuntime::new("alpha")))
                .is_err()
        );
    }
}
