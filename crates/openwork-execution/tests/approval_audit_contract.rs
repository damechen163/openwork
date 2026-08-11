use openwork_execution::{
    ActorId, AuditEvent, AuditEventId, AuditEventType, RedactedAuditMetadata, RunId, UtcTimestamp,
};
use serde_json::Value;
use std::collections::BTreeMap;

#[test]
fn rust_and_schema_use_exact_approval_lifecycle_event_names() {
    let cases = [
        (AuditEventType::ApprovalExpired, "approval_expired"),
        (AuditEventType::ApprovalConsumed, "approval_consumed"),
    ];
    for (event_type, expected) in cases {
        let event = AuditEvent::new(
            AuditEventId::generate(),
            RunId::parse("01890f3e-a5f1-7cc2-98c0-5f9c6f5e7a01").unwrap(),
            1,
            event_type,
            ActorId::parse("system:approval").unwrap(),
            UtcTimestamp::parse("2026-08-10T00:00:00Z").unwrap(),
            RedactedAuditMetadata::from_untrusted(&BTreeMap::new()),
            None,
        )
        .unwrap();
        assert_eq!(serde_json::to_value(event).unwrap()["event_type"], expected);
    }

    let schema: Value = serde_json::from_str(include_str!(
        "../../../contracts/schemas/safe-execution.v1.schema.json"
    ))
    .unwrap();
    let values = schema["$defs"]["auditEvent"]["properties"]["event_type"]["enum"]
        .as_array()
        .unwrap();
    for expected in ["approval_expired", "approval_consumed"] {
        assert!(values.iter().any(|value| value == expected));
    }
}
