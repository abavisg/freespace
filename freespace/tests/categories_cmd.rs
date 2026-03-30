use assert_cmd::Command;
use std::fs;
use tempfile::TempDir;

fn freespace() -> Command {
    Command::cargo_bin("freespace").unwrap()
}

#[test]
fn test_categories_basic() {
    let dir = TempDir::new().unwrap();
    // Create files with various extensions
    fs::write(dir.path().join("test.mp4"), b"video content").unwrap();
    fs::write(dir.path().join("doc.pdf"), b"pdf content").unwrap();
    fs::write(dir.path().join("photo.jpg"), b"image content").unwrap();
    fs::write(dir.path().join("unknown.xyz"), b"unknown content").unwrap();

    let output = freespace()
        .args(["categories", dir.path().to_str().unwrap()])
        .env("RUST_LOG", "off")
        .output()
        .unwrap();

    assert!(output.status.success(), "categories must exit 0");
    let stdout = String::from_utf8(output.stdout).unwrap();
    // Table output must contain category names
    assert!(
        stdout.contains("video"),
        "stdout must contain 'video', got: {}",
        stdout
    );
    assert!(
        stdout.contains("documents"),
        "stdout must contain 'documents', got: {}",
        stdout
    );
    assert!(
        stdout.contains("images"),
        "stdout must contain 'images', got: {}",
        stdout
    );
    assert!(
        stdout.contains("unknown"),
        "stdout must contain 'unknown', got: {}",
        stdout
    );
}

#[test]
fn test_categories_json() {
    let dir = TempDir::new().unwrap();
    fs::write(dir.path().join("test.mp4"), b"video content").unwrap();
    fs::write(dir.path().join("doc.pdf"), b"pdf content").unwrap();
    fs::write(dir.path().join("photo.jpg"), b"image content").unwrap();
    fs::write(dir.path().join("unknown.xyz"), b"unknown content").unwrap();

    let output = freespace()
        .args(["--json", "categories", dir.path().to_str().unwrap()])
        .env("RUST_LOG", "off")
        .output()
        .unwrap();

    assert!(output.status.success(), "categories --json must exit 0");
    let stdout = String::from_utf8(output.stdout).unwrap();
    let parsed: serde_json::Value =
        serde_json::from_str(&stdout).expect("stdout must be valid JSON");

    let categories = parsed.get("categories").expect("must have 'categories' key");
    let arr = categories.as_array().expect("categories must be an array");
    assert_eq!(arr.len(), 14, "must have exactly 14 category entries");

    // Check that each entry has the required fields
    for entry in arr {
        assert!(
            entry.get("category").is_some(),
            "each entry must have 'category'"
        );
        assert!(
            entry.get("total_bytes").is_some(),
            "each entry must have 'total_bytes'"
        );
        assert!(
            entry.get("file_count").is_some(),
            "each entry must have 'file_count'"
        );
    }
}

#[test]
fn test_categories_all_14_present() {
    let dir = TempDir::new().unwrap();
    // Just one file — all 14 categories must still appear (with 0 counts for others)
    fs::write(dir.path().join("single.txt"), b"just one file").unwrap();

    let output = freespace()
        .args(["--json", "categories", dir.path().to_str().unwrap()])
        .env("RUST_LOG", "off")
        .output()
        .unwrap();

    assert!(output.status.success(), "must exit 0");
    let stdout = String::from_utf8(output.stdout).unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&stdout).unwrap();

    let categories = parsed["categories"].as_array().unwrap();
    assert_eq!(
        categories.len(),
        14,
        "must always output all 14 categories even if most are zero"
    );
}

#[test]
fn test_categories_missing_path() {
    let output = freespace()
        .args(["categories", "/nonexistent/path/that/does/not/exist"])
        .env("RUST_LOG", "off")
        .output()
        .unwrap();

    assert!(
        !output.status.success(),
        "categories with nonexistent path must exit non-zero"
    );
}
