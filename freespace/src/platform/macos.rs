use std::path::PathBuf;

/// Returns the five protected root paths, canonicalized at startup.
/// Paths that cannot be canonicalized (non-standard environments or tests)
/// are stored as-is with a warning emitted to stderr.
#[cfg(target_os = "macos")]
pub fn protected_paths() -> Vec<PathBuf> {
    const RAW: &[&str] = &["/System", "/usr", "/bin", "/sbin", "/private"];
    RAW.iter()
        .map(|raw| {
            std::fs::canonicalize(raw).unwrap_or_else(|e| {
                tracing::warn!("Could not canonicalize protected path {raw}: {e}");
                PathBuf::from(raw)
            })
        })
        .collect()
}

/// Returns true if the given path starts with any protected path prefix.
#[cfg(target_os = "macos")]
pub fn is_protected(path: &std::path::Path, protected: &[PathBuf]) -> bool {
    protected.iter().any(|p| path.starts_with(p))
}

#[cfg(test)]
#[cfg(target_os = "macos")]
mod tests {
    use super::*;

    #[test]
    fn protected_paths_returns_five() {
        let paths = protected_paths();
        assert_eq!(paths.len(), 5);
    }

    #[test]
    fn is_protected_system() {
        let protected = vec![
            PathBuf::from("/System"),
            PathBuf::from("/usr"),
            PathBuf::from("/bin"),
            PathBuf::from("/sbin"),
            PathBuf::from("/private"),
        ];
        assert!(is_protected(
            std::path::Path::new("/System/Library/CoreServices"),
            &protected
        ));
    }

    #[test]
    fn is_protected_home_is_not_protected() {
        let protected = vec![
            PathBuf::from("/System"),
            PathBuf::from("/usr"),
            PathBuf::from("/bin"),
            PathBuf::from("/sbin"),
            PathBuf::from("/private"),
        ];
        assert!(!is_protected(
            std::path::Path::new("/Users/alice/Documents"),
            &protected
        ));
    }
}
