use crate::{AgentRuntime, RuntimeId, RuntimeMetadata, RuntimeResult};
use openwork_core::{ErrorCode, OpenWorkError};
use std::collections::BTreeMap;
use std::sync::Arc;

#[derive(Default)]
pub struct RuntimeRegistry {
    runtimes: BTreeMap<RuntimeId, Arc<dyn AgentRuntime>>,
}

impl RuntimeRegistry {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Registers one provider using its stable runtime identifier.
    ///
    /// # Errors
    ///
    /// Returns `config_invalid` for an empty or duplicate identifier.
    pub fn register(&mut self, runtime: Arc<dyn AgentRuntime>) -> RuntimeResult<()> {
        let id = runtime.metadata().id;
        if id.0.trim().is_empty() {
            return Err(OpenWorkError::new(
                ErrorCode::ConfigInvalid,
                "runtime id cannot be empty",
            ));
        }
        if self.runtimes.contains_key(&id) {
            return Err(OpenWorkError::new(
                ErrorCode::ConfigInvalid,
                format!("runtime `{id}` is already registered"),
            ));
        }
        self.runtimes.insert(id, runtime);
        Ok(())
    }

    #[must_use]
    pub fn get(&self, id: &RuntimeId) -> Option<Arc<dyn AgentRuntime>> {
        self.runtimes.get(id).cloned()
    }

    #[must_use]
    pub fn metadata(&self) -> Vec<RuntimeMetadata> {
        self.runtimes
            .values()
            .map(|runtime| runtime.metadata())
            .collect()
    }

    #[must_use]
    pub fn len(&self) -> usize {
        self.runtimes.len()
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.runtimes.is_empty()
    }
}
