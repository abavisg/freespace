use crate::analyze::ScanResult;
use serde::Serialize;
use std::collections::HashSet;
use std::os::unix::fs::MetadataExt;
use std::path::PathBuf;
use walkdir::WalkDir;

#[derive(Debug, Clone, Serialize)]
pub struct FileEntry {
    pub path: PathBuf,
    pub size: u64, // physical bytes: metadata.blocks() * 512
    pub is_dir: bool,
}

pub fn scan_path(root: &std::path::Path, config: &crate::config::schema::Config) -> ScanResult {
    let mut result = ScanResult {
        root: root.to_path_buf(),
        ..Default::default()
    };
    let mut seen_inodes: HashSet<(u64, u64)> = HashSet::new();

    for entry_result in WalkDir::new(root).follow_links(false) {
        match entry_result {
            Ok(entry) => {
                // Skip configured exclusions
                if config
                    .scan
                    .exclude
                    .iter()
                    .any(|ex| entry.path().starts_with(ex))
                {
                    continue;
                }
                // Skip symlinks entirely (not counted; target will be counted via its own path)
                if entry.file_type().is_symlink() {
                    continue;
                }
                let metadata = match entry.metadata() {
                    Ok(m) => m,
                    Err(e) => {
                        tracing::warn!("metadata error at {:?}: {}", entry.path(), e);
                        result.skipped_count += 1;
                        continue;
                    }
                };
                if metadata.is_dir() {
                    result.dir_count += 1;
                } else if metadata.is_file() {
                    let key = (metadata.dev(), metadata.ino());
                    if seen_inodes.insert(key) {
                        // First time seeing this inode — count physical allocation
                        let physical = metadata.blocks() * 512;
                        result.total_bytes += physical;
                        result.file_count += 1;
                        // Phase 5: update largest_files BinaryHeap here
                    }
                    // else: hardlink already counted — skip
                }
                // other file types (sockets, pipes, devices): ignore
            }
            Err(e) => {
                if e.loop_ancestor().is_some() {
                    tracing::warn!("symlink loop detected at {:?}", e.path());
                } else if let Some(io_err) = e.io_error() {
                    match io_err.kind() {
                        std::io::ErrorKind::PermissionDenied => {
                            tracing::warn!("permission denied: {:?}", e.path());
                        }
                        std::io::ErrorKind::NotFound => {
                            // Normal: file deleted between readdir and stat
                            tracing::debug!("not found (deleted mid-scan): {:?}", e.path());
                        }
                        _ => {
                            tracing::warn!("io error at {:?}: {}", e.path(), io_err);
                        }
                    }
                } else {
                    tracing::warn!("scan error at {:?}: {}", e.path(), e);
                }
                result.skipped_count += 1;
            }
        }
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::schema::Config;
    use std::fs;
    use tempfile::TempDir;

    fn default_config() -> Config {
        Config::default()
    }

    #[test]
    fn scan_single_file_counts() {
        let dir = TempDir::new().unwrap();
        let file = dir.path().join("test.txt");
        fs::write(&file, b"hello world").unwrap();
        let result = scan_path(&file, &default_config());
        assert_eq!(result.file_count, 1, "single file must yield file_count=1");
        assert_eq!(result.dir_count, 0, "single file has no dirs");
        // Physical size must be blocks*512, not len()
        let meta = fs::metadata(&file).unwrap();
        let physical = meta.blocks() * 512;
        assert_eq!(
            result.total_bytes, physical,
            "total_bytes must be physical (blocks*512)"
        );
    }

    #[test]
    fn scan_single_dir_counts() {
        let dir = TempDir::new().unwrap();
        let result = scan_path(dir.path(), &default_config());
        // The root dir itself is counted as dir_count=1, no files
        assert_eq!(result.dir_count, 1, "root dir must be counted");
        assert_eq!(result.file_count, 0, "empty dir has no files");
    }

    #[test]
    fn scan_multiple_files_count() {
        let dir = TempDir::new().unwrap();
        fs::write(dir.path().join("a.txt"), b"alpha").unwrap();
        fs::write(dir.path().join("b.txt"), b"beta").unwrap();
        fs::write(dir.path().join("c.txt"), b"gamma").unwrap();
        let result = scan_path(dir.path(), &default_config());
        assert_eq!(result.file_count, 3, "must count all 3 files");
    }

    #[test]
    fn scan_hardlink_dedup() {
        let dir = TempDir::new().unwrap();
        let original = dir.path().join("original.dat");
        fs::write(&original, b"hardlink test content here").unwrap();
        let link = dir.path().join("hardlink.dat");
        fs::hard_link(&original, &link).unwrap();

        let result = scan_path(dir.path(), &default_config());
        assert_eq!(
            result.file_count, 1,
            "hardlinked files must be counted once, got {}",
            result.file_count
        );

        // total_bytes must be for ONE copy only
        let meta = fs::metadata(&original).unwrap();
        let single_physical = meta.blocks() * 512;
        assert_eq!(
            result.total_bytes, single_physical,
            "total_bytes must not be doubled for hardlinks"
        );
    }

    #[test]
    fn scan_clean_tempdir_zero_skipped() {
        let dir = TempDir::new().unwrap();
        let result = scan_path(dir.path(), &default_config());
        assert_eq!(result.skipped_count, 0, "clean tempdir must have zero skipped");
    }

    #[test]
    fn scan_dangling_symlink_no_panic() {
        let dir = TempDir::new().unwrap();
        let link = dir.path().join("broken_link");
        // Create a symlink pointing to a nonexistent target
        std::os::unix::fs::symlink("/nonexistent/target/does/not/exist", &link).unwrap();

        // Must not panic; skipped_count may or may not increment depending on how walkdir handles it
        let result = scan_path(dir.path(), &default_config());
        // The scan must complete without panic
        let _ = result;
    }
}
