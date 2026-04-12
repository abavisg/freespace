use assert_cmd::Command;
use std::fs;
use tempfile::TempDir;

fn freespace() -> Command {
    Command::cargo_bin("freespace").unwrap()
}

#[test]
fn test_scan_basic() {
    let dir = TempDir::new().unwrap();
    fs::write(dir.path().join("a.txt"), b"hello").unwrap();
    fs::write(dir.path().join("b.txt"), b"world").unwrap();
    let output = freespace()
        .args(["scan", dir.path().to_str().unwrap()])
        .env("RUST_LOG", "off")
        .output()
        .unwrap();
    assert!(output.status.success(), "scan must exit 0");
    let stdout = String::from_utf8(output.stdout).unwrap();
    // Table output must contain file count 2 and the path
    assert!(stdout.contains('2'), "file count 2 must appear in output");
}

#[test]
fn test_scan_json() {
    let dir = TempDir::new().unwrap();
    fs::write(dir.path().join("file.dat"), b"data").unwrap();
    let output = freespace()
        .args(["--json", "scan", dir.path().to_str().unwrap()])
        .env("RUST_LOG", "off")
        .output()
        .unwrap();
    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&stdout)
        .expect("stdout must be valid JSON");
    assert!(parsed.get("total_bytes").is_some());
    assert!(parsed.get("file_count").is_some());
    assert!(parsed.get("dir_count").is_some());
    assert!(parsed.get("skipped_count").is_some());
}

#[test]
fn test_scan_hardlink_dedup() {
    let dir = TempDir::new().unwrap();
    let original = dir.path().join("original.dat");
    fs::write(&original, b"hardlink test content").unwrap();
    let link = dir.path().join("hardlink.dat");
    fs::hard_link(&original, &link).unwrap();
    let output = freespace()
        .args(["--json", "scan", dir.path().to_str().unwrap()])
        .env("RUST_LOG", "off")
        .output()
        .unwrap();
    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    let file_count = parsed["file_count"].as_u64().unwrap();
    assert_eq!(file_count, 1, "hardlinked files must be counted once, got {}", file_count);
}

#[test]
fn test_scan_permission_error() {
    use std::os::unix::fs::PermissionsExt;
    let dir = TempDir::new().unwrap();
    let subdir = dir.path().join("restricted");
    fs::create_dir(&subdir).unwrap();
    fs::write(subdir.join("secret.txt"), b"secret").unwrap();
    // Make subdir unreadable — simulates TCC/EPERM
    fs::set_permissions(&subdir, fs::Permissions::from_mode(0o000)).unwrap();
    let output = freespace()
        .args(["--json", "scan", dir.path().to_str().unwrap()])
        .env("RUST_LOG", "off")
        .output()
        .unwrap();
    // Restore permissions so TempDir cleanup succeeds
    fs::set_permissions(&subdir, fs::Permissions::from_mode(0o755)).unwrap();
    assert!(output.status.success(), "scan must not crash on permission error");
    let stdout = String::from_utf8(output.stdout).unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    let skipped = parsed["skipped_count"].as_u64().unwrap();
    assert!(skipped >= 1, "skipped_count must be >= 1 when a subdir is inaccessible");
}

#[test]
fn test_scan_missing_path() {
    let output = freespace()
        .args(["scan", "/nonexistent/path/that/does/not/exist"])
        .env("RUST_LOG", "off")
        .output()
        .unwrap();
    assert!(!output.status.success(), "scan of nonexistent path must exit non-zero");
}
