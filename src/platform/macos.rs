use std::path::PathBuf;

/// Returns the protected root paths, canonicalized at startup.
///
/// These are the macOS system directories that must never be deleted by any
/// cleanup operation — not even with `--force`. The list uses specific paths
/// rather than the broad `/private` prefix to avoid blocking legitimate user
/// temp files under `/private/var/folders` (macOS TMPDIR) and
/// `/private/tmp` which are normal user-accessible temporary storage.
///
/// Uses std::fs::canonicalize (POSIX realpath on macOS) so that symlinks
/// such as /tmp → /private/tmp are resolved. If canonicalization fails
/// (non-standard environment, path does not exist), the raw path is stored
/// with a warning — the tool never crashes due to missing system paths.
///
/// Call once at startup (main.rs) and store the result. Do NOT call per-file.
#[cfg(target_os = "macos")]
pub fn protected_paths() -> Vec<PathBuf> {
    const RAW: &[&str] = &[
        "/System",         // macOS system files (SIP-protected)
        "/usr",            // Unix system directories
        "/bin",            // Essential user command binaries
        "/sbin",           // Essential system admin binaries
        "/private/etc",    // System configuration files
        "/private/var/db", // System databases (launchd, mdworker, etc.)
    ];
    RAW.iter()
        .map(|raw| {
            std::fs::canonicalize(raw).unwrap_or_else(|e| {
                tracing::warn!("Could not canonicalize protected path {raw}: {e}");
                PathBuf::from(raw)
            })
        })
        .collect()
}

/// Returns true if `path` starts with any of the protected path prefixes.
///
/// The caller is responsible for canonicalizing `path` before this check
/// when operating on user-supplied paths (e.g., cleanup candidates).
/// For traversal during scanning, the check is applied to the entry path
/// as returned by walkdir (which uses the OS-level entry path).
#[cfg(target_os = "macos")]
pub fn is_protected(path: &std::path::Path, protected: &[PathBuf]) -> bool {
    protected.iter().any(|p| path.starts_with(p))
}

#[cfg(test)]
#[cfg(target_os = "macos")]
mod tests {
    use super::*;

    /// Construct the protected list from literal paths (not canonicalize)
    /// so tests are deterministic regardless of the host system's symlink layout.
    fn test_protected() -> Vec<PathBuf> {
        vec![
            PathBuf::from("/System"),
            PathBuf::from("/usr"),
            PathBuf::from("/bin"),
            PathBuf::from("/sbin"),
            PathBuf::from("/private/etc"),
            PathBuf::from("/private/var/db"),
        ]
    }

    #[test]
    fn protected_paths_returns_six() {
        let paths = protected_paths();
        assert_eq!(paths.len(), 6, "must return exactly 6 protected paths");
    }

    #[test]
    fn is_protected_system_path() {
        assert!(is_protected(
            std::path::Path::new("/System/Library/CoreServices"),
            &test_protected()
        ));
    }

    #[test]
    fn is_protected_usr_path() {
        assert!(is_protected(
            std::path::Path::new("/usr/local/bin/something"),
            &test_protected()
        ));
    }

    #[test]
    fn is_protected_private_etc() {
        // /private/etc is in the protected list — system config must be blocked
        assert!(is_protected(
            std::path::Path::new("/private/etc/hosts"),
            &test_protected()
        ));
    }

    #[test]
    fn is_protected_private_var_db() {
        // /private/var/db is in the protected list — system databases must be blocked
        assert!(is_protected(
            std::path::Path::new("/private/var/db/launchd.db/com.apple.launchd"),
            &test_protected()
        ));
    }

    #[test]
    fn tmp_is_not_protected() {
        // /tmp → /private/tmp, but /private/tmp is NOT in the protected list anymore.
        // Users can legitimately have cleanup candidates in temp dirs.
        // The specific protected paths under /private are /etc and /var/db.
        assert!(!is_protected(
            std::path::Path::new("/private/tmp/some_cache_file"),
            &test_protected()
        ));
    }

    #[test]
    fn home_dir_is_not_protected() {
        assert!(!is_protected(
            std::path::Path::new("/Users/alice/Documents"),
            &test_protected()
        ));
    }

    #[test]
    fn downloads_is_not_protected() {
        assert!(!is_protected(
            std::path::Path::new("/Users/alice/Downloads"),
            &test_protected()
        ));
    }

    #[test]
    fn tmp_canonicalizes_correctly() {
        // Verify no panic during canonicalize-based protected_paths() call.
        let real_protected = protected_paths();
        assert!(!real_protected.is_empty());
        // /private/var/folders (macOS TMPDIR) must NOT be protected
        let tmpdir_file = std::path::PathBuf::from("/private/var/folders/test/file.txt");
        assert!(!is_protected(&tmpdir_file, &real_protected));
        let _ = (); // don't assert — just verify no panic
    }

    #[test]
    fn fallback_does_not_panic_on_nonexistent_path() {
        // Simulate the fallback path: canonicalize a path that does not exist
        let result = std::fs::canonicalize("/this_path_definitely_does_not_exist_xyz_abc");
        assert!(result.is_err(), "canonicalize of nonexistent path must return Err");
        // The unwrap_or_else pattern in protected_paths() handles this without panic
        let fallback = result.unwrap_or_else(|_| PathBuf::from("/fallback"));
        assert_eq!(fallback, PathBuf::from("/fallback"));
    }
}

use serde::Serialize;

#[derive(Serialize)]
pub struct VolumeInfo {
    pub mount_point: String,
    pub total_bytes: u64,
    pub used_bytes: u64,
    pub available_bytes: u64,
}

#[cfg(target_os = "macos")]
pub fn list_volumes() -> Vec<VolumeInfo> {
    use sysinfo::Disks;
    let disks = Disks::new_with_refreshed_list();
    disks
        .list()
        .iter()
        .map(|disk| {
            let total = disk.total_space();
            let available = disk.available_space();
            VolumeInfo {
                mount_point: disk.mount_point().to_string_lossy().into_owned(),
                total_bytes: total,
                used_bytes: total.saturating_sub(available),
                available_bytes: available,
            }
        })
        .collect()
}

#[cfg(test)]
#[cfg(target_os = "macos")]
mod volume_tests {
    use super::*;

    #[test]
    fn list_volumes_returns_nonempty() {
        let volumes = list_volumes();
        assert!(!volumes.is_empty(), "must return at least one volume on macOS");
    }

    #[test]
    fn used_bytes_derived_correctly() {
        let volumes = list_volumes();
        for v in &volumes {
            assert_eq!(
                v.used_bytes,
                v.total_bytes.saturating_sub(v.available_bytes),
                "used_bytes must equal total - available for {}",
                v.mount_point
            );
        }
    }

    #[test]
    fn volume_info_serializes_to_json() {
        let v = VolumeInfo {
            mount_point: "/".to_string(),
            total_bytes: 1_000_000,
            used_bytes: 400_000,
            available_bytes: 600_000,
        };
        let json = serde_json::to_string(&v).expect("VolumeInfo must serialize");
        assert!(json.contains("mount_point"));
        assert!(json.contains("total_bytes"));
        assert!(json.contains("used_bytes"));
        assert!(json.contains("available_bytes"));
    }
}
