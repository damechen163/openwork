use clap::{Parser, Subcommand};
use openwork_core::{ErrorCode, OpenWorkError, PRODUCT_NAME};
use openwork_doctor::{CheckStatus, DoctorReport, inspect_platform};
use openwork_installer::{InstallPlan, dry_run_plan};
use openwork_platform::{PlatformInfo, PlatformProbe, SystemPlatformProbe, detect};
use openwork_runtime::{
    AgentRuntime, AuthStatus, ClaudeRuntime, DetectionState, RuntimeCapabilities, RuntimeId,
    RuntimeMetadata, RuntimeRegistry, SystemCommandRunner,
};
use serde::Serialize;
use std::process::ExitCode;
use std::sync::Arc;

#[derive(Debug, Parser)]
#[command(
    name = "openwork",
    about = "Cross-platform Bootstrap runtime for OpenWork",
    disable_version_flag = true
)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
    /// Preview a Bootstrap installation without changing the host.
    Install {
        #[arg(long, required = true)]
        dry_run: bool,
        #[arg(long)]
        json: bool,
    },
    /// Show current host and Bootstrap state.
    Status {
        #[arg(long)]
        json: bool,
    },
    /// Run structured host diagnostics.
    Doctor {
        #[arg(long)]
        json: bool,
    },
    /// Inspect registered agent runtimes.
    Runtime {
        #[command(subcommand)]
        command: RuntimeCommand,
    },
}

#[derive(Debug, Subcommand)]
enum RuntimeCommand {
    /// List registered runtimes.
    List {
        #[arg(long)]
        json: bool,
    },
    /// Show one runtime.
    Info {
        id: String,
        #[arg(long)]
        json: bool,
    },
}

#[derive(Serialize)]
struct StatusReport {
    schema_version: u32,
    state: &'static str,
    platform: PlatformInfo,
    runtimes: Vec<RuntimeSummary>,
}

#[derive(Serialize)]
struct RuntimeSummary {
    metadata: RuntimeMetadata,
    detection: DetectionState,
    version: Option<String>,
    auth: AuthStatus,
    capabilities: RuntimeCapabilities,
}

fn main() -> ExitCode {
    let arguments: Vec<String> = std::env::args().collect();
    if arguments.len() == 2 && matches!(arguments[1].as_str(), "--version" | "-V") {
        println!("{PRODUCT_NAME} {}", env!("CARGO_PKG_VERSION"));
        return ExitCode::SUCCESS;
    }

    match Cli::try_parse_from(&arguments) {
        Ok(cli) => run(cli, &SystemPlatformProbe),
        Err(error) => {
            let code = if error.use_stderr() { 2 } else { 0 };
            let _ = error.print();
            ExitCode::from(code)
        }
    }
}

fn run(cli: Cli, probe: &impl PlatformProbe) -> ExitCode {
    match execute(cli, probe) {
        Ok(code) => ExitCode::from(code),
        Err((error, json)) => {
            if json {
                println!(
                    "{}",
                    serde_json::to_string_pretty(&error).unwrap_or_default()
                );
            } else {
                eprintln!("error[{:?}]: {error}", error.code);
            }
            ExitCode::from(error.exit_code())
        }
    }
}

