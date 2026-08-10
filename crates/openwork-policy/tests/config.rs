use openwork_policy::config::PolicyConfig;
use serde_json::json;

const VALID: &str = r"
version: 1
defaults:
  unknown: deny
actions:
  filesystem.read:
    risk: L0
    decision: allow
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
fn valid_versioned_config_loads() {
    PolicyConfig::from_yaml(VALID).expect("valid policy");
}

#[test]
fn unknown_duplicate_and_unsafe_config_fail_closed() {
    for invalid in [
        VALID.replace("version: 1", "version: 2"),
        VALID.replace("unknown: deny", "unknown: allow"),
        VALID.replace(
            "decision: approval\n    resources:",
            "decision: allow\n    resources:",
        ),
        VALID.replace("decision: deny\n", "decision: allow\n"),
        format!("{VALID}\nextra: true"),
        format!("{VALID}\nactions:\n  filesystem.read:\n    risk: L0\n    decision: allow"),
    ] {
        assert!(
            PolicyConfig::from_yaml(&invalid).is_err(),
            "accepted invalid policy"
        );
    }
}

#[test]
fn duplicate_resource_and_secret_parse_errors_are_rejected_without_echo() {
    let duplicate = VALID.replace(
        "finance@company.com: approval",
        "finance@company.com: approval\n        finance@company.com: deny",
    );
    assert!(PolicyConfig::from_yaml(&duplicate).is_err());

    let secret = "version: 1\ndefaults: [password=visible]\nactions: {}";
    let error = PolicyConfig::from_yaml(secret).expect_err("invalid policy");
    assert!(!error.to_string().contains("visible"));
}

#[test]
fn checked_in_schema_enforces_syntax_and_m1_risk_floor() {
    let schema: serde_json::Value = serde_json::from_str(include_str!(
        "../../../contracts/schemas/policy/policy.v1.schema.json"
    ))
    .expect("schema JSON");
    let validator = jsonschema::validator_for(&schema).expect("schema compiles");
    let valid = json!({
        "version": 1,
        "defaults": {"unknown": "deny"},
        "actions": {"email.send": {"risk": "L3", "decision": "approval"}}
    });
    assert!(validator.is_valid(&valid));
    let unsafe_l4 = json!({
        "version": 1,
        "defaults": {"unknown": "deny"},
        "actions": {"database.delete": {"risk": "L4", "decision": "allow"}}
    });
    assert!(!validator.is_valid(&unsafe_l4));
    let unknown_field = json!({
        "version": 1,
        "defaults": {"unknown": "deny"},
        "actions": {},
        "password": "visible"
    });
    assert!(!validator.is_valid(&unknown_field));
}
