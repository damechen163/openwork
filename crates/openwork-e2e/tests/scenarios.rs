use openwork_e2e::scenario::ScenarioFixture;
use openwork_execution::{PolicyDecision, RiskLevel};

const RISKY: &str = include_str!("../../../samples/sales/scenarios/risky-email-send.json");
const DESTRUCTIVE: &str =
    include_str!("../../../samples/sales/scenarios/destructive-database-delete.json");

#[test]
fn risky_and_destructive_scenarios_pin_exact_action_bindings() {
    let risky = ScenarioFixture::from_json(RISKY).expect("risky scenario");
    assert_eq!(risky.name(), "risky-email-send");
    assert_eq!(risky.expected_risk(), RiskLevel::ExternalEffect);
    assert_eq!(risky.expected_decision(), PolicyDecision::RequireApproval);
    assert_eq!(
        risky.action_request().parameter_hash().as_str(),
        "dd61f447dd966d3b9e0546fdbb9ba5de9685f327dec4f69a98d7c35aa21c5355"
    );

    let destructive = ScenarioFixture::from_json(DESTRUCTIVE).expect("destructive scenario");
    assert!(destructive.is_destructive());
    assert_eq!(
        destructive.expected_risk(),
        RiskLevel::DestructiveOrFinancial
    );
    assert_eq!(destructive.expected_decision(), PolicyDecision::Deny);
}

#[test]
fn tampered_parameters_labels_and_duplicate_fields_are_rejected() {
    let changed_recipient = RISKY.replace("sales-manager@example.invalid", "other@example.invalid");
    assert!(ScenarioFixture::from_json(&changed_recipient).is_err());
    let lowered_risk = RISKY.replace("\"L3\"", "\"L0\"");
    assert!(ScenarioFixture::from_json(&lowered_risk).is_err());
    let duplicate_action = RISKY.replace(
        "\"action\": \"email.send\"",
        "\"action\": \"email.send\",\n  \"action\": \"database.delete\"",
    );
    assert!(ScenarioFixture::from_json(&duplicate_action).is_err());
}

#[test]
fn malformed_scenario_errors_do_not_echo_secrets() {
    let error = ScenarioFixture::from_json(r#"{"password":"visible"}"#)
        .err()
        .expect("malformed fixture");
    assert!(!error.to_string().contains("visible"));
}
