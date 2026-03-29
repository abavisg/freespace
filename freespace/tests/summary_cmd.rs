//! Integration tests for `freespace summary` — covers SUMM-01 and SUMM-02.

use assert_cmd::Command;

#[test]
#[cfg(target_os = "macos")]
fn summary_table_exits_0() {
    Command::cargo_bin("freespace")
        .unwrap()
        .arg("summary")
        .assert()
        .success();
}

#[test]
#[cfg(target_os = "macos")]
fn summary_table_has_stdout() {
    let output = Command::cargo_bin("freespace")
        .unwrap()
        .arg("summary")
        .output()
        .unwrap();
    assert!(
        !output.stdout.is_empty(),
        "freespace summary must print table to stdout"
    );
}

#[test]
#[cfg(target_os = "macos")]
fn summary_json_exits_0() {
    Command::cargo_bin("freespace")
        .unwrap()
        .args(["--json", "summary"])
        .assert()
        .success();
}

#[test]
#[cfg(target_os = "macos")]
fn summary_json_is_valid_array() {
    let output = Command::cargo_bin("freespace")
        .unwrap()
        .args(["--json", "summary"])
        .output()
        .unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout);
    let parsed: serde_json::Value =
        serde_json::from_str(stdout.trim()).expect("stdout must be valid JSON");
    assert!(
        parsed.is_array(),
        "JSON output must be an array, got: {parsed}"
    );
}

#[test]
#[cfg(target_os = "macos")]
fn summary_json_stderr_empty() {
    let output = Command::cargo_bin("freespace")
        .unwrap()
        .args(["--json", "summary"])
        .env("RUST_LOG", "off")
        .output()
        .unwrap();
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.is_empty(),
        "stderr must be empty for --json output, got: {stderr}"
    );
}

#[test]
#[cfg(target_os = "macos")]
fn summary_json_has_required_fields() {
    let output = Command::cargo_bin("freespace")
        .unwrap()
        .args(["--json", "summary"])
        .output()
        .unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout);
    let parsed: Vec<serde_json::Value> =
        serde_json::from_str(stdout.trim()).expect("must be a JSON array");
    assert!(!parsed.is_empty(), "JSON array must have at least one volume");
    let first = &parsed[0];
    assert!(first.get("mount_point").is_some(), "missing mount_point field");
    assert!(first.get("total_bytes").is_some(), "missing total_bytes field");
    assert!(first.get("used_bytes").is_some(), "missing used_bytes field");
    assert!(first.get("available_bytes").is_some(), "missing available_bytes field");
}
