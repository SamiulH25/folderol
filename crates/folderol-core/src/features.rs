//! The text representation of a file.
//!
//! Embeddings are only as good as what you feed them. For a file browser the
//! useful signal is not the file's bytes but its *name*, the words in its path,
//! its type and its rough size/age. That is what this module builds.

use crate::file::{FileKind, FileRecord};

/// Lowercase, split camelCase and PascalCase, and split on every non-alphanumeric
/// character. Tokens shorter than two characters are dropped because single
/// letters add collisions without adding meaning.
pub fn tokenize(input: &str) -> Vec<String> {
    let mut tokens = Vec::new();
    let mut current = String::new();
    let mut previous: Option<char> = None;

    for ch in input.chars() {
        if !ch.is_alphanumeric() {
            push_token(&mut tokens, &mut current);
            previous = None;
            continue;
        }

        let starts_new_word = ch.is_uppercase()
            && previous
                .map(|prev| prev.is_lowercase() || prev.is_ascii_digit())
                .unwrap_or(false);

        if starts_new_word {
            push_token(&mut tokens, &mut current);
        }

        current.extend(ch.to_lowercase());
        previous = Some(ch);
    }

    push_token(&mut tokens, &mut current);
    tokens
}

fn push_token(tokens: &mut Vec<String>, current: &mut String) {
    if current.chars().count() >= 2 && current.chars().count() <= 64 {
        tokens.push(std::mem::take(current));
    } else {
        current.clear();
    }
}

/// Build the string handed to the embedder.
///
/// The basename is repeated because it carries more intent than the directory
/// it happens to sit in. Nothing here reads file content, so scanning a million
/// files never depends on a PDF parser.
pub fn feature_text(record: &FileRecord) -> String {
    let mut parts: Vec<String> = Vec::new();

    // Path words, including the basename twice (once here, once below).
    for component in record.path.components() {
        match component {
            std::path::Component::RootDir
            | std::path::Component::Prefix(_)
            | std::path::Component::CurDir => continue,
            _ => {
                let text = component.as_os_str().to_string_lossy();
                parts.extend(tokenize(&text));
            }
        }
    }
    parts.extend(tokenize(&record.name));

    if let Some(ext) = &record.extension {
        parts.push(format!("ext:{ext}"));
    }
    if let Some(mime) = &record.mime {
        parts.push(format!("mime:{}", mime.replace('/', ".")));
    }

    parts.push(match record.kind {
        FileKind::File => "kind:file".to_string(),
        FileKind::Directory => "kind:directory".to_string(),
        FileKind::Symlink => "kind:symlink".to_string(),
        FileKind::Other => "kind:other".to_string(),
    });
    parts.push(format!("size:{}", record.size_bucket()));

    if let Some(age) = record.age_days(std::time::SystemTime::now()) {
        let bucket = if age < 1.0 {
            "today"
        } else if age < 7.0 {
            "week"
        } else if age < 31.0 {
            "month"
        } else if age < 365.0 {
            "year"
        } else {
            "old"
        };
        parts.push(format!("age:{bucket}"));
    }

    parts.join(" ")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::file::{FileKind, FileRecord};
    use std::path::PathBuf;

    #[test]
    fn tokenizer_splits_paths_and_camel_case() {
        assert_eq!(
            tokenize("2024/TaxReturn-final_v2.pdf"),
            vec!["2024", "tax", "return", "final", "v2", "pdf"]
        );
    }

    #[test]
    fn tokenizer_keeps_unicode() {
        let tokens = tokenize("Rechnung_Übersicht.md");
        assert!(tokens.contains(&"übersicht".to_string()));
    }

    #[test]
    fn feature_text_contains_type_and_size() {
        let record = FileRecord {
            path: PathBuf::from("/home/sam/Finance/2024/Invoice_Acme.pdf"),
            name: "Invoice_Acme.pdf".into(),
            extension: Some("pdf".into()),
            mime: Some("application/pdf".into()),
            size: 42_000,
            modified: None,
            kind: FileKind::File,
        };

        let text = feature_text(&record);
        assert!(text.contains("invoice"));
        assert!(text.contains("finance"));
        assert!(text.contains("ext:pdf"));
        assert!(text.contains("size:small"));
    }
}
