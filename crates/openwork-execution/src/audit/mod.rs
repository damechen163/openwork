//! Audit append inputs and canonical event construction.

use crate::{
    ActionId, ActorId, Artifact, AuditEvent, AuditEventId, AuditEventType, RedactedAuditMetadata,
    RunId, RunStatus, Sha256Digest, UtcTimestamp,
};
use openwork_core::OpenWorkError;
use serde_json::Value;
use std::collections::BTreeMap;

/// Trusted audit envelope. Event-specific metadata must use typed constructors
/// instead of accepting arbitrary runtime or tool content.
#[derive(Clone, Debug)]
pub struct AuditAppend {
    pub actor: ActorId,
    pub timestamp: UtcTimestamp,
    details: AuditDetails,
}

#[derive(Clone, Debug, Default)]
enum AuditDetails {
    #[default]
    None,
    RunStatus(RunStatus),
    Artifact {
        sha256: Sha256Digest,
        size_bytes: u64,
    },
    ActionExecution {
        action_id: ActionId,
        parameter_hash: Sha256Digest,
    },
}

impl AuditAppend {
    #[must_use]
    pub const fn new(actor: ActorId, timestamp: UtcTimestamp) -> Self {
        Self {
            actor,
            timestamp,
            details: AuditDetails::None,
        }
    }

    pub(crate) fn with_run_status(mut self, status: RunStatus) -> Self {
        self.details = AuditDetails::RunStatus(status);
        self
    }

    pub(crate) fn with_artifact(mut self, artifact: &Artifact) -> Self {
        self.details = AuditDetails::Artifact {
            sha256: artifact.sha256.clone(),
            size_bytes: artifact.size_bytes.get(),
        };
        self
    }

    pub(crate) fn with_action_execution(
        mut self,
        action_id: ActionId,
        parameter_hash: Sha256Digest,
    ) -> Self {
        self.details = AuditDetails::ActionExecution {
            action_id,
            parameter_hash,
        };
        self
    }

    pub(crate) fn build(
        &self,
        run_id: RunId,
        sequence: u64,
        event_type: AuditEventType,
        previous_hash: Option<Sha256Digest>,
    ) -> Result<AuditEvent, OpenWorkError> {
        let metadata = match &self.details {
            AuditDetails::None => BTreeMap::new(),
            AuditDetails::RunStatus(status) => BTreeMap::from([(
                "run_status".to_owned(),
                Value::String(run_status_name(*status).to_owned()),
            )]),
            AuditDetails::Artifact { sha256, size_bytes } => BTreeMap::from([
                (
                    "artifact_sha256".to_owned(),
                    Value::String(sha256.as_str().to_owned()),
                ),
                (
                    "artifact_size_bytes".to_owned(),
                    Value::Number((*size_bytes).into()),
                ),
            ]),
            AuditDetails::ActionExecution {
                action_id,
                parameter_hash,
            } => BTreeMap::from([
                (
                    "action_id".to_owned(),
                    Value::String(action_id.to_hyphenated()),
                ),
                (
                    "parameter_hash".to_owned(),
                    Value::String(parameter_hash.as_str().to_owned()),
                ),
            ]),
        };
        AuditEvent::new(
            AuditEventId::generate(),
            run_id,
            sequence,
            event_type,
            self.actor.clone(),
            self.timestamp,
            RedactedAuditMetadata::from_untrusted(&metadata),
            previous_hash,
        )
    }
}

const fn run_status_name(status: RunStatus) -> &'static str {
    match status {
        RunStatus::Queued => "queued",
        RunStatus::Planning => "planning",
        RunStatus::AwaitingApproval => "awaiting_approval",
        RunStatus::Running => "running",
        RunStatus::Succeeded => "succeeded",
        RunStatus::Failed => "failed",
        RunStatus::Cancelled => "cancelled",
        RunStatus::TimedOut => "timed_out",
    }
}
