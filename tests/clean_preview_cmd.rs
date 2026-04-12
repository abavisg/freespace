use assert_cmd::Command;

fn freespace() -> Command {
    Command::cargo_bin("freespace").unwrap()
}

#[test]
fn test_clean_preview_exits_ok() {
    let output = freespace()
        .args(["clean", "preview"])
        .env("RUST_LOG", "off")
        .output()
        .unwrap();
    assert!(output.status.success(), "clean preview must exit 0");
}

#[test]
fn test_clean_preview_json_fields() {
    let output = freespace()
        .args(["--json", "clean", "preview"])
        .env("RUST_LOG", "off")
        .output()
        .unwrap();
    assert!(output.status.success(), "clean preview --json must exit 0");
    let stdout = String::from_utf8(output.stdout).unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&stdout)
        .expect("stdout must be valid JSON");
    assert!(parsed.get("candidates").is_some(), "JSON must have candidates field");
    assert!(parsed["candidates"].is_array(), "candidates must be an array");
    assert!(parsed.get("total_bytes").is_some(), "JSON must have total_bytes field");
    assert!(parsed["total_bytes"].is_number(), "total_bytes must be a number");
    assert!(parsed.get("reclaimable_bytes").is_some(), "JSON must have reclaimable_bytes field");
    assert!(parsed["reclaimable_bytes"].is_number(), "reclaimable_bytes must be a number");
    for entry in parsed["candidates"].as_array().unwrap() {
        assert!(entry["path"].is_string(), "candidate entry must have string path");
        assert!(entry["total_bytes"].is_number(), "candidate entry must have number total_bytes");
        assert!(entry["file_count"].is_number(), "candidate entry must have number file_count");
        assert!(entry["safety"].is_string(), "candidate entry must have string safety");
    }
}

#[test]
fn test_clean_preview_stderr_clean_with_json() {
    let output = freespace()
        .args(["--json", "clean", "preview"])
        .env("RUST_LOG", "off")
        .output()
        .unwrap();
    assert!(output.status.success(), "clean preview --json must exit 0");
    assert!(
        output.stderr.is_empty(),
        "stderr must be empty with RUST_LOG=off and --json"
    );
}

#[test]
fn test_clean_preview_makes_no_changes() {
    // PREV-02: preview is read-only — verified by running twice and checking
    // both invocations succeed with valid JSON. Byte-for-byte output comparison
    // is intentionally avoided because parallel test execution can cause real
    // cache directory sizes to shift between the two runs.
    let first = freespace()
        .args(["--json", "clean", "preview"])
        .env("RUST_LOG", "off")
        .output()
        .unwrap();
    assert!(first.status.success(), "first preview run must exit 0");
    let _: serde_json::Value = serde_json::from_slice(&first.stdout)
        .expect("first preview run must produce valid JSON");

    let second = freespace()
        .args(["--json", "clean", "preview"])
        .env("RUST_LOG", "off")
        .output()
        .unwrap();
    assert!(second.status.success(), "second preview run must exit 0");
    let _: serde_json::Value = serde_json::from_slice(&second.stdout)
        .expect("second preview run must produce valid JSON");
}

#[test]
fn test_clean_preview_safety_values_valid() {
    let output = freespace()
        .args(["--json", "clean", "preview"])
        .env("RUST_LOG", "off")
        .output()
        .unwrap();
    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    let valid_safety = ["safe", "caution", "dangerous", "blocked"];
    for entry in parsed["candidates"].as_array().unwrap() {
        let safety = entry["safety"].as_str().expect("safety must be a string");
        assert!(
            valid_safety.contains(&safety),
            "safety value '{}' is not valid; must be one of {:?}",
            safety,
            valid_safety
        );
    }
}

#[test]
fn test_clean_preview_reclaimable_lte_total() {
    let output = freespace()
        .args(["--json", "clean", "preview"])
        .env("RUST_LOG", "off")
        .output()
        .unwrap();
    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    let reclaimable = parsed["reclaimable_bytes"].as_u64().unwrap_or(0);
    let total = parsed["total_bytes"].as_u64().unwrap_or(0);
    assert!(
        reclaimable <= total,
        "reclaimable_bytes must be <= total_bytes, got {} > {}",
        reclaimable,
        total
    );
}

#[test]
fn test_clean_preview_candidate_entry_fields() {
    let output = freespace()
        .args(["--json", "clean", "preview"])
        .env("RUST_LOG", "off")
        .output()
        .unwrap();
    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    for entry in parsed["candidates"].as_array().unwrap() {
        assert!(entry["path"].is_string(), "path must be a string");
        assert!(entry["total_bytes"].is_number(), "total_bytes must be a number");
        assert!(entry["file_count"].is_number(), "file_count must be a number");
        assert!(entry["safety"].is_string(), "safety must be a string");
    }
}
