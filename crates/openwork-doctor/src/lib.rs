//! Structured host and runtime diagnostic checks.

use openwork_platform::{PlatformInfo, SupportTier};
use serde::{Deserialize, Serialize};
use std::time::{SystemTime, UNIX_EPOCH};

pub const DOCTOR_SCHEMA_VERSION: u32 = 1;
const MIN_MEMORY_BYTES: u64 = 8 * 1024 * 1024 * 1024;
const MIN_DISK_BYTES: u64 = 10 * 1024 * 1024 * 1024;

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum CheckStatus {
    Pass,
    Warn,
    Fail,
    Skip,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct DoctorCheck {
    pub id: String,
    pub status: CheckStatus,
    pub summary: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub details: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub remediation: Option<String>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct DoctorSummary {
    pub pass: usize,
    pub warn: usize,
    pub fail: usize,
    pub skip: usize,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct DoctorReport {
    pub schema_version: u32,
    pub generated_at_unix_seconds: u64,
    pub checks: Vec<DoctorCheck>,
    pub summary: DoctorSummary,
}

impl DoctorReport {
    #[must_use]
    pub fn has_failures(&self) -> bool {
        self.summary.fail > 0
    }
}

#[must_use]
pub fn inspect_platform(platform: &PlatformInfo) -> DoctorReport {
    let mut checks = vec![support_check(platform)];
    checks.extend([
        boolean_check(
            "permissions.home",
            platform.permissions.home_writable,
            "Home directory is writable",
            "Home directory is not writable",
            "Fix home-directory ownership or choose a writable OpenWork home.",
        ),
        boolean_check(
            "permissions.install",
            platform.permissions.install_dir_writable,
            "Install directory is writable",
            "Install directory is not writable",
            "Choose a user-writable bin directory; do not run the whole installer as root.",
        ),
        resource_check(
            "resources.memory",
            platform.resources.total_memory_bytes,
            MIN_MEMORY_BYTES,
            "memory",
        ),
        resource_check(
            "resources.disk",
            platform.resources.available_disk_bytes,
            MIN_DISK_BYTES,
            "disk space",
        ),
        boolean_check(
            "prerequisite.git",
            platform.prerequisites.git_present,
            "Git is available",
            "Git is missing",
            "Install Git using the host package manager, then rerun doctor.",
        ),
        optional_docker_check(platform.prerequisites.docker_present),
    ]);

    DoctorReport {
        schema_version: DOCTOR_SCHEMA_VERSION,
        generated_at_unix_seconds: SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_or(0, |duration| duration.as_secs()),
        summary: summarize(&checks),
        checks,
    }
}

fn support_check(platform: &PlatformInfo) -> DoctorCheck {
    match platform.support_tier {
        SupportTier::Tier1 => check(
            "platform.support",
            CheckStatus::Pass,
            "Tier 1 host is supported",
        ),
        SupportTier::Tier2 => DoctorCheck {
            remediation: Some("Use a Tier 1 host for the Bootstrap release path.".to_owned()),
            ..check(
                "platform.support",
                CheckStatus::Warn,
                "Tier 2 host has limited validation",
            )
        },
    }
}

fn boolean_check(
    id: &str,
    value: bool,
    success: &str,
    failure: &str,
    remediation: &str,
) -> DoctorCheck {
    if value {
        check(id, CheckStatus::Pass, success)
    } else {
        DoctorCheck {
            remediation: Some(remediation.to_owned()),
            ..check(id, CheckStatus::Fail, failure)
        }
    }
}

fn resource_check(id: &str, actual: Option<u64>, minimum: u64, label: &str) -> DoctorCheck {
    match actual {
        None => check(
            id,
            CheckStatus::Skip,
            &format!("{label} could not be measured"),
        ),
        Some(bytes) if bytes >= minimum => DoctorCheck {
            details: Some(format!("{bytes} bytes available")),
            ..check(id, CheckStatus::Pass, &format!("Sufficient {label}"))
        },
        Some(bytes) => DoctorCheck {
            details: Some(format!("{bytes} bytes available; {minimum} required")),
            remediation: Some(format!(
                "Free resources until at least {minimum} bytes are available."
            )),
            ..check(id, CheckStatus::Fail, &format!("Insufficient {label}"))
        },
    }
}

fn optional_docker_check(present: bool) -> DoctorCheck {
    if present {
        check("optional.docker", CheckStatus::Pass, "Docker is available")
    } else {
        check(
            "optional.docker",
            CheckStatus::Skip,
            "Docker is optional and was not found",
        )
    }
}

fn check(id: &str, status: CheckStatus, summary: &str) -> DoctorCheck {
    DoctorCheck {
        id: id.to_owned(),
        status,
        summary: summary.to_owned(),
        details: None,
        remediation: None,
    }
}

fn summarize(checks: &[DoctorCheck]) -> DoctorSummary {
    let mut summary = DoctorSummary {
        pass: 0,
        warn: 0,
        fail: 0,
        skip: 0,
    };
    for check in checks {
        match check.status {
            CheckStatus::Pass => summary.pass += 1,
            CheckStatus::Warn => summary.warn += 1,
            CheckStatus::Fail => summary.fail += 1,
            CheckStatus::Skip => summary.skip += 1,
        }
    }
    summary
}

#[cfg(test)]
mod tests {
    use super::*;
    use openwork_platform::{
        Architecture, HostEnvironment, OpenWorkPaths, OperatingSystem, PermissionFacts,
        PrerequisiteFacts, ResourceFacts,
    };
    use std::path::PathBuf;

    fn fixture() -> PlatformInfo {
        PlatformInfo {
            schema_version: 1,
            os: OperatingSystem::MacOs,
            os_version: Some("fixture".to_owned()),
            architecture: Architecture::Arm64,
            environment: HostEnvironment::Native,
            support_tier: SupportTier::Tier1,
            shell: Some("/bin/zsh".to_owned()),
            package_managers: vec!["brew".to_owned()],
            paths: OpenWorkPaths {
                config: PathBuf::from("/tmp/config"),
                data: PathBuf::from("/tmp/data"),
                cache: PathBuf::from("/tmp/cache"),
                logs: PathBuf::from("/tmp/logs"),
                bin: PathBuf::from("/tmp/bin"),
            },
            permissions: PermissionFacts {
                home_writable: true,
                install_dir_writable: true,
                elevated: false,
            },
            resources: ResourceFacts {
                total_memory_bytes: Some(MIN_MEMORY_BYTES),
                available_disk_bytes: Some(MIN_DISK_BYTES),
            },
            prerequisites: PrerequisiteFacts {
                git_present: true,
                docker_present: false,
            },
        }
    }

    #[test]
    fn healthy_host_passes_and_skips_optional_docker() {
        let report = inspect_platform(&fixture());
        assert!(!report.has_failures());
        assert_eq!(report.summary.pass, 6);
        assert_eq!(report.summary.skip, 1);
    }

    #[test]
    fn failures_include_remediation() {
        let mut platform = fixture();
        platform.permissions.home_writable = false;
        platform.resources.total_memory_bytes = Some(1);
        let report = inspect_platform(&platform);
        assert_eq!(report.summary.fail, 2);
        assert!(
            report
                .checks
                .iter()
                .filter(|check| check.status == CheckStatus::Fail)
                .all(|check| check.remediation.is_some())
        );
    }

    #[test]
    fn json_is_versioned_and_uses_stable_status_names() {
        let value = serde_json::to_value(inspect_platform(&fixture())).unwrap();
        assert_eq!(value["schema_version"], 1);
        assert_eq!(value["checks"][0]["status"], "PASS");

        let schema: serde_json::Value = serde_json::from_str(include_str!(
            "../../../contracts/schemas/doctor-report.v1.schema.json"
        ))
        .unwrap();
        assert_eq!(schema["properties"]["schema_version"]["const"], 1);
    }
}
