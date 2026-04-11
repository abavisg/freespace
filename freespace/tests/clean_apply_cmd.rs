use assert_cmd::Command;
use serde_json::json;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};
use tempfile::TempDir;

fn freespace() -> Command {
    Command::cargo_bin("freespace").unwrap()
}

fn setup_state(tmp: &TempDir) -> PathBuf {
    let dir = tmp.path().join("state");
    fs::create_dir_all(&dir).unwrap();
    dir
}

fn now_secs() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs()
}

fn write_session(
    state_dir: &Path,
    candidates: &[(PathBuf, u64, u64)], // (path, total_bytes, file_count)
    timestamp_secs: u64,
) {
    let cands: Vec<_> = candidates
        .iter()
        .map(|(p, tb, fc)| {
            json!({
                "path": p.to_string_lossy(),
                "total_bytes": tb,
                "file_count": fc,
                "safety": "safe",
            })
        })
        .collect();
    let session = json!({ "timestamp": timestamp_secs, "candidates": cands });
    fs::write(
        state_dir.join("preview-session.json"),
        serde_json::to_string_pretty(&session).unwrap(),
    )
    .unwrap();
}

fn read_audit_lines(state_dir: &Path) -> Vec<serde_json::Value> {
    let path = state_dir.join("cleanup.log");
    if !path.exists() {
        return vec![];
    }
    fs::read_to_string(&path)
        .unwrap()
        .lines()
        .filter(|l| !l.trim().is_empty())
        .map(|l| serde_json::from_str::<serde_json::Value>(l).unwrap())
        .collect()
}

/// APPLY-05: No session file → non-zero exit with "preview" in stderr
#[test]
fn test_apply_no_session_fails() {
    let tmp = TempDir::new().unwrap();
    let state_dir = setup_state(&tmp);
    // Do NOT create preview-session.json

    let output = freespace()
        .args(["clean", "apply"])
        .env("FREESPACE_STATE_DIR", &state_dir)
        .env("RUST_LOG", "off")
        .write_stdin("y\n")
        .output()
        .unwrap();

    assert!(
        !output.status.success(),
        "apply without session must exit non-zero"
    );
    let stderr = String::from_utf8_lossy(&output.stderr).to_lowercase();
    assert!(
        stderr.contains("preview"),
        "stderr must mention 'preview', got: {}",
        stderr
    );
}

/// APPLY-05: Session file older than 3600s → non-zero exit with "expired" in stderr
#[test]
fn test_apply_expired_session_fails() {
    let tmp = TempDir::new().unwrap();
    let state_dir = setup_state(&tmp);
    // Write session with epoch 0 — guaranteed > 1h old
    write_session(&state_dir, &[], 0);

    let output = freespace()
        .args(["clean", "apply"])
        .env("FREESPACE_STATE_DIR", &state_dir)
        .env("RUST_LOG", "off")
        .write_stdin("y\n")
        .output()
        .unwrap();

    assert!(
        !output.status.success(),
        "apply with expired session must exit non-zero"
    );
    let stderr = String::from_utf8_lossy(&output.stderr).to_lowercase();
    assert!(
        stderr.contains("expired"),
        "stderr must mention 'expired', got: {}",
        stderr
    );
}

