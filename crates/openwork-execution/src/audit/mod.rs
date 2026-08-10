//! Audit append inputs and canonical event construction.

use crate::{
    ActorId, AuditEvent, AuditEventId, AuditEventType, RedactedAuditMetadata, RunId, Sha256Digest,
    UtcTimestamp,
};
use openwork_core::OpenWorkError;
use std::collections::BTreeMap;

/// Trusted audit envelope. Event-specific metadata must use typed constructors
/// instead of accepting arbitrary runtime or tool content.
#[derive(Clone, Debug)]
pub struct AuditAppend {
    pub actor: ActorId,
    pub timestamp: UtcTimestamp,
}

impl AuditAppend {
    #[must_use]
    pub const fn new(actor: ActorId, timestamp: UtcTimestamp) -> Self {
        Self { actor, timestamp }
    }

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
            RedactedAuditMetadata::from_untrusted(&BTreeMap::default()),
            previous_hash,
        )
    }
}
