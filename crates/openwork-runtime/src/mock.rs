use crate::{
    AgentRuntime, AuthStatus, CancellationToken, DetectionState, DistributionModel,
    RuntimeCapabilities, RuntimeDetection, RuntimeDoctorCheck, RuntimeEvent, RuntimeEventKind,
    RuntimeId, RuntimeInstallOutcome, RuntimeInstallPlan, RuntimeMetadata, RuntimeResult,
    RuntimeRunRequest,
};
use openwork_core::{ErrorCode, OpenWorkError, redact_text};
use std::path::PathBuf;
use std::sync::{Mutex, MutexGuard};

#[derive(Debug)]
struct MockState {
    installed: bool,
    version: Option<String>,
    authenticated: bool,
}

pub struct MockRuntime {
    id: RuntimeId,
    state: Mutex<MockState>,
}

impl Default for MockRuntime {
    fn default() -> Self {
        Self::new("mock")
    }
}

impl MockRuntime {
    #[must_use]
    pub fn new(id: &str) -> Self {
        Self {
            id: RuntimeId::from(id),
            state: Mutex::new(MockState {
                installed: false,
                version: None,
                authenticated: false,
            }),
        }
    }

    /// Changes synthetic authentication state for fixture tests.
    ///
    /// # Errors
    ///
    /// Returns an internal error if another test poisoned the state lock.
    pub fn set_authenticated(&self, authenticated: bool) -> RuntimeResult<()> {
        self.state()?.authenticated = authenticated;
        Ok(())
    }

    fn state(&self) -> RuntimeResult<MutexGuard<'_, MockState>> {
        self.state.lock().map_err(|_| {
            OpenWorkError::new(ErrorCode::Internal, "mock runtime state lock was poisoned")
        })
    }
}

impl AgentRuntime for MockRuntime {
    fn metadata(&self) -> RuntimeMetadata {
        RuntimeMetadata {
            id: self.id.clone(),
            name: "OpenWork Mock Runtime".to_owned(),
            upstream: "https://github.com/shichenghaoshu/openwork".to_owned(),
            license: "Apache-2.0".to_owned(),
            distribution: DistributionModel::Embedded,
        }
    }

    fn detect(&self) -> RuntimeResult<RuntimeDetection> {
        let state = self.state()?;
        Ok(if state.installed {
            RuntimeDetection {
                state: DetectionState::Healthy,
                executable: Some(PathBuf::from("/fixture/openwork-mock")),
                details: None,
            }
        } else {
            RuntimeDetection {
                state: DetectionState::Missing,
                executable: None,
                details: None,
            }
        })
    }

    fn install_plan(&self, version: Option<&str>) -> RuntimeResult<RuntimeInstallPlan> {
        Ok(RuntimeInstallPlan {
            source_url: "https://example.invalid/openwork/mock".to_owned(),
            version: version.map(str::to_owned),
            commands: vec![],
            warnings: vec!["Synthetic fixture only; no command will execute.".to_owned()],
        })
    }

    fn install(&self, plan: &RuntimeInstallPlan) -> RuntimeResult<RuntimeInstallOutcome> {
        let mut state = self.state()?;
        let version = plan.version.clone().unwrap_or_else(|| "1.0.0".to_owned());
        state.installed = true;
        state.version = Some(version.clone());
        Ok(RuntimeInstallOutcome {
            installed: true,
            version: Some(version),
            executable: Some(PathBuf::from("/fixture/openwork-mock")),
        })
    }

    fn uninstall(&self) -> RuntimeResult<()> {
        let mut state = self.state()?;
        state.installed = false;
        state.version = None;
        Ok(())
    }

    fn version(&self) -> RuntimeResult<Option<String>> {
        Ok(self.state()?.version.clone())
    }

    fn update(&self, version: Option<&str>) -> RuntimeResult<RuntimeInstallOutcome> {
        let plan = self.install_plan(version)?;
        self.install(&plan)
    }

    fn doctor(&self) -> RuntimeResult<Vec<RuntimeDoctorCheck>> {
        let installed = self.state()?.installed;
        Ok(vec![RuntimeDoctorCheck {
            id: "mock.installed".to_owned(),
            healthy: installed,
            summary: if installed {
                "Mock runtime is installed"
            } else {
                "Mock runtime is missing"
            }
            .to_owned(),
            remediation: (!installed).then(|| "Install the isolated mock plan.".to_owned()),
        }])
    }

    fn auth_status(&self) -> RuntimeResult<AuthStatus> {
        Ok(if self.state()?.authenticated {
            AuthStatus::Authenticated
        } else {
            AuthStatus::Unauthenticated
        })
    }

    fn capabilities(&self) -> RuntimeCapabilities {
        RuntimeCapabilities {
            install: true,
            uninstall: true,
            update: true,
            authenticate: true,
            run: true,
            cancel: true,
        }
    }

    fn run(
        &self,
        request: &RuntimeRunRequest,
        cancellation: &CancellationToken,
    ) -> RuntimeResult<Vec<RuntimeEvent>> {
        if !self.state()?.installed {
            return Err(OpenWorkError::new(
                ErrorCode::RuntimeNotFound,
                "mock runtime is not installed",
            ));
        }
        if cancellation.is_cancelled() {
            return Ok(vec![RuntimeEvent {
                kind: RuntimeEventKind::Cancelled,
                message: "Mock run cancelled".to_owned(),
            }]);
        }
        Ok(vec![
            RuntimeEvent {
                kind: RuntimeEventKind::Started,
                message: "Mock run started".to_owned(),
            },
            RuntimeEvent {
                kind: RuntimeEventKind::Output,
                message: redact_text(&request.prompt),
            },
            RuntimeEvent {
                kind: RuntimeEventKind::Completed,
                message: "Mock run completed".to_owned(),
            },
        ])
    }

    fn cancel(&self, cancellation: &CancellationToken) -> RuntimeResult<()> {
        cancellation.cancel();
        Ok(())
    }
}
