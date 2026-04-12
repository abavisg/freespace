use crate::analyze::ScanResult;
use serde::Serialize;
use std::cmp::Reverse;
use std::collections::{BinaryHeap, HashMap, HashSet};
use std::os::unix::fs::MetadataExt;
use std::path::PathBuf;
use walkdir::WalkDir;

const DEFAULT_TOP_N: usize = 20;

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
    let mut file_heap: BinaryHeap<Reverse<(u64, PathBuf)>> = BinaryHeap::new();
    let mut dir_sizes: HashMap<PathBuf, u64> = HashMap::new();

    // Get the device ID of the root so we can skip entries on other filesystems
    // (network mounts, /dev, /System/Volumes, etc.)
    let root_dev: Option<u64> = std::fs::metadata(root).ok().map(|m| m.dev());

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
                        tracing::debug!("metadata error at {:?}: {}", entry.path(), e);
                        result.skipped_count += 1;
                        continue;
                    }
                };
                // Skip entries on a different filesystem (network mounts, /dev, etc.)
                if let Some(rdev) = root_dev {
                    if metadata.dev() != rdev {
                        result.skipped_count += 1;
                        continue;
                    }
                }
                if metadata.is_dir() {
                    result.dir_count += 1;
                } else if metadata.is_file() {
                    let key = (metadata.dev(), metadata.ino());
                    if seen_inodes.insert(key) {
                        // First time seeing this inode — count physical allocation
                        let physical = metadata.blocks() * 512;
                        result.total_bytes += physical;
                        result.file_count += 1;

                        // Top-N largest files via bounded min-heap
                        if file_heap.len() < DEFAULT_TOP_N {
                            file_heap.push(Reverse((physical, entry.path().to_path_buf())));
                        } else if file_heap.peek().map_or(false, |Reverse((min, _))| physical > *min) {
                            file_heap.pop();
                            file_heap.push(Reverse((physical, entry.path().to_path_buf())));
                        }

                        // Directory size rollup: add this file's size to every ancestor up to root
                        let mut ancestor = entry.path().parent();
                        while let Some(dir) = ancestor {
                            if !dir.starts_with(root) && dir != root {
                                break;
                            }
                            *dir_sizes.entry(dir.to_path_buf()).or_insert(0) += physical;
                            if dir == root {
                                break;
                            }
                            ancestor = dir.parent();
                        }
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
                            tracing::debug!("permission denied: {:?}", e.path());
                        }
                        std::io::ErrorKind::NotFound => {
                            // Normal: file deleted between readdir and stat
                            tracing::debug!("not found (deleted mid-scan): {:?}", e.path());
                        }
                        _ => {
                            tracing::debug!("io error at {:?}: {}", e.path(), io_err);
                        }
                    }
                } else {
                    tracing::warn!("scan error at {:?}: {}", e.path(), e);
                }
                result.skipped_count += 1;
            }
        }
    }

    // Convert file heap to sorted Vec<FileEntry> (largest first).
    // into_sorted_vec() on BinaryHeap<Reverse<T>> returns in ascending Reverse order,
    // which is descending by original size — exactly what we want.
    result.largest_files = file_heap
        .into_sorted_vec()
        .into_iter()
        .map(|Reverse((size, path))| FileEntry { path, size, is_dir: false })
        .collect();

    // Select top-N directories from dir_sizes using same bounded heap pattern
    let mut dir_heap: BinaryHeap<Reverse<(u64, PathBuf)>> = BinaryHeap::new();
    for (path, size) in dir_sizes {
        if dir_heap.len() < DEFAULT_TOP_N {
            dir_heap.push(Reverse((size, path)));
        } else if dir_heap.peek().map_or(false, |Reverse((min, _))| size > *min) {
            dir_heap.pop();
            dir_heap.push(Reverse((size, path)));
        }
    }
    result.largest_dirs = dir_heap
        .into_sorted_vec()
        .into_iter()
        .map(|Reverse((size, path))| FileEntry { path, size, is_dir: true })
        .collect();

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

    #[test]
    fn bounded_heap_does_not_exceed_top_n() {
        let dir = TempDir::new().unwrap();
        // Create 30 files (more than DEFAULT_TOP_N=20)
        for i in 0..30 {
            let data = vec![0u8; (i + 1) * 512]; // varying sizes, multiples of 512
            fs::write(dir.path().join(format!("file_{:02}.dat", i)), &data).unwrap();
        }
        let result = scan_path(dir.path(), &default_config());
        assert!(
            result.largest_files.len() <= 20,
            "largest_files must be bounded to DEFAULT_TOP_N (20), got {}",
            result.largest_files.len()
        );
        // The largest file (30 * 512 = 15360 bytes written) should be present
        assert!(
            result.largest_files.iter().any(|f| f.path.file_name().unwrap() == "file_29.dat"),
            "largest file must appear in largest_files"
        );
    }

    #[test]
    fn dir_size_hardlink_dedup() {
        let dir = TempDir::new().unwrap();
        let sub = dir.path().join("subdir");
        fs::create_dir(&sub).unwrap();
        let original = sub.join("original.dat");
        fs::write(&original, vec![0u8; 4096]).unwrap();
        let link = sub.join("hardlink.dat");
        fs::hard_link(&original, &link).unwrap();

        let result = scan_path(dir.path(), &default_config());

        // Find the subdir in largest_dirs
        let subdir_entry = result
            .largest_dirs
            .iter()
            .find(|e| e.path == sub)
            .expect("subdir must appear in largest_dirs");

        // Size must be ONE copy, not two
        let meta = fs::metadata(&original).unwrap();
        let single_physical = meta.blocks() * 512;
        assert_eq!(
            subdir_entry.size, single_physical,
            "dir size must not double-count hardlinks: got {} expected {}",
            subdir_entry.size, single_physical
        );
    }

    #[test]
    fn largest_files_sorted_descending() {
        let dir = TempDir::new().unwrap();
        fs::write(dir.path().join("small.dat"), vec![0u8; 512]).unwrap();
        fs::write(dir.path().join("medium.dat"), vec![0u8; 4096]).unwrap();
        fs::write(dir.path().join("large.dat"), vec![0u8; 8192]).unwrap();

        let result = scan_path(dir.path(), &default_config());
        assert!(result.largest_files.len() >= 3);
        for w in result.largest_files.windows(2) {
            assert!(
                w[0].size >= w[1].size,
                "largest_files must be sorted descending: {} < {}",
                w[0].size, w[1].size
            );
        }
    }

    #[test]
    fn largest_dirs_includes_subdirs() {
        let dir = TempDir::new().unwrap();
        let a = dir.path().join("a");
        let b = a.join("b");
        fs::create_dir_all(&b).unwrap();
        fs::write(b.join("file.dat"), vec![0u8; 4096]).unwrap();

        let result = scan_path(dir.path(), &default_config());

        let dir_paths: Vec<_> = result.largest_dirs.iter().map(|e| &e.path).collect();
        assert!(
            dir_paths.contains(&&a),
            "largest_dirs must contain ancestor dir 'a', got {:?}",
            dir_paths
        );
        assert!(
            dir_paths.contains(&&b),
            "largest_dirs must contain leaf dir 'b', got {:?}",
            dir_paths
        );
    }
}
