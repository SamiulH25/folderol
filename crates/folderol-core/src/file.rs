use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

/// What a path points at, without following symlinks.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum FileKind {
    File,
    Directory,
    Symlink,
    Other,
}

/// The metadata Folderol keeps for one path.
///
/// Content is deliberately *not* part of this record. Extraction happens in a
/// later stage so a scan never blocks on a PDF parser or a network mount.
#[derive(Debug, Clone)]
pub struct FileRecord {
    pub path: PathBuf,
    pub name: String,
    pub extension: Option<String>,
    pub mime: Option<String>,
    pub size: u64,
    pub modified: Option<SystemTime>,
    pub kind: FileKind,
}

impl FileRecord {
    /// Stat a path. Symlinks are described, not followed.
    pub fn from_path(path: impl Into<PathBuf>) -> std::io::Result<Self> {
        let path = path.into();
        let metadata = std::fs::symlink_metadata(&path)?;
        let file_type = metadata.file_type();

        let kind = if file_type.is_symlink() {
            FileKind::Symlink
        } else if file_type.is_dir() {
            FileKind::Directory
        } else if file_type.is_file() {
            FileKind::File
        } else {
            FileKind::Other
        };

        let name = path
            .file_name()
            .map(|name| name.to_string_lossy().into_owned())
            .unwrap_or_else(|| path.to_string_lossy().into_owned());

        let extension = path
            .extension()
            .map(|ext| ext.to_string_lossy().to_lowercase())
            .filter(|ext| !ext.is_empty());

        let mime = extension
            .as_deref()
            .and_then(|ext| mime_guess::from_ext(ext).first_raw())
            .map(str::to_owned);

        let size = if kind == FileKind::File {
            metadata.len()
        } else {
            0
        };

        Ok(Self {
            path,
            name,
            extension,
            mime,
            size,
            modified: metadata.modified().ok(),
            kind,
        })
    }

    /// Seconds since the Unix epoch, or `None` when the platform cannot say.
    pub fn modified_unix(&self) -> Option<i64> {
        self.modified
            .and_then(|time| time.duration_since(UNIX_EPOCH).ok())
            .map(|duration| duration.as_secs() as i64)
    }

    /// Age in days, used as a soft ranking signal rather than a filter.
    pub fn age_days(&self, now: SystemTime) -> Option<f64> {
        let modified = self.modified?;
        let age = now.duration_since(modified).ok()?;
        Some(age.as_secs_f64() / 86_400.0)
    }

    /// Coarse size bucket. Exact byte counts are noise for a classifier; the
    /// order of magnitude is the signal.
    pub fn size_bucket(&self) -> &'static str {
        match self.size {
            0 => "empty",
            1..=4_095 => "tiny",
            4_096..=65_535 => "small",
            65_536..=1_048_575 => "medium",
            1_048_576..=16_777_215 => "large",
            16_777_216..=268_435_455 => "huge",
            _ => "massive",
        }
    }

    pub fn is_hidden(&self) -> bool {
        self.name.starts_with('.') && self.name != "." && self.name != ".."
    }

    pub fn path(&self) -> &Path {
        &self.path
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn size_buckets_are_ordered() {
        let mut record = FileRecord {
            path: PathBuf::from("/tmp/example.bin"),
            name: "example.bin".into(),
            extension: Some("bin".into()),
            mime: None,
            size: 0,
            modified: None,
            kind: FileKind::File,
        };
        assert_eq!(record.size_bucket(), "empty");

        record.size = 4_096;
        assert_eq!(record.size_bucket(), "small");

        record.size = 300_000_000;
        assert_eq!(record.size_bucket(), "massive");
    }

    #[test]
    fn hidden_names_are_detected() {
        let record = FileRecord {
            path: PathBuf::from("/tmp/.gitignore"),
            name: ".gitignore".into(),
            extension: None,
            mime: None,
            size: 12,
            modified: None,
            kind: FileKind::File,
        };
        assert!(record.is_hidden());
    }
}