fn execute(cli: Cli, probe: &impl PlatformProbe) -> Result<u8, (OpenWorkError, bool)> {
    match cli.command {
        Command::Install { dry_run: _, json } => {
            let host = platform(probe, json)?;
            render_install(&dry_run_plan(&host), json);
            Ok(0)
        }
        Command::Status { json } => {
            let host = platform(probe, json)?;
            let runtimes = runtime_summaries(&runtime_registry(&host), json)?;
            let report = StatusReport {
                schema_version: 1,
                state: "not_installed",
                platform: host,
                runtimes,
            };
            render_status(&report, json);
            Ok(0)
        }
        Command::Doctor { json } => {
            let report = inspect_platform(&platform(probe, json)?);
            render_doctor(&report, json);
            Ok(if report.has_failures() {
                ErrorCode::PreflightFailed.exit_code()
            } else {
                0
            })
        }
        Command::Runtime {
            command: RuntimeCommand::List { json },
        } => {
            let host = platform(probe, json)?;
            let runtimes = runtime_summaries(&runtime_registry(&host), json)?;
            if json {
                println!(
                    "{}",
                    serde_json::to_string_pretty(&runtimes).unwrap_or_default()
                );
            } else {
                for runtime in &runtimes {
                    println!(
                        "{}\t{:?}\t{}",
                        runtime.metadata.id,
                        runtime.detection,
                        runtime.version.as_deref().unwrap_or("unknown")
                    );
                }
            }
            Ok(0)
        }
        Command::Runtime {
            command: RuntimeCommand::Info { id, json },
        } => {
            let host = platform(probe, json)?;
            let registry = runtime_registry(&host);
            let runtime = registry.get(&RuntimeId::from(id.as_str())).ok_or_else(|| {
                (
                    OpenWorkError::new(
                        ErrorCode::RuntimeNotFound,
                        format!("runtime `{id}` is not registered"),
                    )
                    .with_remediation("Run `openwork runtime list` to see available runtimes."),
                    json,
                )
            })?;
            let summary = runtime_summary(runtime.as_ref()).map_err(|error| (error, json))?;
            if json {
                println!(
                    "{}",
                    serde_json::to_string_pretty(&summary).unwrap_or_default()
                );
            } else {
                println!("Runtime: {}", summary.metadata.name);
                println!("State: {:?}", summary.detection);
                println!(
                    "Version: {}",
                    summary.version.as_deref().unwrap_or("unknown")
                );
                println!("Auth: {:?}", summary.auth);
                println!("Distribution: {:?}", summary.metadata.distribution);
            }
            Ok(0)
        }
    }
}

fn runtime_registry(host: &PlatformInfo) -> RuntimeRegistry {
    let mut registry = RuntimeRegistry::new();
    registry
        .register(Arc::new(ClaudeRuntime::new(
            Arc::new(SystemCommandRunner),
            None,
            host.clone(),
        )))
        .expect("built-in runtime ids are unique");
    registry
}

fn runtime_summaries(
    registry: &RuntimeRegistry,
    json: bool,
) -> Result<Vec<RuntimeSummary>, (OpenWorkError, bool)> {
    registry
        .metadata()
        .into_iter()
        .map(|metadata| {
            registry
                .get(&metadata.id)
                .expect("metadata came from this registry")
        })
        .map(|runtime| runtime_summary(runtime.as_ref()).map_err(|error| (error, json)))
        .collect()
}

fn runtime_summary(runtime: &dyn AgentRuntime) -> Result<RuntimeSummary, OpenWorkError> {
    Ok(RuntimeSummary {
        metadata: runtime.metadata(),
        detection: runtime.detect()?.state,
        version: runtime.version()?,
        auth: runtime.auth_status()?,
        capabilities: runtime.capabilities(),
    })
}

fn platform(probe: &impl PlatformProbe, json: bool) -> Result<PlatformInfo, (OpenWorkError, bool)> {
    detect(probe).map_err(|error| {
        (
            OpenWorkError::new(ErrorCode::UnsupportedPlatform, error.to_string())
                .with_remediation("Use a documented Tier 1 host and rerun the command."),
            json,
        )
    })
}

fn render_install(plan: &InstallPlan, json: bool) {
    if json {
        println!("{}", serde_json::to_string_pretty(plan).unwrap_or_default());
        return;
    }
    println!("OpenWork installation plan (dry-run)");
    for step in &plan.steps {
        println!("- {}: {}", step.id, step.path.display());
    }
    for warning in &plan.warnings {
        println!("warning: {warning}");
    }
}

fn render_status(report: &StatusReport, json: bool) {
    if json {
        println!(
            "{}",
            serde_json::to_string_pretty(report).unwrap_or_default()
        );
    } else {
        println!("OpenWork state: {}", report.state);
        println!(
            "Host: {:?} {:?}",
            report.platform.os, report.platform.architecture
        );
        println!("Runtimes: {}", report.runtimes.len());
    }
}

fn render_doctor(report: &DoctorReport, json: bool) {
    if json {
        println!(
            "{}",
            serde_json::to_string_pretty(report).unwrap_or_default()
        );
        return;
    }
    println!("OpenWork Doctor");
    for check in &report.checks {
        let marker = match check.status {
            CheckStatus::Pass => "PASS",
            CheckStatus::Warn => "WARN",
            CheckStatus::Fail => "FAIL",
            CheckStatus::Skip => "SKIP",
        };
        println!("[{marker}] {} — {}", check.id, check.summary);
        if let Some(remediation) = &check.remediation {
            println!("       remediation: {remediation}");
        }
    }
    println!(
        "Summary: {} pass, {} warn, {} fail, {} skip",
        report.summary.pass, report.summary.warn, report.summary.fail, report.summary.skip
    );
}
