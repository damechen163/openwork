//! Audit append inputs and canonical event construction.

use crate::{
    ActorId, AuditEvent, AuditEventId, AuditEventType, RedactedAuditMetadata, RunId, Sha256Digest,
    UtcTimestamp,
};
use openwork_core::OpenWorkError;
use serde_json::Value;
use std::collections::BTreeMap;

/// Trusted envelope plus untrusted metadata for one audit append transaction.
#[derive(Clone, Debug)]
pub struct AuditAppend {
    pub actor: ActorId,
    pub timestamp: UtcTimestamp,
    pub metadata: BTreeMap<String, Value>,
}

impl AuditAppend {
    pub(crate) fn build(
        &self,
        run_id: RunId,
        sequence: u64,
        event_type: AuditEventType,
        previous_hash: Option<Sha256Digest>,
    ) -> Result<AuditEvent, OpenWorkError> {
        AuditEvent::new(
            AuditEventId::generate(),
            run_id,
            sequence,
            event_type,
            self.actor.clone(),
            self.timestamp,
            RedactedAuditMetadata::from_untrusted(&self.metadata),
            previous_hash,
        )
    }
}
