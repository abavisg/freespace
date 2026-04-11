use serde::Serialize;
use std::path::Path;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum Category {
    Video,
    Audio,
    Images,
    Documents,
    Archives,
    Applications,
    Developer,
    Caches,
    Mail,
    Containers,
    CloudSync,
    Hidden,
    SystemRelated,
    Unknown,
}

impl Category {
    pub fn all() -> &'static [Category] {
        &[
            Category::Video,
            Category::Audio,
            Category::Images,
            Category::Documents,
            Category::Archives,
            Category::Applications,
            Category::Developer,
            Category::Caches,
            Category::Mail,
            Category::Containers,
            Category::CloudSync,
            Category::Hidden,
            Category::SystemRelated,
            Category::Unknown,
        ]
    }
}

impl std::fmt::Display for Category {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            Category::Video => "video",
            Category::Audio => "audio",
            Category::Images => "images",
            Category::Documents => "documents",
            Category::Archives => "archives",
            Category::Applications => "applications",
            Category::Developer => "developer",
            Category::Caches => "caches",
            Category::Mail => "mail",
            Category::Containers => "containers",
            Category::CloudSync => "cloud-sync",
            Category::Hidden => "hidden",
            Category::SystemRelated => "system-related",
            Category::Unknown => "unknown",
        };
        write!(f, "{}", s)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, serde::Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum SafetyClass {
    Safe,
    Caution,
    Dangerous,
    Blocked,
}

impl std::fmt::Display for SafetyClass {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            SafetyClass::Safe => "safe",
            SafetyClass::Caution => "caution",
            SafetyClass::Dangerous => "dangerous",
            SafetyClass::Blocked => "blocked",
        };
        write!(f, "{}", s)
    }
}

/// Classify a path into a Category using tiered priority:
/// 1. System path rules (beats everything)
/// 2. Trash override -> Unknown
/// 3. Known macOS dirs under home
/// 4. Hidden files (dotfiles)
/// 5. Extension-based classification
/// 6. Fallback: Unknown
pub fn classify_path(path: &Path, home: &Path) -> Category {
    // Tier 1: System path rules
    if path.starts_with("/System")
        || path.starts_with("/usr")
        || path.starts_with("/bin")
        || path.starts_with("/sbin")
        || path.starts_with("/private")
    {
        return Category::SystemRelated;
    }

    // Tier 2: Trash override
    if path.starts_with(home.join(".Trash")) {
        return Category::Unknown;
    }

    // Tier 3: Known macOS dirs under home
    if path.starts_with(home.join("Library/Caches")) {
        return Category::Caches;
    }
    if path.starts_with(home.join("Library/Mail")) {
        return Category::Mail;
    }
    if path.starts_with(home.join("Library/Containers")) {
        return Category::Containers;
    }
    if path.starts_with(home.join("Library/Developer")) {
        return Category::Developer;
    }
    if path.starts_with(home.join("Library/CloudStorage")) {
        return Category::CloudSync;
    }
    if path.starts_with(home.join("Library/Mobile Documents")) {
        return Category::CloudSync;
    }
    if path.starts_with(home.join(".ollama")) {
        return Category::Developer;
    }
    if path.starts_with(home.join(".dropbox")) {
        return Category::CloudSync;
    }

    // Tier 4: Hidden (dotfiles or inside a hidden directory)
    if is_hidden(path) || path_has_hidden_component(path, home) {
        return Category::Hidden;
    }

    // Tier 5 & 6: Extension-based (returns Unknown for unmapped)
    classify_by_extension(path)
}

/// Returns true if the file name starts with a dot.
pub fn is_hidden(path: &Path) -> bool {
    path.file_name()
        .and_then(|n| n.to_str())
        .map(|n| n.starts_with('.'))
        .unwrap_or(false)
}