/// APPLY-03: Protected path in session → skip (never deleted), audit log has "skip", stderr has "blocked"/"protected"
#[test]
fn test_apply_protected_path_never_deletes() {
    let tmp = TempDir::new().unwrap();
    let state_dir = setup_state(&tmp);

    // This path is under /private → must be blocked unconditionally
    let protected_path = PathBuf::from("/private/tmp/freespace_fake_nonexistent_file_do_not_create");
    write_session(
        &state_dir,
        &[(protected_path.clone(), 1024, 1)],
        now_secs(),
    );

    let output = freespace()
        .args(["clean", "apply", "--force"])
        .env("FREESPACE_STATE_DIR", &state_dir)
        .env("RUST_LOG", "off")
        .write_stdin("y\n")
        .output()
        .unwrap();

    assert!(
        output.status.success(),
        "apply with protected path should exit 0 (item skipped, pipeline continues), stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    // Audit log must show "skip" for this path
    let audit = read_audit_lines(&state_dir);
    let skip_entry = audit.iter().find(|entry| {
        entry["path"]
            .as_str()
            .map(|p| p.contains("freespace_fake_nonexistent_file_do_not_create"))
            .unwrap_or(false)
            && entry["action"].as_str() == Some("skip")
    });
    assert!(
        skip_entry.is_some(),
        "audit log must have a 'skip' entry for the protected path, got: {:?}",
        audit
    );

    // stderr must mention "blocked" or "protected"
    let stderr = String::from_utf8_lossy(&output.stderr).to_lowercase();
    assert!(
        stderr.contains("blocked") || stderr.contains("protected"),
        "stderr must mention 'blocked' or 'protected', got: {}",
        stderr
    );
}

/// APPLY-01: Safe candidates trashed; original path no longer exists; audit line has "action":"trash"
#[test]
fn test_apply_trashes_safe_candidates() {
    let tmp = TempDir::new().unwrap();
    let state_dir = setup_state(&tmp);

    // Create a real file to trash
    let victim = tmp.path().join("victim.txt");
    fs::write(&victim, "bye").unwrap();
    let victim_abs = victim.canonicalize().unwrap();

    write_session(&state_dir, &[(victim_abs.clone(), 3, 1)], now_secs());

    let output = freespace()
        .args(["clean", "apply"])
        .env("FREESPACE_STATE_DIR", &state_dir)
        .env("RUST_LOG", "off")
        .write_stdin("y\n")
        .output()
        .unwrap();

    assert!(
        output.status.success(),
        "apply must exit 0, stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    assert!(
        !victim_abs.exists(),
        "victim.txt must no longer exist at original path after trash"
    );

    let audit = read_audit_lines(&state_dir);
    let trash_entry = audit.iter().find(|entry| {
        entry["path"]
            .as_str()
            .map(|p| p.contains("victim.txt"))
            .unwrap_or(false)
            && entry["action"].as_str() == Some("trash")
    });
    assert!(
        trash_entry.is_some(),
        "audit log must have a 'trash' entry for victim.txt, got: {:?}",
        audit
    );
}

/// APPLY-02: Without --force → trash (not permanent delete); with --force → permanent delete
#[test]
fn test_apply_force_required_for_permanent_delete() {
    // Run A: without --force → trash
    let tmp_a = TempDir::new().unwrap();
    let state_a = setup_state(&tmp_a);
    let victim2 = tmp_a.path().join("victim2.txt");
    fs::write(&victim2, "delete me").unwrap();
    let victim2_abs = victim2.canonicalize().unwrap();
    write_session(&state_a, &[(victim2_abs.clone(), 9, 1)], now_secs());

    let out_a = freespace()
        .args(["clean", "apply"])
        .env("FREESPACE_STATE_DIR", &state_a)
        .env("RUST_LOG", "off")
        .write_stdin("y\n")
        .output()
        .unwrap();

    assert!(
        out_a.status.success(),
        "Run A (no --force) must exit 0, stderr: {}",
        String::from_utf8_lossy(&out_a.stderr)
    );
    assert!(
        !victim2_abs.exists(),
        "victim2.txt should be gone after trash"
    );
    let audit_a = read_audit_lines(&state_a);
    let trash_line = audit_a
        .iter()
        .find(|e| e["action"].as_str() == Some("trash") && e["path"].as_str().map(|p| p.contains("victim2.txt")).unwrap_or(false));
    assert!(
        trash_line.is_some(),
        "Run A audit must show 'trash' for victim2.txt, got: {:?}",
        audit_a
    );

    // Run B: with --force → permanent delete
    let tmp_b = TempDir::new().unwrap();
    let state_b = setup_state(&tmp_b);
    let victim3 = tmp_b.path().join("victim3.txt");
    fs::write(&victim3, "really delete me").unwrap();
    let victim3_abs = victim3.canonicalize().unwrap();
    write_session(&state_b, &[(victim3_abs.clone(), 16, 1)], now_secs());

    let out_b = freespace()
        .args(["clean", "apply", "--force"])
        .env("FREESPACE_STATE_DIR", &state_b)
        .env("RUST_LOG", "off")
        .write_stdin("y\n")
        .output()
        .unwrap();

    assert!(
        out_b.status.success(),
        "Run B (--force) must exit 0, stderr: {}",
        String::from_utf8_lossy(&out_b.stderr)
    );
    assert!(
        !victim3_abs.exists(),
        "victim3.txt should be gone after force delete"
    );
    let audit_b = read_audit_lines(&state_b);
    let delete_line = audit_b
        .iter()
        .find(|e| e["action"].as_str() == Some("delete") && e["path"].as_str().map(|p| p.contains("victim3.txt")).unwrap_or(false));
    assert!(
        delete_line.is_some(),
        "Run B audit must show 'delete' for victim3.txt, got: {:?}",
        audit_b
    );
}

/// APPLY-04: Audit log exists after apply, each line is valid JSON with required fields
#[test]
fn test_apply_audit_log_written() {
    let tmp = TempDir::new().unwrap();
    let state_dir = setup_state(&tmp);

    let auditme = tmp.path().join("auditme.txt");
    fs::write(&auditme, "abc").unwrap(); // 3 bytes
    let auditme_abs = auditme.canonicalize().unwrap();

    write_session(&state_dir, &[(auditme_abs.clone(), 3, 1)], now_secs());

    let output = freespace()
        .args(["clean", "apply"])
        .env("FREESPACE_STATE_DIR", &state_dir)
        .env("RUST_LOG", "off")
        .write_stdin("y\n")
        .output()
        .unwrap();

    assert!(
        output.status.success(),
        "apply must exit 0, stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let log_path = state_dir.join("cleanup.log");
    assert!(log_path.exists(), "cleanup.log must exist after apply");

    let lines = read_audit_lines(&state_dir);
    assert!(
        !lines.is_empty(),
        "cleanup.log must contain at least one entry"
    );

    // Validate last line has required fields
    let entry = lines.last().unwrap();
    assert!(
        entry.get("timestamp").and_then(|v| v.as_str()).is_some(),
        "entry must have string 'timestamp'"
    );
    assert!(
        entry.get("path").and_then(|v| v.as_str()).is_some(),
        "entry must have string 'path'"
    );
    assert!(
        entry.get("size_bytes").and_then(|v| v.as_u64()).is_some(),
        "entry must have number 'size_bytes'"
    );
    assert!(
        entry.get("action").and_then(|v| v.as_str()).is_some(),
        "entry must have string 'action'"
    );

    // Validate timestamp format: YYYY-MM-DDTHH:MM:SSZ
    let ts = entry["timestamp"].as_str().unwrap();
    let ts_re = regex_lite_check(ts);
    assert!(
        ts_re,
        "timestamp '{}' must match ISO 8601 pattern YYYY-MM-DDTHH:MM:SSZ",
        ts
    );
}

/// Simple manual regex check for ISO 8601 UTC timestamp format YYYY-MM-DDTHH:MM:SSZ
fn regex_lite_check(ts: &str) -> bool {
    if ts.len() != 20 {
        return false;
    }
    let b = ts.as_bytes();
    // YYYY-MM-DDTHH:MM:SSZ
    b[4] == b'-'
        && b[7] == b'-'
        && b[10] == b'T'
        && b[13] == b':'
        && b[16] == b':'
        && b[19] == b'Z'
        && b[0..4].iter().all(|c| c.is_ascii_digit())
        && b[5..7].iter().all(|c| c.is_ascii_digit())
        && b[8..10].iter().all(|c| c.is_ascii_digit())
        && b[11..13].iter().all(|c| c.is_ascii_digit())
        && b[14..16].iter().all(|c| c.is_ascii_digit())
        && b[17..19].iter().all(|c| c.is_ascii_digit())
}

/// Network volume lock-in: path under FREESPACE_FAKE_NETWORK_MOUNT is skipped,
/// file not deleted, audit line has "action":"skip", stderr has "network volume"
#[test]
fn test_apply_network_volume_warned_and_skipped() {
    let tmp = TempDir::new().unwrap();
    let state_dir = setup_state(&tmp);

    // Create a fake "network mount" dir and a file under it
    let fake_net_mount = tmp.path().join("fake_net_mount");
    fs::create_dir_all(&fake_net_mount).unwrap();
    let net_file = fake_net_mount.join("net_file.txt");
    fs::write(&net_file, "network file").unwrap();
    let net_file_abs = net_file.canonicalize().unwrap();
    let fake_net_mount_abs = fake_net_mount.canonicalize().unwrap();

    write_session(&state_dir, &[(net_file_abs.clone(), 12, 1)], now_secs());

    let output = freespace()
        .args(["clean", "apply"])
        .env("FREESPACE_STATE_DIR", &state_dir)
        .env("FREESPACE_FAKE_NETWORK_MOUNT", &fake_net_mount_abs)
        .env("RUST_LOG", "off")
        .write_stdin("y\n")
        .output()
        .unwrap();

    assert!(
        output.status.success(),
        "apply with network-mount file must exit 0, stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    // File must NOT have been deleted
    assert!(
        net_file_abs.exists(),
        "file on network mount must not be deleted"
    );

    // Audit log must show "skip"
    let audit = read_audit_lines(&state_dir);
    let skip_entry = audit.iter().find(|e| {
        e["path"]
            .as_str()
            .map(|p| p.contains("net_file.txt"))
            .unwrap_or(false)
            && e["action"].as_str() == Some("skip")
    });
    assert!(
        skip_entry.is_some(),
        "audit must have 'skip' for network file, got: {:?}",
        audit
    );

    // stderr must mention "network volume"
    let stderr = String::from_utf8_lossy(&output.stderr).to_lowercase();
    assert!(
        stderr.contains("network volume"),
        "stderr must mention 'network volume', got: {}",
        stderr
    );
}

/// --json mode bypasses confirmation prompt (no stdin needed); exits success; file gone; audit present
#[test]
fn test_apply_json_mode_bypasses_prompt() {
    let tmp = TempDir::new().unwrap();
    let state_dir = setup_state(&tmp);

    let jsontest = tmp.path().join("jsontest.txt");
    fs::write(&jsontest, "json mode test").unwrap();
    let jsontest_abs = jsontest.canonicalize().unwrap();

    write_session(&state_dir, &[(jsontest_abs.clone(), 14, 1)], now_secs());

    // Do NOT call .write_stdin() — proves no stdin blocking
    let output = freespace()
        .args(["--json", "clean", "apply"])
        .env("FREESPACE_STATE_DIR", &state_dir)
        .env("RUST_LOG", "off")
        .timeout(std::time::Duration::from_secs(10))
        .output()
        .unwrap();

    assert!(
        output.status.success(),
        "apply --json must exit 0 without stdin, stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    // stdout must be valid JSON
    let stdout = String::from_utf8_lossy(&output.stdout);
    let parsed: serde_json::Value =
        serde_json::from_str(&stdout).expect("stdout must be valid JSON in --json mode");
    assert!(
        parsed.get("status").is_some(),
        "JSON output must have 'status' field"
    );

    // File must be gone (trashed)
    assert!(
        !jsontest_abs.exists(),
        "jsontest.txt must be gone after apply --json"
    );

    // Audit line must be present
    let audit = read_audit_lines(&state_dir);
    assert!(
        !audit.is_empty(),
        "audit log must have at least one entry after --json apply"
    );
}
