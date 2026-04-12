use assert_cmd::Command;
use std::fs;
use tempfile::TempDir;

fn freespace() -> Command {
    Command::cargo_bin("freespace").unwrap()
}

#[test]
fn test_largest_basic() {
    let dir = TempDir::new().unwrap();
    fs::write(dir.path().join("big.dat"), vec![0u8; 8192]).unwrap();
    fs::write(dir.path().join("small.dat"), b"tiny").unwrap();
    let output = freespace()
        .args(["largest", dir.path().to_str().unwrap()])
        .env("RUST_LOG", "off")
        .output()
        .unwrap();
    assert!(output.status.success(), "largest must exit 0");
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(!stdout.is_empty(), "largest must produce output");
    assert!(stdout.contains("Largest"), "output must contain 'Largest' header");
}

#[test]
fn test_largest_json() {
    let dir = TempDir::new().unwrap();
    fs::write(dir.path().join("file.dat"), vec![0u8; 4096]).unwrap();
    let output = freespace()
        .args(["--json", "largest", dir.path().to_str().unwrap()])
        .env("RUST_LOG", "off")
        .output()
        .unwrap();
    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    let parsed: serde_json::Value =
        serde_json::from_str(&stdout).expect("stdout must be valid JSON");
    assert!(
        parsed.get("largest_files").is_some(),
        "JSON must have largest_files"
    );
    assert!(
        parsed.get("largest_dirs").is_some(),
        "JSON must have largest_dirs"
    );
    assert!(
        parsed.get("total_bytes").is_some(),
        "JSON must have total_bytes"
    );
}

#[test]
fn test_largest_stderr_clean_with_json() {
    let dir = TempDir::new().unwrap();
    fs::write(dir.path().join("x.dat"), b"data").unwrap();
    let output = freespace()
        .args(["--json", "largest", dir.path().to_str().unwrap()])
        .env("RUST_LOG", "off")
        .output()
        .unwrap();
    assert!(output.status.success());
    assert!(
        output.stderr.is_empty(),
        "stderr must be empty with RUST_LOG=off and --json"
    );
}

#[test]
fn test_largest_missing_path() {
    let output = freespace()
        .args(["largest", "/nonexistent/path"])
        .env("RUST_LOG", "off")
        .output()
        .unwrap();
    assert!(
        !output.status.success(),
        "must exit non-zero for missing path"
    );
}

#[test]
fn test_largest_ordering() {
    let dir = TempDir::new().unwrap();
    fs::write(dir.path().join("big.dat"), vec![0u8; 8192]).unwrap();
    fs::write(dir.path().join("small.dat"), vec![0u8; 512]).unwrap();
    let output = freespace()
        .args(["--json", "largest", dir.path().to_str().unwrap()])
        .env("RUST_LOG", "off")
        .output()
        .unwrap();
    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    let files = parsed["largest_files"].as_array().unwrap();
    assert!(files.len() >= 2, "must have at least 2 files");
    let first_size = files[0]["size"].as_u64().unwrap();
    let second_size = files[1]["size"].as_u64().unwrap();
    assert!(
        first_size >= second_size,
        "largest_files must be sorted descending: {} < {}",
        first_size,
        second_size
    );
}