/// Returns true if any path component (after home) starts with a dot.
/// This catches files inside hidden directories like ~/.ssh/config.
fn path_has_hidden_component(path: &Path, home: &Path) -> bool {
    // Get the portion of the path relative to home (or use the whole path)
    let rel = path.strip_prefix(home).unwrap_or(path);
    rel.components().any(|c| {
        if let std::path::Component::Normal(s) = c {
            s.to_str().map(|s| s.starts_with('.')).unwrap_or(false)
        } else {
            false
        }
    })
}

/// Classify by file extension. Returns Unknown for unmapped extensions.
fn classify_by_extension(path: &Path) -> Category {
    let ext = path
        .extension()
        .and_then(|e| e.to_str())
        .map(|e| e.to_lowercase());

    match ext.as_deref() {
        Some("mp4" | "mov" | "avi" | "mkv" | "m4v" | "wmv" | "flv" | "webm") => Category::Video,
        Some("mp3" | "aac" | "flac" | "wav" | "m4a" | "ogg" | "opus") => Category::Audio,
        Some("jpg" | "jpeg" | "png" | "gif" | "bmp" | "tiff" | "webp" | "heic") => {
            Category::Images
        }
        Some(
            "pdf" | "doc" | "docx" | "xls" | "xlsx" | "ppt" | "pptx" | "txt" | "md" | "pages"
            | "numbers" | "keynote",
        ) => Category::Documents,
        Some("zip" | "tar" | "gz" | "bz2" | "xz" | "7z" | "rar" | "dmg" | "pkg") => {
            Category::Archives
        }
        Some("app" | "ipa" | "apk") => Category::Applications,
        _ => Category::Unknown,
    }
}

