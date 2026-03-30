use assert_cmd::Command;
use std::fs;
use tempfile::TempDir;

fn freespace() -> Command {
    Command::cargo_bin("freespace").unwrap()
}

#[test]
fn test_hidden_basic() {
    let dir = TempDir::new().unwrap();
    // Create a hidden file
    fs::write(dir.path().join(".hidden_file"), b"0123456789").unwrap();
    // Create a visible file (should NOT appear)
    fs::write(dir.path().join("visible.txt"), b"visible").unwrap();
    // Create a hidden dir with a file inside
    let hidden_dir = dir.path().join(".hidden_dir");
    fs::create_dir(&hidden_dir).unwrap();
    fs::write(hidden_dir.join("inner.txt"), b"inner").unwrap();

    let output = freespace()
        .args(["hidden", dir.path().to_str().unwrap()])
        .env("RUST_LOG", "off")
        .output()
        .unwrap();

    assert!(output.status.success(), "hidden command must exit 0");
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(
        stdout.contains(".hidden_file"),
        "stdout must contain .hidden_file, got: {stdout}"
    );
    assert!(
        !stdout.contains("visible.txt"),
        "stdout must NOT contain visible.txt, got: {stdout}"
    );
}

#[test]
fn test_hidden_json() {
    let dir = TempDir::new().unwrap();
    fs::write(dir.path().join(".hidden_file"), b"0123456789").unwrap();
    fs::write(dir.path().join("visible.txt"), b"visible").unwrap();
    let hidden_dir = dir.path().join(".hidden_dir");
    fs::create_dir(&hidden_dir).unwrap();
    fs::write(hidden_dir.join("inner.txt"), b"inner content here").unwrap();

    let output = freespace()
        .args(["--json", "hidden", dir.path().to_str().unwrap()])
        .env("RUST_LOG", "off")
        .output()
        .unwrap();

    assert!(output.status.success(), "hidden --json must exit 0");
    let stdout = String::from_utf8(output.stdout).unwrap();
    let parsed: serde_json::Value =
        serde_json::from_str(&stdout).expect("stdout must be valid JSON");

    let entries = parsed["entries"].as_array().expect("entries must be an array");
    assert_eq!(
        entries.len(),
        2,
        "must have 2 hidden entries (.hidden_file and .hidden_dir), got: {}",
        entries.len()
    );

    // Verify each entry has required fields
    for entry in entries {
        assert!(entry.get("path").is_some(), "each entry must have path");
        assert!(
            entry.get("size_bytes").is_some(),
            "each entry must have size_bytes"
        );
        assert!(entry.get("is_dir").is_some(), "each entry must have is_dir");
    }

    let total_hidden_bytes = parsed["total_hidden_bytes"]
        .as_u64()
        .expect("total_hidden_bytes must be a number");
    assert!(
        total_hidden_bytes > 0,
        "total_hidden_bytes must be > 0"
    );

    let total_hidden_count = parsed["total_hidden_count"]
        .as_u64()
        .expect("total_hidden_count must be a number");
    assert_eq!(
        total_hidden_count, 2,
        "total_hidden_count must be 2"
    );
}

#[test]
fn test_hidden_total() {
    let dir = TempDir::new().unwrap();
    fs::write(dir.path().join(".hidden_file"), b"0123456789").unwrap();
    let hidden_dir = dir.path().join(".hidden_dir");
    fs::create_dir(&hidden_dir).unwrap();
    fs::write(hidden_dir.join("inner.txt"), b"inner").unwrap();

    let output = freespace()
        .args(["--json", "hidden", dir.path().to_str().unwrap()])
        .env("RUST_LOG", "off")
        .output()
        .unwrap();

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&stdout).unwrap();

    let entries = parsed["entries"].as_array().unwrap();
    let sum: u64 = entries
        .iter()
        .map(|e| e["size_bytes"].as_u64().unwrap_or(0))
        .sum();

    let total_hidden_bytes = parsed["total_hidden_bytes"].as_u64().unwrap();
    assert_eq!(
        total_hidden_bytes, sum,
        "total_hidden_bytes must equal sum of entry size_bytes: got total={}, sum={}",
        total_hidden_bytes, sum
    );
}

#[test]
fn test_hidden_missing_path() {
    let output = freespace()
        .args(["hidden", "/nonexistent/path/that/does/not/exist"])
        .env("RUST_LOG", "off")
        .output()
        .unwrap();

    assert!(
        !output.status.success(),
        "hidden command must exit non-zero for missing path"
    );
}
