use assert_cmd::Command;

fn freespace() -> Command {
    Command::cargo_bin("freespace").unwrap()
}

#[test]
fn test_caches_exits_ok() {
    // Must not crash even if some cache dirs don't exist
    let output = freespace()
        .args(["caches"])
        .env("RUST_LOG", "off")
        .output()
        .unwrap();

    assert!(
        output.status.success(),
        "caches command must exit 0, stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn test_caches_json_fields() {
    let output = freespace()
        .args(["--json", "caches"])
        .env("RUST_LOG", "off")
        .output()
        .unwrap();

    assert!(output.status.success(), "caches --json must exit 0");
    let stdout = String::from_utf8(output.stdout).unwrap();
    let parsed: serde_json::Value =
        serde_json::from_str(&stdout).expect("stdout must be valid JSON");

    assert!(
        parsed.get("entries").is_some(),
        "JSON must have entries field"
    );
    assert!(
        parsed["entries"].is_array(),
        "entries must be an array"
    );
    assert!(
        parsed.get("total_cache_bytes").is_some(),
        "JSON must have total_cache_bytes"
    );
    assert!(
        parsed["total_cache_bytes"].is_number(),
        "total_cache_bytes must be a number"
    );
    assert!(
        parsed.get("reclaimable_bytes").is_some(),
        "JSON must have reclaimable_bytes"
    );
    assert!(
        parsed["reclaimable_bytes"].is_number(),
        "reclaimable_bytes must be a number"
    );

    // Each entry must have the required fields
    for entry in parsed["entries"].as_array().unwrap() {
        assert!(entry.get("path").is_some(), "entry must have path");
        assert!(entry.get("total_bytes").is_some(), "entry must have total_bytes");
        assert!(entry.get("file_count").is_some(), "entry must have file_count");
        assert!(entry.get("safety").is_some(), "entry must have safety");
    }
}

#[test]
fn test_caches_reclaimable() {
    let output = freespace()
        .args(["--json", "caches"])
        .env("RUST_LOG", "off")
        .output()
        .unwrap();

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&stdout).unwrap();

    let reclaimable = parsed["reclaimable_bytes"]
        .as_u64()
        .expect("reclaimable_bytes must be a number");
    // Must be a non-negative number (u64 is always >= 0)
    let _ = reclaimable;
    // Additional: reclaimable must be <= total_cache_bytes
    let total = parsed["total_cache_bytes"].as_u64().unwrap_or(0);
    assert!(
        reclaimable <= total,
        "reclaimable_bytes ({}) must be <= total_cache_bytes ({})",
        reclaimable,
        total
    );
}

#[test]
fn test_caches_safety_values() {
    let output = freespace()
        .args(["--json", "caches"])
        .env("RUST_LOG", "off")
        .output()
        .unwrap();

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&stdout).unwrap();

    let valid_safety = ["safe", "caution", "dangerous", "blocked"];
    for entry in parsed["entries"].as_array().unwrap() {
        let safety = entry["safety"].as_str().expect("safety must be a string");
        assert!(
            valid_safety.contains(&safety),
            "safety value '{}' must be one of {:?}",
            safety,
            valid_safety
        );
    }
}
