//! Integration tests for `freespace doctor` and `freespace completions`
//! Covers DIAG-01 (self-diagnostics: TCC, protected paths, config) and
//! DIAG-02 (actionable remediation in output).
//!
//! Wave 0: doctor_* tests initially FAIL (RED) against the Plan 01 stub.
//! Plan 02 turns them GREEN by implementing doctor.rs.

use assert_cmd::Command;
use serde_json::Value;

fn freespace() -> Command {
    Command::cargo_bin("freespace").expect("freespace binary must build")
}

// ---------- DIAG-01: exit codes ----------

#[test]
fn doctor_exits_0_all_pass() {
    // When all checks pass (or only warnings), exit code is 0.
    // On a machine with FDA granted this should exit 0; on a machine without
    // FDA it may exit 1. We therefore accept either success or a specific
    // failure mode. The canonical assertion is: if doctor reports no hard
    // failures in JSON, exit code is 0.
    let output = freespace()
        .args(["doctor", "--json"])
        .output()
        .expect("doctor must execute");
    let stdout = String::from_utf8_lossy(&output.stdout);
    let json: Value = serde_json::from_str(&stdout)
        .expect("doctor --json must emit valid JSON on stdout");
    let overall = json.get("overall").and_then(|v| v.as_str())
        .expect("JSON must have top-level `overall` field of type string");
    if overall == "pass" || overall == "warn" {
        assert!(
            output.status.success(),
            "when overall is pass/warn, exit code must be 0 (was {:?})",
            output.status.code()
        );
    }
}

#[test]
fn doctor_exits_1_on_failure() {
    // When overall == "fail", exit code is non-zero.
    let output = freespace()
        .args(["doctor", "--json"])
        .output()
        .expect("doctor must execute");
    let stdout = String::from_utf8_lossy(&output.stdout);
    let json: Value = serde_json::from_str(&stdout)
        .expect("doctor --json must emit valid JSON on stdout");
    let overall = json.get("overall").and_then(|v| v.as_str())
        .expect("JSON must have top-level `overall` field");
    if overall == "fail" {
        assert!(
            !output.status.success(),
            "when overall is fail, exit code must be non-zero"
        );
        assert_eq!(
            output.status.code(),
            Some(1),
            "failure exit code must be exactly 1"
        );
    }
}

// ---------- DIAG-01: JSON structure ----------

#[test]
fn doctor_json_structure() {
    // JSON contains a `checks` array.
    let output = freespace()
        .args(["doctor", "--json"])
        .output()
        .expect("doctor must execute");
    let stdout = String::from_utf8_lossy(&output.stdout);
    let json: Value = serde_json::from_str(&stdout)
        .expect("doctor --json must emit valid JSON");
    let checks = json.get("checks")
        .expect("JSON must have top-level `checks` field");
    let arr = checks.as_array()
        .expect("`checks` must be an array");
    assert!(!arr.is_empty(), "`checks` array must not be empty");
    for (i, check) in arr.iter().enumerate() {
        assert!(
            check.get("name").and_then(|v| v.as_str()).is_some(),
            "check[{i}] must have `name` string"
        );
        assert!(
            check.get("status").and_then(|v| v.as_str()).is_some(),
            "check[{i}] must have `status` string"
        );
        assert!(
            check.get("message").and_then(|v| v.as_str()).is_some(),
            "check[{i}] must have `message` string"
        );
        let status = check["status"].as_str().unwrap();
        assert!(
            matches!(status, "pass" | "fail" | "warn"),
            "check[{i}].status must be one of pass/fail/warn (was {status})"
        );
    }
}

#[test]
fn doctor_json_overall_field() {
    // JSON contains a top-level `overall` field with a valid value.
    let output = freespace()
        .args(["doctor", "--json"])
        .output()
        .expect("doctor must execute");
    let stdout = String::from_utf8_lossy(&output.stdout);
    let json: Value = serde_json::from_str(&stdout)
        .expect("doctor --json must emit valid JSON");
    let overall = json.get("overall").and_then(|v| v.as_str())
        .expect("JSON must have top-level `overall` string field");
    assert!(
        matches!(overall, "pass" | "fail" | "warn"),
        "`overall` must be one of pass/fail/warn (was {overall})"
    );
}

// ---------- DIAG-01: checks present ----------

#[test]
fn doctor_includes_required_checks() {
    // DIAG-01 requires TCC/FDA, protected paths, and config file checks.
    let output = freespace()
        .args(["doctor", "--json"])
        .output()
        .expect("doctor must execute");
    let stdout = String::from_utf8_lossy(&output.stdout);
    let json: Value = serde_json::from_str(&stdout)
        .expect("doctor --json must emit valid JSON");
    let names: Vec<String> = json["checks"]
        .as_array()
        .unwrap()
        .iter()
        .map(|c| c["name"].as_str().unwrap().to_string())
        .collect();
    assert!(
        names.iter().any(|n| n.contains("Full Disk Access")),
        "checks must include a 'Full Disk Access' entry, got: {names:?}"
    );
    assert!(
        names.iter().any(|n| n.contains("Protected paths")),
        "checks must include a 'Protected paths' entry, got: {names:?}"
    );
    assert!(
        names.iter().any(|n| n.contains("Config file")),
        "checks must include a 'Config file' entry, got: {names:?}"
    );
}

// ---------- DIAG-02: remediation text ----------

#[test]
fn doctor_remediation_message() {
    // DIAG-02: Every non-pass check must have a non-empty, actionable message.
    // "Actionable" is operationalized as: message length >= 10 chars (not just "error"/"fail").
    let output = freespace()
        .args(["doctor", "--json"])
        .output()
        .expect("doctor must execute");
    let stdout = String::from_utf8_lossy(&output.stdout);
    let json: Value = serde_json::from_str(&stdout)
        .expect("doctor --json must emit valid JSON");
    for check in json["checks"].as_array().unwrap() {
        let status = check["status"].as_str().unwrap();
        let message = check["message"].as_str().unwrap();
        if status == "fail" || status == "warn" {
            assert!(
                message.len() >= 10,
                "non-pass check '{}' must have actionable message (>=10 chars), got: {:?}",
                check["name"].as_str().unwrap(),
                message
            );
        }
        // Every check has a message (even pass).
        assert!(
            !message.is_empty(),
            "check message must never be empty for '{}'",
            check["name"].as_str().unwrap()
        );
    }
}

// ---------- DIAG-01: completions subcommand (Plan 01 GREEN) ----------

#[test]
fn completions_zsh_exits_0() {
    let output = freespace()
        .args(["completions", "zsh"])
        .output()
        .expect("completions zsh must execute");
    assert!(output.status.success(), "completions zsh must exit 0");
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.len() > 100, "zsh completion script must be non-trivial");
    assert!(stdout.contains("freespace"), "script must reference binary name");
}

#[test]
fn completions_bash_exits_0() {
    let output = freespace()
        .args(["completions", "bash"])
        .output()
        .expect("completions bash must execute");
    assert!(output.status.success(), "completions bash must exit 0");
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.len() > 100, "bash completion script must be non-trivial");
    assert!(stdout.contains("freespace"), "script must reference binary name");
}

#[test]
fn completions_fish_exits_0() {
    let output = freespace()
        .args(["completions", "fish"])
        .output()
        .expect("completions fish must execute");
    assert!(output.status.success(), "completions fish must exit 0");
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.len() > 100, "fish completion script must be non-trivial");
    assert!(stdout.contains("freespace"), "script must reference binary name");
}
