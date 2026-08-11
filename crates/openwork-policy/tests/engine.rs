use openwork_execution::{ActionId, ActionRequest, PolicyDecision, RiskLevel, RunId, UtcTimestamp};
use openwork_policy::engine::PolicyEngine;
use openwork_policy::gateway::{ActionGateway, ManagedAction};
use serde_json::json;

const POLICY: &str = r"
version: 1
defaults:
  unknown: deny
actions:
  filesystem.read:
    risk: L0
    decision: allow
  filesystem.write:
    risk: L1
    decision: allow
    resources:
      exact:
        /workspace/protected.txt: deny
      default: allow
  email.send:
    risk: L3
    decision: approval
    resources:
      exact:
        finance@company.com: approval
      default: deny
  database.delete:
    risk: L4
    decision: deny
";

#[test]
fn safe_risky_destructive_and_unknown_actions_follow_policy_risk() {
    let engine = PolicyEngine::from_yaml(POLICY).expect("policy");
    for (action, resource, expected_risk, expected_decision) in [
        (
            "filesystem.read",
            "/workspace/in.txt",
            RiskLevel::Read,
            PolicyDecision::Allow,
        ),
        (
            "filesystem.write",
            "/workspace/out.txt",
            RiskLevel::LocalWrite,
            PolicyDecision::Allow,
        ),
        (
            "email.send",
            "finance@company.com",
            RiskLevel::ExternalEffect,
            PolicyDecision::RequireApproval,
        ),
        (
            "database.delete",
            "all",
            RiskLevel::DestructiveOrFinancial,
            PolicyDecision::Deny,
        ),
        (
            "unknown.action",
            "anything",
            RiskLevel::DestructiveOrFinancial,
            PolicyDecision::Deny,
        ),
    ] {
        let result = engine.evaluate_at(&action_request(action, resource, json!({})), now());
        assert_eq!(result.effective_risk, expected_risk);
        assert_eq!(result.decision, expected_decision);
    }
}

#[test]
fn resource_mismatch_uses_explicit_default_and_request_hint_cannot_lower_risk() {
    let engine = PolicyEngine::from_yaml(POLICY).expect("policy");
    let protected = engine.evaluate_at(
        &action_request("filesystem.write", "/workspace/protected.txt", json!({})),
        now(),
    );
    assert_eq!(protected.decision, PolicyDecision::Deny);
    let external = engine.evaluate_at(
        &action_request("email.send", "outside@example.net", json!({"risk": "L0"})),
        now(),
    );
    assert_eq!(external.effective_risk, RiskLevel::ExternalEffect);
    assert_eq!(external.decision, PolicyDecision::Deny);
}

#[test]
fn approval_preserves_exact_binding_and_tampering_fails_closed() {
    let engine = PolicyEngine::from_yaml(POLICY).expect("policy");
    let original = action_request(
        "email.send",
        "finance@company.com",
        json!({"recipient": "finance@company.com"}),
    );
    let changed = action_request(
        "email.send",
        "finance@company.com",
        json!({"recipient": "outside@example.net"}),
    );
    assert_ne!(original.parameter_hash(), changed.parameter_hash());
    let approved = engine.evaluate_at(&original, now());
    assert_eq!(approved.decision, PolicyDecision::RequireApproval);
    assert_eq!(&approved.parameter_hash, original.parameter_hash());

    let mut tampered = original;
    tampered.parameters = json!({"recipient": "outside@example.net"});
    let denied = engine.evaluate_at(&tampered, now());
    assert_eq!(denied.decision, PolicyDecision::Deny);
    assert_eq!(denied.reason_code, "action_binding_mismatch");
}

#[test]
fn decisions_are_deterministic_and_gateway_only_evaluates_managed_actions() {
    let request = action_request("filesystem.read", "/workspace/in.txt", json!({}));
    let first = PolicyEngine::from_yaml(POLICY)
        .expect("policy")
        .evaluate_at(&request, now());
    let second = PolicyEngine::from_yaml(POLICY)
        .expect("policy")
        .evaluate_at(&request, now());
    assert_eq!(first, second);

    let gateway = ActionGateway::new(PolicyEngine::from_yaml(POLICY).expect("policy"));
    let result = gateway.decide(ManagedAction::new(&request));
    assert_eq!(result.decision, PolicyDecision::Allow);
}

fn action_request(action: &str, resource: &str, parameters: serde_json::Value) -> ActionRequest {
    ActionRequest::new(
        ActionId::parse("01890f3e-a5f1-7cc2-98c0-5f9c6f5e7a02").expect("action ID"),
        RunId::parse("01890f3e-a5f1-7cc2-98c0-5f9c6f5e7a01").expect("run ID"),
        action,
        resource,
        parameters,
    )
    .expect("action")
}

fn now() -> UtcTimestamp {
    UtcTimestamp::parse("2026-08-10T00:00:00Z").expect("time")
}