/// Classify a path into a SafetyClass.
pub fn safety_class(path: &Path, home: &Path) -> SafetyClass {
    if path.starts_with("/System")
        || path.starts_with("/usr")
        || path.starts_with("/bin")
        || path.starts_with("/sbin")
        || path.starts_with("/private")
    {
        return SafetyClass::Blocked;
    }
    if path.starts_with(home.join("Library/Caches")) {
        return SafetyClass::Safe;
    }
    if path.starts_with(home.join(".npm")) {
        return SafetyClass::Caution;
    }
    if path.starts_with(home.join(".cargo/registry")) {
        return SafetyClass::Caution;
    }
    if path.starts_with(home.join("Library/Developer/Xcode/DerivedData")) {
        return SafetyClass::Caution;
    }
    if path.starts_with(home.join("Library/Containers/com.docker.docker")) {
        return SafetyClass::Dangerous;
    }
    SafetyClass::Caution
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    fn home() -> &'static Path {
        Path::new("/Users/testuser")
    }

    // ---- classify_path tests ----

    #[test]
    fn system_library_is_system_related() {
        assert_eq!(
            classify_path(Path::new("/System/Library/foo.txt"), home()),
            Category::SystemRelated
        );
    }

    #[test]
    fn usr_local_bin_is_system_related() {
        assert_eq!(
            classify_path(Path::new("/usr/local/bin/tool"), home()),
            Category::SystemRelated
        );
    }

    #[test]
    fn home_library_caches_is_caches() {
        assert_eq!(
            classify_path(
                &home().join("Library/Caches/com.apple.foo/data"),
                home()
            ),
            Category::Caches
        );
    }

    #[test]
    fn home_library_mail_is_mail() {
        assert_eq!(
            classify_path(&home().join("Library/Mail/V10/foo.emlx"), home()),
            Category::Mail
        );
    }

    #[test]
    fn home_library_containers_is_containers() {
        assert_eq!(
            classify_path(
                &home().join("Library/Containers/com.docker.docker/foo"),
                home()
            ),
            Category::Containers
        );
    }

    #[test]
    fn dotollama_is_developer() {
        assert_eq!(
            classify_path(&home().join(".ollama/models/blob"), home()),
            Category::Developer
        );
    }

    #[test]
    fn library_developer_is_developer() {
        assert_eq!(
            classify_path(
                &home().join("Library/Developer/Xcode/DerivedData/foo"),
                home()
            ),
            Category::Developer
        );
    }

    #[test]
    fn library_cloud_storage_is_cloud_sync() {
        assert_eq!(
            classify_path(&home().join("Library/CloudStorage/foo"), home()),
            Category::CloudSync
        );
    }

    #[test]
    fn library_mobile_documents_is_cloud_sync() {
        assert_eq!(
            classify_path(&home().join("Library/Mobile Documents/foo"), home()),
            Category::CloudSync
        );
    }

    #[test]
    fn dropbox_is_cloud_sync() {
        assert_eq!(
            classify_path(&home().join(".dropbox/foo"), home()),
            Category::CloudSync
        );
    }

    #[test]
    fn trash_mp4_is_unknown() {
        assert_eq!(
            classify_path(&home().join(".Trash/foo.mp4"), home()),
            Category::Unknown
        );
    }

    #[test]
    fn dotfile_is_hidden() {
        assert_eq!(
            classify_path(&home().join(".ssh/config"), home()),
            Category::Hidden
        );
    }

    #[test]
    fn mp4_is_video() {
        assert_eq!(
            classify_path(&home().join("video.mp4"), home()),
            Category::Video
        );
    }

    #[test]
    fn mp3_is_audio() {
        assert_eq!(
            classify_path(&home().join("song.mp3"), home()),
            Category::Audio
        );
    }

    #[test]
    fn jpg_is_images() {
        assert_eq!(
            classify_path(&home().join("photo.jpg"), home()),
            Category::Images
        );
    }

    #[test]
    fn pdf_is_documents() {
        assert_eq!(
            classify_path(&home().join("report.pdf"), home()),
            Category::Documents
        );
    }

    #[test]
    fn zip_is_archives() {
        assert_eq!(
            classify_path(&home().join("backup.zip"), home()),
            Category::Archives
        );
    }

    #[test]
    fn app_is_applications() {
        assert_eq!(
            classify_path(&home().join("tool.app"), home()),
            Category::Applications
        );
    }

    #[test]
    fn unknown_extension_is_unknown() {
        assert_eq!(
            classify_path(&home().join("random.xyz"), home()),
            Category::Unknown
        );
    }

    #[test]
    fn path_rule_beats_extension() {
        // ~/Library/Caches/foo.mp4 must be Caches, NOT Video
        assert_eq!(
            classify_path(&home().join("Library/Caches/foo.mp4"), home()),
            Category::Caches
        );
    }

    #[test]
    fn all_returns_14_variants() {
        assert_eq!(Category::all().len(), 14);
    }

    // ---- is_hidden tests ----

    #[test]
    fn dotfile_hidden_true() {
        assert!(is_hidden(Path::new(".gitignore")));
    }

    #[test]
    fn visible_file_hidden_false() {
        assert!(!is_hidden(Path::new("visible.txt")));
    }

    // ---- safety_class tests ----

    #[test]
    fn library_caches_is_safe() {
        assert_eq!(
            safety_class(&home().join("Library/Caches"), home()),
            SafetyClass::Safe
        );
    }

    #[test]
    fn xcode_derived_data_is_caution() {
        assert_eq!(
            safety_class(
                &home().join("Library/Developer/Xcode/DerivedData"),
                home()
            ),
            SafetyClass::Caution
        );
    }

    #[test]
    fn docker_containers_is_dangerous() {
        assert_eq!(
            safety_class(
                &home().join("Library/Containers/com.docker.docker"),
                home()
            ),
            SafetyClass::Dangerous
        );
    }

    #[test]
    fn system_library_is_blocked() {
        assert_eq!(
            safety_class(Path::new("/System/Library"), home()),
            SafetyClass::Blocked
        );
    }

    // ---- SafetyClass Ord tests ----

    #[test]
    fn safety_class_ord_safe_lt_caution() {
        assert!(SafetyClass::Safe < SafetyClass::Caution);
    }

    #[test]
    fn safety_class_ord_caution_lt_dangerous() {
        assert!(SafetyClass::Caution < SafetyClass::Dangerous);
    }

    #[test]
    fn safety_class_ord_dangerous_lt_blocked() {
        assert!(SafetyClass::Dangerous < SafetyClass::Blocked);
    }
}
