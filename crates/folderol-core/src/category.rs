use serde::{Deserialize, Serialize};

use crate::file::{FileKind, FileRecord};

/// The seeded taxonomy.
///
/// These are *priors*, not a prison: the classifier can learn that this user
/// keeps invoices under `Documents/Finance` or that `.blend` files belong with
/// projects rather than 3D assets. Adding a variant here is cheap; the slug is
/// the stable identifier written to the index.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Category {
    Documents,
    Images,
    Audio,
    Video,
    Code,
    Archives,
    Data,
    Config,
    Fonts,
    ThreeD,
    Executables,
    Other,
}

impl Category {
    pub const ALL: [Category; 12] = [
        Category::Documents,
        Category::Images,
        Category::Audio,
        Category::Video,
        Category::Code,
        Category::Archives,
        Category::Data,
        Category::Config,
        Category::Fonts,
        Category::ThreeD,
        Category::Executables,
        Category::Other,
    ];

    pub fn slug(self) -> &'static str {
        match self {
            Category::Documents => "documents",
            Category::Images => "images",
            Category::Audio => "audio",
            Category::Video => "video",
            Category::Code => "code",
            Category::Archives => "archives",
            Category::Data => "data",
            Category::Config => "config",
            Category::Fonts => "fonts",
            Category::ThreeD => "3d",
            Category::Executables => "executables",
            Category::Other => "other",
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Category::Documents => "Documents",
            Category::Images => "Images",
            Category::Audio => "Audio",
            Category::Video => "Video",
            Category::Code => "Code",
            Category::Archives => "Archives",
            Category::Data => "Data",
            Category::Config => "Config",
            Category::Fonts => "Fonts",
            Category::ThreeD => "3D",
            Category::Executables => "Executables",
            Category::Other => "Other",
        }
    }

    pub fn from_slug(slug: &str) -> Option<Self> {
        Self::ALL
            .into_iter()
            .find(|category| category.slug().eq_ignore_ascii_case(slug))
    }

    /// A first guess from the extension and MIME type, with a confidence in
    /// `0.0..=1.0`. This is the prior the learned classifier blends against,
    /// never the final answer.
    pub fn guess(record: &FileRecord) -> (Self, f32) {
        if record.kind == FileKind::Directory {
            return (Category::Other, 0.20);
        }

        if let Some(ext) = record.extension.as_deref()
            && let Some(category) = from_extension(ext)
        {
            return (category, 0.92);
        }

        if let Some(mime) = record.mime.as_deref()
            && let Some(category) = from_mime(mime)
        {
            return (category, 0.70);
        }

        (Category::Other, 0.25)
    }
}

fn from_extension(ext: &str) -> Option<Category> {
    let category = match ext {
        "pdf" | "doc" | "docx" | "odt" | "rtf" | "txt" | "md" | "markdown" | "epub" | "tex"
        | "pages" | "djvu" => Category::Documents,

        "png" | "jpg" | "jpeg" | "gif" | "webp" | "svg" | "bmp" | "tif" | "tiff" | "heic"
        | "avif" | "raw" | "cr2" | "nef" | "psd" | "xcf" => Category::Images,

        "mp3" | "flac" | "wav" | "ogg" | "opus" | "m4a" | "aac" | "aiff" | "wma" | "mid"
        | "midi" => Category::Audio,

        "mp4" | "mkv" | "webm" | "avi" | "mov" | "m4v" | "wmv" | "flv" | "mpg" | "mpeg" => {
            Category::Video
        }

        "rs" | "py" | "js" | "mjs" | "cjs" | "ts" | "tsx" | "jsx" | "go" | "c" | "h" | "cc"
        | "cpp" | "cxx" | "hpp" | "java" | "kt" | "kts" | "rb" | "php" | "lua" | "sh" | "bash"
        | "zsh" | "fish" | "sql" | "html" | "htm" | "css" | "scss" | "sass" | "vue" | "svelte"
        | "swift" | "zig" | "nim" | "ex" | "exs" | "erl" | "hs" | "ml" | "clj" | "gd" | "glsl"
        | "wgsl" | "asm" | "ipynb" => Category::Code,

        "zip" | "tar" | "gz" | "tgz" | "bz2" | "xz" | "7z" | "rar" | "zst" | "lz4" | "cab"
        | "iso" => Category::Archives,

        "json" | "jsonl" | "yaml" | "yml" | "toml" | "csv" | "tsv" | "parquet" | "arrow"
        | "feather" | "sqlite" | "sqlite3" | "db" | "xml" | "ndjson" => Category::Data,

        "conf" | "cfg" | "ini" | "env" | "service" | "desktop" | "properties" | "lock" => {
            Category::Config
        }

        "ttf" | "otf" | "woff" | "woff2" | "pfb" | "pfm" => Category::Fonts,

        "blend" | "obj" | "fbx" | "stl" | "gltf" | "glb" | "usd" | "usdz" | "ply" | "3ds"
        | "dae" | "step" | "stp" => Category::ThreeD,

        "appimage" | "exe" | "msi" | "dll" | "so" | "dylib" | "deb" | "rpm" | "apk" | "flatpak"
        | "snap" => Category::Executables,

        _ => return None,
    };
    Some(category)
}

fn from_mime(mime: &str) -> Option<Category> {
    if mime.starts_with("image/") {
        Some(Category::Images)
    } else if mime.starts_with("audio/") {
        Some(Category::Audio)
    } else if mime.starts_with("video/") {
        Some(Category::Video)
    } else if mime.starts_with("text/") {
        Some(Category::Documents)
    } else if mime.starts_with("font/") {
        Some(Category::Fonts)
    } else if mime.contains("zip")
        || mime.contains("tar")
        || mime.contains("compressed")
        || mime.contains("archive")
    {
        Some(Category::Archives)
    } else if mime.starts_with("application/x-executable")
        || mime.starts_with("application/x-sharedlib")
    {
        Some(Category::Executables)
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::file::{FileKind, FileRecord};
    use std::path::PathBuf;

    fn record(name: &str) -> FileRecord {
        let path = PathBuf::from("/tmp").join(name);
        FileRecord {
            path,
            name: name.to_string(),
            extension: name.rsplit_once('.').map(|(_, ext)| ext.to_lowercase()),
            mime: None,
            size: 10,
            modified: None,
            kind: FileKind::File,
        }
    }

    #[test]
    fn extensions_map_to_categories() {
        assert_eq!(Category::guess(&record("notes.md")).0, Category::Documents);
        assert_eq!(Category::guess(&record("photo.HEIC")).0, Category::Images);
        assert_eq!(Category::guess(&record("main.rs")).0, Category::Code);
        assert_eq!(
            Category::guess(&record("archive.tar.gz")).0,
            Category::Archives
        );
        assert_eq!(Category::guess(&record("model.glb")).0, Category::ThreeD);
        assert_eq!(Category::guess(&record("mystery.xyz")).0, Category::Other);
    }

    #[test]
    fn slugs_round_trip() {
        for category in Category::ALL {
            assert_eq!(Category::from_slug(category.slug()), Some(category));
        }
        assert_eq!(Category::from_slug("3D"), Some(Category::ThreeD));
    }

    #[test]
    fn directories_are_never_confident() {
        let mut folder = record("Projects");
        folder.kind = FileKind::Directory;
        folder.extension = None;
        let (category, confidence) = Category::guess(&folder);
        assert_eq!(category, Category::Other);
        assert!(confidence < 0.5);
    }
}
