//! Reversible installation planning and execution.

use openwork_platform::PlatformInfo;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

pub const INSTALL_PLAN_SCHEMA_VERSION: u32 = 1;

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum InstallAction {
    CreateDirectory,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct InstallStep {
    pub id: String,
    pub action: InstallAction,
    pub path: PathBuf,
    pub reason: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct InstallPlan {
    pub schema_version: u32,
    pub dry_run: bool,
    pub steps: Vec<InstallStep>,
    pub warnings: Vec<String>,
}

/// Builds the side-effect-free Bootstrap directory plan.
#[must_use]
pub fn dry_run_plan(platform: &PlatformInfo) -> InstallPlan {
    let paths = &platform.paths;
    let steps = [
        ("config", &paths.config),
        ("data", &paths.data),
        ("cache", &paths.cache),
        ("logs", &paths.logs),
        ("bin", &paths.bin),
    ]
    .into_iter()
    .map(|(id, path)| InstallStep {
        id: format!("directory.{id}"),
        action: InstallAction::CreateDirectory,
        path: path.clone(),
        reason: format!("Prepare the OpenWork {id} location"),
    })
    .collect();

    InstallPlan {
        schema_version: INSTALL_PLAN_SCHEMA_VERSION,
        dry_run: true,
        steps,
        warnings: vec![
            "Dry-run only: no directories, downloads, or subprocesses were executed.".to_owned(),
            "Runtime install steps will be added after runtime selection.".to_owned(),
        ],
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use openwork_platform::{
        Architecture, HostEnvironment, OpenWorkPaths, OperatingSystem, PermissionFacts,
        PlatformInfo, PrerequisiteFacts, ResourceFacts, SupportTier,
    };

    #[test]
    fn dry_run_contains_only_declarative_steps() {
        let root = PathBuf::from("/path/that/openwork/test/does/not/create");
        let platform = PlatformInfo {
            schema_version: 1,
            os: OperatingSystem::Linux,
            os_version: None,
            architecture: Architecture::X64,
            environment: HostEnvironment::Native,
            support_tier: SupportTier::Tier1,
            shell: None,
            package_managers: vec![],
            paths: OpenWorkPaths {
                config: root.join("config"),
                data: root.join("data"),
                cache: root.join("cache"),
                logs: root.join("logs"),
                bin: root.join("bin"),
            },
            permissions: PermissionFacts {
                home_writable: true,
                install_dir_writable: true,
                elevated: false,
            },
            resources: ResourceFacts {
                total_memory_bytes: None,
                available_disk_bytes: None,
            },
            prerequisites: PrerequisiteFacts {
                git_present: true,
                docker_present: false,
            },
        };
        let plan = dry_run_plan(&platform);
        assert!(plan.dry_run);
        assert_eq!(plan.steps.len(), 5);
        assert!(plan.steps.iter().all(|step| !step.path.exists()));
    }
}
