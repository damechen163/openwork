use openwork_execution::artifact::ArtifactScanner;
use openwork_execution::{RelativeArtifactPath, RunId, UtcTimestamp, sha256_bytes};
use std::fs;

#[test]
fn scanner_hashes_regular_output_and_detects_drift() {
    let root = tempfile::tempdir().expect("root");
    fs::create_dir(root.path().join("reports")).expect("reports");
    let output = root.path().join("reports/summary.md");
    fs::write(&output, b"safe report").expect("write");
    let scanner = ArtifactScanner::new(1024).expect("scanner");
    let artifacts = scanner
        .scan(
            &run_id(),
            root.path(),
            &[RelativeArtifactPath::parse("reports/summary.md").expect("path")],
            timestamp(),
        )
        .expect("scan");
    assert_eq!(artifacts.len(), 1);
    assert_eq!(artifacts[0].sha256, sha256_bytes(b"safe report"));
    assert_eq!(artifacts[0].media_type, "text/markdown");
    scanner.verify(root.path(), &artifacts[0]).expect("verify");

    fs::write(output, b"changed report").expect("change");
    assert!(scanner.verify(root.path(), &artifacts[0]).is_err());
}

#[test]
fn scanner_rejects_limits_duplicates_and_special_files() {
    let root = tempfile::tempdir().expect("root");
    fs::write(root.path().join("large.txt"), b"12345").expect("large");
    fs::create_dir(root.path().join("directory.txt")).expect("directory");
    let scanner = ArtifactScanner::new(4).expect("scanner");
    let large = RelativeArtifactPath::parse("large.txt").expect("path");
    assert!(
        scanner
            .scan(
                &run_id(),
                root.path(),
                std::slice::from_ref(&large),
                timestamp()
            )
            .is_err()
    );
    assert!(
        scanner
            .scan(&run_id(), root.path(), &[large.clone(), large], timestamp())
            .is_err()
    );
    let directory = RelativeArtifactPath::parse("directory.txt").expect("path");
    assert!(
        scanner
            .scan(&run_id(), root.path(), &[directory], timestamp())
            .is_err()
    );
}

#[cfg(unix)]
#[test]
fn scanner_rejects_symlink_escape() {
    use std::os::unix::fs::symlink;
    let root = tempfile::tempdir().expect("root");
    let outside = tempfile::NamedTempFile::new().expect("outside");
    symlink(outside.path(), root.path().join("escape.txt")).expect("symlink");
    let scanner = ArtifactScanner::new(1024).expect("scanner");
    let escape = RelativeArtifactPath::parse("escape.txt").expect("path");
    assert!(
        scanner
            .scan(&run_id(), root.path(), &[escape], timestamp())
            .is_err()
    );
}

fn run_id() -> RunId {
    RunId::parse("01890f3e-a5f1-7cc2-98c0-5f9c6f5e7a01").expect("UUIDv7")
}

fn timestamp() -> UtcTimestamp {
    UtcTimestamp::parse("2026-08-10T00:00:00Z").expect("timestamp")
}
