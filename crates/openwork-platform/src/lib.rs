//! Cross-platform host detection, paths, permissions, and prerequisite probes.

use serde::{Deserialize, Serialize};
use std::env;
use std::fmt;
use std::path::{Path, PathBuf};
use sysinfo::{MemoryRefreshKind, RefreshKind, System};

/// Current schema for serialized platform facts.
pub const PLATFORM_SCHEMA_VERSION: u32 = 1;

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OperatingSystem {
    MacOs,
    Linux,
    Windows,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Architecture {
    Arm64,
    X64,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HostEnvironment {
    Native,
    Wsl,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SupportTier {
    Tier1,
    Tier2,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct OpenWorkPaths {
    pub config: PathBuf,
    pub data: PathBuf,
    pub cache: PathBuf,
    pub logs: PathBuf,
    pub bin: PathBuf,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct PermissionFacts {
    pub home_writable: bool,
    pub install_dir_writable: bool,
    pub elevated: bool,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ResourceFacts {
    pub total_memory_bytes: Option<u64>,
    pub available_disk_bytes: Option<u64>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct PrerequisiteFacts {
    pub git_present: bool,
    pub docker_present: bool,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct PlatformInfo {
    pub schema_version: u32,
    pub os: OperatingSystem,
    pub os_version: Option<String>,
    pub architecture: Architecture,
    pub environment: HostEnvironment,
    pub support_tier: SupportTier,
    pub shell: Option<String>,
    pub package_managers: Vec<String>,
    pub paths: OpenWorkPaths,
    pub permissions: PermissionFacts,
    pub resources: ResourceFacts,
    pub prerequisites: PrerequisiteFacts,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum PlatformError {
    UnsupportedOs(String),
    UnsupportedArchitecture(String),
    HomeDirectoryUnavailable,
}

impl fmt::Display for PlatformError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnsupportedOs(os) => write!(
                formatter,
                "unsupported operating system `{os}`; use macOS, Ubuntu Linux, Windows 11, or WSL2"
            ),
            Self::UnsupportedArchitecture(architecture) => write!(
                formatter,
                "unsupported architecture `{architecture}`; use arm64/aarch64 or x64/x86_64"
            ),
            Self::HomeDirectoryUnavailable => write!(
                formatter,
                "home directory is unavailable; set HOME or USERPROFILE before running OpenWork"
            ),
        }
    }
}

impl std::error::Error for PlatformError {}

/// Injectable facts used by the detector; tests never inspect the developer host.
pub trait PlatformProbe {
    fn raw_os(&self) -> String;
    fn raw_architecture(&self) -> String;
    fn environment(&self, key: &str) -> Option<String>;
    fn home_dir(&self) -> Option<PathBuf>;
    fn file_contains(&self, path: &Path, needle: &str) -> bool;
    fn executable_exists(&self, executable: &str) -> bool;
    fn path_writable(&self, path: &Path) -> bool;
    fn total_memory_bytes(&self) -> Option<u64>;
    fn available_disk_bytes(&self, path: &Path) -> Option<u64>;
    fn os_version(&self) -> Option<String>;
    fn elevated(&self) -> bool;
}

#[derive(Clone, Copy, Debug, Default)]
pub struct SystemPlatformProbe;

impl PlatformProbe for SystemPlatformProbe {
    fn raw_os(&self) -> String {
        env::consts::OS.to_owned()
    }

    fn raw_architecture(&self) -> String {
        env::consts::ARCH.to_owned()
    }

    fn environment(&self, key: &str) -> Option<String> {
        env::var(key).ok().filter(|value| !value.is_empty())
    }

    fn home_dir(&self) -> Option<PathBuf> {
        self.environment(if cfg!(windows) { "USERPROFILE" } else { "HOME" })
            .map(PathBuf::from)
    }

    fn file_contains(&self, path: &Path, needle: &str) -> bool {
        std::fs::read_to_string(path).is_ok_and(|text| {
            text.to_ascii_lowercase()
                .contains(&needle.to_ascii_lowercase())
        })
    }

    fn executable_exists(&self, executable: &str) -> bool {
        executable_on_path(executable, env::var_os("PATH").as_deref())
    }

    fn path_writable(&self, path: &Path) -> bool {
        nearest_existing(path)
            .and_then(|existing| std::fs::metadata(existing).ok())
            .is_some_and(|metadata| !metadata.permissions().readonly())
    }

    fn total_memory_bytes(&self) -> Option<u64> {
        let system = System::new_with_specifics(
            RefreshKind::nothing().with_memory(MemoryRefreshKind::everything()),
        );
        (system.total_memory() > 0).then_some(system.total_memory())
    }

    fn available_disk_bytes(&self, path: &Path) -> Option<u64> {
        nearest_existing(path).and_then(|existing| fs2::available_space(existing).ok())
    }

    fn os_version(&self) -> Option<String> {
        System::long_os_version()
    }

    fn elevated(&self) -> bool {
        self.environment("USER").is_some_and(|user| user == "root")
            || self
                .environment("USERNAME")
                .is_some_and(|user| user.eq_ignore_ascii_case("administrator"))
    }
}

/// Detects normalized host facts without changing the host.
///
/// # Errors
///
/// Returns an actionable error for unsupported OS/architecture pairs or when the
/// home directory required for safe `OpenWork` paths cannot be resolved.
pub fn detect(probe: &impl PlatformProbe) -> Result<PlatformInfo, PlatformError> {
    let os = normalize_os(&probe.raw_os())?;
    let architecture = normalize_architecture(&probe.raw_architecture())?;
    let environment = detect_environment(probe, &os);
    let home = probe
        .home_dir()
        .ok_or(PlatformError::HomeDirectoryUnavailable)?;
    let paths = platform_paths(probe, &os, &home);
    let package_managers = ["brew", "apt-get", "dnf", "pacman", "winget", "choco"]
        .into_iter()
        .filter(|manager| probe.executable_exists(manager))
        .map(str::to_owned)
        .collect();

    Ok(PlatformInfo {
        schema_version: PLATFORM_SCHEMA_VERSION,
        support_tier: support_tier(&os, &architecture, &environment),
        shell: probe.environment(if os == OperatingSystem::Windows {
            "COMSPEC"
        } else {
            "SHELL"
        }),
        package_managers,
        permissions: PermissionFacts {
            home_writable: probe.path_writable(&home),
            install_dir_writable: probe.path_writable(&paths.bin),
            elevated: probe.elevated(),
        },
        resources: ResourceFacts {
            total_memory_bytes: probe.total_memory_bytes(),
            available_disk_bytes: probe.available_disk_bytes(&home),
        },
        prerequisites: PrerequisiteFacts {
            git_present: probe.executable_exists("git"),
            docker_present: probe.executable_exists("docker"),
        },
        os,
        os_version: probe.os_version(),
        architecture,
        environment,
        paths,
    })
}

fn normalize_os(raw: &str) -> Result<OperatingSystem, PlatformError> {
    match raw.to_ascii_lowercase().as_str() {
        "macos" | "darwin" => Ok(OperatingSystem::MacOs),
        "linux" => Ok(OperatingSystem::Linux),
        "windows" => Ok(OperatingSystem::Windows),
        other => Err(PlatformError::UnsupportedOs(other.to_owned())),
    }
}

fn normalize_architecture(raw: &str) -> Result<Architecture, PlatformError> {
    match raw.to_ascii_lowercase().as_str() {
        "aarch64" | "arm64" => Ok(Architecture::Arm64),
        "x86_64" | "x64" | "amd64" => Ok(Architecture::X64),
        other => Err(PlatformError::UnsupportedArchitecture(other.to_owned())),
    }
}

fn detect_environment(probe: &impl PlatformProbe, os: &OperatingSystem) -> HostEnvironment {
    if *os == OperatingSystem::Linux
        && (probe.environment("WSL_DISTRO_NAME").is_some()
            || probe.file_contains(Path::new("/proc/version"), "microsoft"))
    {
        HostEnvironment::Wsl
    } else {
        HostEnvironment::Native
    }
}

fn support_tier(
    os: &OperatingSystem,
    architecture: &Architecture,
    environment: &HostEnvironment,
) -> SupportTier {
    match (os, architecture, environment) {
        (OperatingSystem::Windows, Architecture::Arm64, HostEnvironment::Native) => {
            SupportTier::Tier2
        }
        _ => SupportTier::Tier1,
    }
}

fn platform_paths(probe: &impl PlatformProbe, os: &OperatingSystem, home: &Path) -> OpenWorkPaths {
    match os {
        OperatingSystem::MacOs => OpenWorkPaths {
            config: home.join("Library/Application Support/OpenWork/config"),
            data: home.join("Library/Application Support/OpenWork/data"),
            cache: home.join("Library/Caches/OpenWork"),
            logs: home.join("Library/Logs/OpenWork"),
            bin: home.join(".local/bin"),
        },
        OperatingSystem::Linux => OpenWorkPaths {
            config: env_path(probe, "XDG_CONFIG_HOME", home.join(".config")).join("openwork"),
            data: env_path(probe, "XDG_DATA_HOME", home.join(".local/share")).join("openwork"),
            cache: env_path(probe, "XDG_CACHE_HOME", home.join(".cache")).join("openwork"),
            logs: env_path(probe, "XDG_STATE_HOME", home.join(".local/state"))
                .join("openwork/logs"),
            bin: home.join(".local/bin"),
        },
        OperatingSystem::Windows => {
            let roaming = env_path(probe, "APPDATA", home.join("AppData/Roaming"));
            let local = env_path(probe, "LOCALAPPDATA", home.join("AppData/Local"));
            OpenWorkPaths {
                config: roaming.join("OpenWork/config"),
                data: local.join("OpenWork/data"),
                cache: local.join("OpenWork/cache"),
                logs: local.join("OpenWork/logs"),
                bin: local.join("OpenWork/bin"),
            }
        }
    }
}

fn env_path(probe: &impl PlatformProbe, key: &str, fallback: PathBuf) -> PathBuf {
    probe.environment(key).map_or(fallback, PathBuf::from)
}

fn nearest_existing(path: &Path) -> Option<&Path> {
    path.ancestors().find(|candidate| candidate.exists())
}

fn executable_on_path(executable: &str, path: Option<&std::ffi::OsStr>) -> bool {
    let extensions = env::var("PATHEXT").unwrap_or_else(|_| ".EXE;.CMD;.BAT".to_owned());
    path.into_iter()
        .flat_map(env::split_paths)
        .any(|directory| {
            let direct = directory.join(executable);
            direct.is_file()
                || cfg!(windows)
                    && extensions.split(';').any(|extension| {
                        directory.join(format!("{executable}{extension}")).is_file()
                    })
        })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeMap;

    struct FixtureProbe {
        os: String,
        arch: String,
        env: BTreeMap<String, String>,
        executables: Vec<String>,
        proc_is_wsl: bool,
    }

    impl FixtureProbe {
        fn new(os: &str, arch: &str) -> Self {
            let home = if os == "windows" {
                ("USERPROFILE".to_owned(), r"C:\Users\test".to_owned())
            } else {
                ("HOME".to_owned(), "/home/test".to_owned())
            };
            Self {
                os: os.to_owned(),
                arch: arch.to_owned(),
                env: BTreeMap::from([home]),
                executables: vec!["git".to_owned()],
                proc_is_wsl: false,
            }
        }
    }

    impl PlatformProbe for FixtureProbe {
        fn raw_os(&self) -> String {
            self.os.clone()
        }
        fn raw_architecture(&self) -> String {
            self.arch.clone()
        }
        fn environment(&self, key: &str) -> Option<String> {
            self.env.get(key).cloned()
        }
        fn home_dir(&self) -> Option<PathBuf> {
            self.env
                .get("HOME")
                .or_else(|| self.env.get("USERPROFILE"))
                .map(PathBuf::from)
        }
        fn file_contains(&self, _: &Path, _: &str) -> bool {
            self.proc_is_wsl
        }
        fn executable_exists(&self, executable: &str) -> bool {
            self.executables
                .iter()
                .any(|candidate| candidate == executable)
        }
        fn path_writable(&self, _: &Path) -> bool {
            true
        }
        fn total_memory_bytes(&self) -> Option<u64> {
            Some(8 * 1024 * 1024 * 1024)
        }
        fn available_disk_bytes(&self, _: &Path) -> Option<u64> {
            Some(64 * 1024 * 1024 * 1024)
        }
        fn os_version(&self) -> Option<String> {
            Some("fixture".to_owned())
        }
        fn elevated(&self) -> bool {
            false
        }
    }

    #[test]
    fn normalizes_tier_one_targets() {
        for (os, arch) in [
            ("macos", "aarch64"),
            ("macos", "x86_64"),
            ("linux", "amd64"),
            ("linux", "arm64"),
            ("windows", "x64"),
        ] {
            let info = detect(&FixtureProbe::new(os, arch)).expect("supported fixture");
            assert_eq!(info.support_tier, SupportTier::Tier1);
            assert!(info.prerequisites.git_present);
            assert!(!info.prerequisites.docker_present);
        }
    }

    #[test]
    fn distinguishes_wsl_from_native_linux() {
        let mut probe = FixtureProbe::new("linux", "x86_64");
        probe.proc_is_wsl = true;
        assert_eq!(detect(&probe).unwrap().environment, HostEnvironment::Wsl);
    }

    #[test]
    fn windows_uses_windows_paths_and_arm_is_tier_two() {
        let mut probe = FixtureProbe::new("windows", "arm64");
        probe
            .env
            .insert("LOCALAPPDATA".to_owned(), r"C:\Users\test\Local".to_owned());
        let info = detect(&probe).unwrap();
        assert_eq!(info.support_tier, SupportTier::Tier2);
        assert!(info.paths.bin.ends_with("OpenWork/bin"));
    }

    #[test]
    fn unsupported_hosts_fail_with_remediation() {
        let error = detect(&FixtureProbe::new("freebsd", "riscv64")).unwrap_err();
        assert!(error.to_string().contains("use macOS"));
    }
}
