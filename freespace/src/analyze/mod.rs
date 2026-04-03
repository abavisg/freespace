use serde::Serialize;
use std::path::PathBuf;

#[derive(Debug, Default, Serialize)]
pub struct ScanResult {
    pub root: PathBuf,
    pub total_bytes: u64,
    pub file_count: u64,
    pub dir_count: u64,
    pub skipped_count: u64,
    pub largest_files: Vec<crate::fs_scan::FileEntry>,
    pub largest_dirs: Vec<crate::fs_scan::FileEntry>,
}
