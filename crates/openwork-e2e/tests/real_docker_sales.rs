//! Opt-in real-Docker proof for the reusable sales demo runner.
//!
//! This test is ignored by default. When explicitly selected it also requires
//! `OPENWORK_REAL_DOCKER_TESTS=1`, `OPENWORK_DOCKER_BIN` (an absolute path),
//! and `OPENWORK_SALES_SANDBOX_IMAGE` (a digest-pinned BusyBox-compatible image).

use openwork_core::ErrorCode;
use openwork_e2e::sales_demo::{SalesDemoConfig, run_sales_demo};
use openwork_execution::{AuditEventType, DigestPinnedImageRef, RunStatus};
use std::env;
use std::path::PathBuf;

#[test]
#[ignore = "requires an explicitly configured Docker daemon and digest-pinned image"]
fn real_docker_sales_analyzer_matches_exact_golden() {
    assert_eq!(
        env::var("OPENWORK_REAL_DOCKER_TESTS").as_deref(),
        Ok("1"),
        "set OPENWORK_REAL_DOCKER_TESTS=1 to run the ignored real Docker proof"
    );
    let output_root = tempfile::tempdir().expect("retained output root for assertions");
    let report = run_sales_demo(SalesDemoConfig {
        engine_bin: PathBuf::from(required_string("OPENWORK_DOCKER_BIN")),
        image: DigestPinnedImageRef::parse(required_string("OPENWORK_SALES_SANDBOX_IMAGE"))
            .expect("OPENWORK_SALES_SANDBOX_IMAGE must be pinned by a sha256 digest"),
        output_root: Some(output_root.path().to_path_buf()),
    })
    .expect("real Docker sales demo");

    assert_eq!(report.status, RunStatus::Succeeded);
    assert_eq!(report.revision, 3);
    assert_eq!(report.artifacts.len(), 2);
    assert!(report.output_directory.is_some());
    assert!(
        report
            .audit_events
            .iter()
            .any(|event| { matches!(event.event_type, AuditEventType::ArtifactCreated) })
    );
    assert!(matches!(
        report.audit_events.last().map(|event| event.event_type),
        Some(AuditEventType::RunCompleted)
    ));
    for (index, event) in report.audit_events.iter().enumerate() {
        assert_eq!(event.sequence, u64::try_from(index + 1).expect("sequence"));
    }
}

#[test]
fn real_docker_configuration_rejects_a_floating_image_tag() {
    assert!(DigestPinnedImageRef::parse("docker.io/library/busybox:1.37").is_err());
}

#[test]
fn reusable_runner_rejects_a_relative_engine_path_before_execution() {
    let error = run_sales_demo(SalesDemoConfig {
        engine_bin: PathBuf::from("docker"),
        image: DigestPinnedImageRef::parse(format!(
            "docker.io/library/busybox@sha256:{}",
            "a".repeat(64)
        ))
        .expect("syntactically pinned image"),
        output_root: None,
    })
    .expect_err("relative engine path must fail closed");
    assert_eq!(error.code, ErrorCode::InvalidArguments);
}

fn required_string(name: &str) -> String {
    env::var(name)
        .unwrap_or_else(|_| panic!("{name} is required when real Docker tests are enabled"))
}
