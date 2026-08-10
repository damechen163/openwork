use std::fs;
use std::process::Command;

fn openwork() -> Command {
    Command::new(env!("CARGO_BIN_EXE_openwork"))
}

#[test]
fn version_is_stable() {
    let output = openwork().arg("--version").output().unwrap();
    assert!(output.status.success());
    assert_eq!(
        String::from_utf8(output.stdout).unwrap(),
        "OpenWork 0.1.0\n"
    );
}

#[test]
fn doctor_json_is_structured() {
    let output = openwork().args(["doctor", "--json"]).output().unwrap();
    let value: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(value["schema_version"], 1);
    assert!(
        value["checks"]
            .as_array()
            .is_some_and(|checks| !checks.is_empty())
    );
}

#[test]
fn install_dry_run_has_no_filesystem_side_effects() {
    let home = tempfile::tempdir().unwrap();
    let output = openwork()
        .args(["install", "--dry-run", "--json"])
        .env("HOME", home.path())
        .output()
        .unwrap();
    assert!(output.status.success());
    let value: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(value["dry_run"], true);
    assert_eq!(fs::read_dir(home.path()).unwrap().count(), 0);
}

#[test]
fn runtime_commands_have_stable_empty_and_error_states() {
    let list = openwork()
        .args(["runtime", "list", "--json"])
        .output()
        .unwrap();
    assert!(list.status.success());
    assert_eq!(
        serde_json::from_slice::<serde_json::Value>(&list.stdout).unwrap(),
        serde_json::json!([])
    );

    let info = openwork()
        .args(["runtime", "info", "missing", "--json"])
        .output()
        .unwrap();
    assert_eq!(info.status.code(), Some(20));
    let error: serde_json::Value = serde_json::from_slice(&info.stdout).unwrap();
    assert_eq!(error["code"], "runtime_not_found");
}
