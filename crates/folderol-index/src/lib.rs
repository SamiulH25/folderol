//! The local catalog.
//!
//! One SQLite file holds everything Folderol knows: file metadata, vectors,
//! learned labels and (later) interaction events. It is ordinary SQLite, so a
//! curious user can open it with `sqlite3` and a backup is a file copy.
//!
//! Retrieval is hybrid from day one: FTS5 for exact words and vectors for
//! meaning, blended in [`Catalog::search`]. Brute-force cosine is intentional
//! at this stage; the index is designed so `sqlite-vec` can take over without a
//! schema change (see `docs/adr/0003-sqlite-catalog.md`).

use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use folderol_core::features::tokenize;
use folderol_core::{Category, FileRecord};
use folderol_ml::{Embedder, cosine};
use rusqlite::{Connection, OptionalExtension, params};

const SCHEMA_VERSION: i64 = 2;

const FILE_COLUMNS: &str = "files.id, files.path, files.name, files.ext, files.mime, files.size, files.modified, files.category, files.confidence, files.embedding";

#[derive(Debug, thiserror::Error)]
pub enum IndexError {
    #[error("sqlite: {0}")]
    Sqlite(#[from] rusqlite::Error),

    #[error("could not prepare the catalog directory: {0}")]
    Io(#[from] std::io::Error),

    #[error("no indexed file at {0}")]
    NotFound(String),

    #[error("embedding for file {0} is corrupt")]
    CorruptEmbedding(i64),
}

/// A file in the catalog, with its vector loaded.
#[derive(Debug, Clone)]
pub struct IndexedFile {
    pub id: i64,
    pub path: PathBuf,
    pub name: String,
    pub extension: Option<String>,
    pub mime: Option<String>,
    pub size: u64,
    pub modified: Option<i64>,
    pub category: Category,
    pub confidence: f32,
    pub embedding: Vec<f32>,
}

impl IndexedFile {
    pub fn has_embedding(&self) -> bool {
        !self.embedding.is_empty()
    }

    pub fn age_days(&self) -> Option<f64> {
        let modified = self.modified?;
        let now = SystemTime::now().duration_since(UNIX_EPOCH).ok()?.as_secs() as i64;
        let age_seconds = (now - modified).max(0) as f64;
        Some(age_seconds / 86_400.0)
    }
}

/// A row about to be written.
pub struct NewFile {
    pub record: FileRecord,
    pub category: Category,
    pub confidence: f32,
    pub embedding: Vec<f32>,
    pub tokens: String,
}

#[derive(Debug, Clone, Copy)]
pub struct SearchWeights {
    pub semantic: f32,
    pub lexical: f32,
    pub prior: f32,
    pub recency: f32,
}

impl Default for SearchWeights {
    fn default() -> Self {
        Self {
            semantic: 1.0,
            lexical: 0.9,
            prior: 0.15,
            recency: 0.05,
        }
    }
}

#[derive(Debug, Clone)]
pub struct SearchHit {
    pub file: IndexedFile,
    pub score: f32,
    pub lexical: f32,
    pub semantic: f32,
    pub recency: f32,
}

/// A catalog backed by a single SQLite file (or `:memory:` for tests).
pub struct Catalog {
    conn: Connection,
}

impl Catalog {
    pub fn open(path: impl AsRef<Path>) -> Result<Self, IndexError> {
        let path = path.as_ref();
        if let Some(parent) = path.parent()
            && !parent.as_os_str().is_empty()
        {
            fs::create_dir_all(parent)?;
        }

        let conn = Connection::open(path)?;
        Self::from_connection(conn)
    }

    pub fn open_in_memory() -> Result<Self, IndexError> {
        Self::from_connection(Connection::open_in_memory()?)
    }

    fn from_connection(conn: Connection) -> Result<Self, IndexError> {
        conn.execute_batch(
            "PRAGMA journal_mode = WAL;
             PRAGMA synchronous = NORMAL;
             PRAGMA foreign_keys = ON;
             PRAGMA busy_timeout = 5000;",
        )?;

        let mut catalog = Self { conn };
        catalog.migrate()?;
        Ok(catalog)
    }

    fn migrate(&mut self) -> Result<(), IndexError> {
        let version: i64 = self
            .conn
            .query_row("PRAGMA user_version", [], |row| row.get(0))?;

        if version < SCHEMA_VERSION {
            self.conn.execute_batch(&format!(
                "CREATE TABLE IF NOT EXISTS files (
                    id          INTEGER PRIMARY KEY,
                    path        TEXT NOT NULL UNIQUE,
                    name        TEXT NOT NULL,
                    ext         TEXT,
                    mime        TEXT,
                    size        INTEGER NOT NULL,
                    modified    INTEGER,
                    category    TEXT NOT NULL,
                    confidence  REAL NOT NULL,
                    embedding   BLOB,
                    indexed_at  INTEGER NOT NULL
                );

                CREATE INDEX IF NOT EXISTS files_category_idx ON files(category);
                CREATE INDEX IF NOT EXISTS files_modified_idx ON files(modified DESC);

                CREATE VIRTUAL TABLE IF NOT EXISTS files_fts USING fts5(
                    name,
                    path,
                    tokens,
                    tokenize = 'unicode61 remove_diacritics 2'
                );

                CREATE TABLE IF NOT EXISTS labels (
                    file_id    INTEGER NOT NULL REFERENCES files(id) ON DELETE CASCADE,
                    category   TEXT NOT NULL,
                    created_at INTEGER NOT NULL,
                    PRIMARY KEY (file_id)
                );

                CREATE TABLE IF NOT EXISTS events (
                    id      INTEGER PRIMARY KEY,
                    file_id INTEGER REFERENCES files(id) ON DELETE CASCADE,
                    kind    TEXT NOT NULL,
                    at      INTEGER NOT NULL
                );

                CREATE INDEX IF NOT EXISTS events_file_idx ON events(file_id);
                PRAGMA user_version = {SCHEMA_VERSION};"
            ))?;
        }

        if version < 2 {
            self.conn.execute_batch(
                "CREATE TABLE IF NOT EXISTS meta (
                    key   TEXT PRIMARY KEY,
                    value TEXT NOT NULL
                );
                PRAGMA user_version = 2;",
            )?;
        }

        Ok(())
    }

    pub fn set_meta(&self, key: &str, value: &str) -> Result<(), IndexError> {
        self.conn.execute(
            "INSERT INTO meta (key, value) VALUES (?1, ?2)
             ON CONFLICT(key) DO UPDATE SET value = excluded.value",
            params![key, value],
        )?;
        Ok(())
    }

    pub fn get_meta(&self, key: &str) -> Result<Option<String>, IndexError> {
        Ok(self
            .conn
            .query_row(
                "SELECT value FROM meta WHERE key = ?1",
                params![key],
                |row| row.get(0),
            )
            .optional()?)
    }

    /// Insert or update files in one transaction. Returns the number written.
    pub fn upsert_many(&mut self, files: &[NewFile]) -> Result<usize, IndexError> {
        if files.is_empty() {
            return Ok(0);
        }

        let transaction = self.conn.transaction()?;
        {
            let mut upsert = transaction.prepare(
                "INSERT INTO files
                    (path, name, ext, mime, size, modified, category, confidence, embedding, indexed_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)
                 ON CONFLICT(path) DO UPDATE SET
                    name = excluded.name,
                    ext = excluded.ext,
                    mime = excluded.mime,
                    size = excluded.size,
                    modified = excluded.modified,
                    -- A human correction outlives a rescan. The subquery reads
                    -- the label row that was written for this file, if any.
                    category = COALESCE(
                        (SELECT labels.category FROM labels WHERE labels.file_id = files.id),
                        excluded.category
                    ),
                    confidence = CASE
                        WHEN EXISTS (SELECT 1 FROM labels WHERE labels.file_id = files.id)
                            THEN 1.0
                        ELSE excluded.confidence
                    END,
                    embedding = excluded.embedding,
                    indexed_at = excluded.indexed_at
                 RETURNING id",
            )?;

            let mut clear_fts = transaction.prepare("DELETE FROM files_fts WHERE rowid = ?1")?;
            let mut insert_fts = transaction.prepare(
                "INSERT INTO files_fts (rowid, name, path, tokens) VALUES (?1, ?2, ?3, ?4)",
            )?;

            let now = unix_now();

            for file in files {
                let id: i64 = upsert.query_row(
                    params![
                        file.record.path.to_string_lossy(),
                        file.record.name,
                        file.record.extension,
                        file.record.mime,
                        file.record.size as i64,
                        file.record.modified_unix(),
                        file.category.slug(),
                        file.confidence,
                        encode_embedding(&file.embedding),
                        now,
                    ],
                    |row| row.get(0),
                )?;

                clear_fts.execute(params![id])?;
                insert_fts.execute(params![
                    id,
                    file.record.name,
                    file.record.path.to_string_lossy(),
                    file.tokens
                ])?;
            }
        }
        transaction.commit()?;

        Ok(files.len())
    }

    pub fn count(&self) -> Result<usize, IndexError> {
        let count: i64 = self
            .conn
            .query_row("SELECT COUNT(*) FROM files", [], |row| row.get(0))?;
        Ok(count as usize)
    }

    pub fn stats(&self) -> Result<Vec<(Category, usize)>, IndexError> {
        let mut statement = self.conn.prepare(
            "SELECT category, COUNT(*) FROM files GROUP BY category ORDER BY COUNT(*) DESC",
        )?;
        let rows = statement.query_map([], |row| {
            let slug: String = row.get(0)?;
            let count: i64 = row.get(1)?;
            Ok((
                Category::from_slug(&slug).unwrap_or(Category::Other),
                count as usize,
            ))
        })?;

        let mut stats = Vec::new();
        for row in rows {
            stats.push(row?);
        }
        Ok(stats)
    }

    pub fn get_by_path(&self, path: &Path) -> Result<Option<IndexedFile>, IndexError> {
        let sql = format!("SELECT {FILE_COLUMNS} FROM files WHERE path = ?1");
        self.conn
            .query_row(&sql, params![path.to_string_lossy()], row_to_file)
            .optional()?
            .transpose()
    }

    pub fn all(&self, limit: usize) -> Result<Vec<IndexedFile>, IndexError> {
        let sql =
            format!("SELECT {FILE_COLUMNS} FROM files ORDER BY modified DESC NULLS LAST LIMIT ?1");
        let mut statement = self.conn.prepare(&sql)?;
        let rows = statement.query_map(params![limit as i64], row_to_file)?;

        let mut files = Vec::new();
        for row in rows {
            files.push(row??);
        }
        Ok(files)
    }

    pub fn all_with_embeddings(&self) -> Result<Vec<IndexedFile>, IndexError> {
        let sql = format!("SELECT {FILE_COLUMNS} FROM files WHERE embedding IS NOT NULL");
        let mut statement = self.conn.prepare(&sql)?;
        let rows = statement.query_map([], row_to_file)?;

        let mut files = Vec::new();
        for row in rows {
            files.push(row??);
        }
        Ok(files)
    }

    /// Update the predicted category for one file.
    pub fn update_prediction(
        &self,
        id: i64,
        category: Category,
        confidence: f32,
    ) -> Result<(), IndexError> {
        self.conn.execute(
            "UPDATE files SET category = ?1, confidence = ?2 WHERE id = ?3",
            params![category.slug(), confidence, id],
        )?;
        Ok(())
    }

    pub fn apply_predictions(
        &mut self,
        predictions: &[(i64, Category, f32)],
    ) -> Result<usize, IndexError> {
        let transaction = self.conn.transaction()?;
        {
            let mut update = transaction
                .prepare("UPDATE files SET category = ?1, confidence = ?2 WHERE id = ?3")?;
            for (id, category, confidence) in predictions {
                update.execute(params![category.slug(), confidence, id])?;
            }
        }
        transaction.commit()?;
        Ok(predictions.len())
    }

    /// Record an explicit correction. One label per file keeps the model
    /// honest: the most recent human answer wins.
    pub fn set_label(&self, path: &Path, category: Category) -> Result<(), IndexError> {
        let file_id: i64 = self
            .conn
            .query_row(
                "SELECT id FROM files WHERE path = ?1",
                params![path.to_string_lossy()],
                |row| row.get(0),
            )
            .optional()?
            .ok_or_else(|| IndexError::NotFound(path.display().to_string()))?;

        self.conn.execute(
            "INSERT INTO labels (file_id, category, created_at) VALUES (?1, ?2, ?3)
             ON CONFLICT(file_id) DO UPDATE SET category = excluded.category, created_at = excluded.created_at",
            params![file_id, category.slug(), unix_now()],
        )?;

        self.update_prediction(file_id, category, 1.0)?;
        Ok(())
    }

    pub fn labels(&self) -> Result<Vec<(IndexedFile, Category)>, IndexError> {
        let sql = format!(
            "SELECT {FILE_COLUMNS}, labels.category
             FROM files JOIN labels ON labels.file_id = files.id
             ORDER BY labels.created_at DESC"
        );
        let mut statement = self.conn.prepare(&sql)?;
        let rows = statement.query_map([], |row| {
            let file = row_to_file(row)?;
            let slug: String = row.get(10)?;
            Ok((file, slug))
        })?;

        let mut labels = Vec::new();
        for row in rows {
            let (file, slug) = row?;
            let file = file?;
            labels.push((file, Category::from_slug(&slug).unwrap_or(Category::Other)));
        }
        Ok(labels)
    }

    /// Hybrid retrieval.
    ///
    /// Lexical ranking comes from FTS5; meaning comes from cosine similarity
    /// against the query embedding. The category confidence stored at index
    /// time acts as a light prior, and recency as a tie-breaker. Every
    /// component is returned so the UI can explain a result.
    pub fn search(
        &self,
        query: &str,
        embedder: &dyn Embedder,
        weights: SearchWeights,
        limit: usize,
    ) -> Result<Vec<SearchHit>, IndexError> {
        let tokens = tokenize(query);
        if tokens.is_empty() {
            return Ok(Vec::new());
        }

        let fts_query = tokens
            .iter()
            .map(|token| format!("\"{}\"*", token.replace('"', "")))
            .collect::<Vec<_>>()
            .join(" OR ");

        let mut lexical_scores: HashMap<i64, f32> = HashMap::new();
        {
            let mut statement = self.conn.prepare(
                "SELECT rowid, bm25(files_fts) FROM files_fts
                 WHERE files_fts MATCH ?1
                 ORDER BY bm25(files_fts)
                 LIMIT 400",
            )?;
            let mut rows = statement.query(params![fts_query])?;
            let mut rank = 0usize;
            while let Some(row) = rows.next()? {
                let id: i64 = row.get(0)?;
                // Rank-based normalization is robust to bm25's unbounded scale.
                lexical_scores.insert(id, 1.0 / (1.0 + rank as f32 * 0.25));
                rank += 1;
            }
        }

        let query_embedding = embedder.embed_query(query);
        let semantic_floor = embedder.semantic_floor();
        let files = self.all_with_embeddings()?;

        let mut hits = Vec::new();
        for file in files {
            let lexical = lexical_scores.get(&file.id).copied().unwrap_or(0.0);
            let semantic = if file.embedding.len() == query_embedding.len() {
                cosine(&query_embedding, &file.embedding).max(0.0)
            } else {
                0.0
            };

            if lexical == 0.0 && semantic < semantic_floor {
                continue;
            }

            let recency = file
                .age_days()
                .map(|days| 1.0 / (1.0 + days / 90.0))
                .unwrap_or(0.0) as f32;

            let score = weights.semantic * semantic
                + weights.lexical * lexical
                + weights.prior * file.confidence
                + weights.recency * recency;

            hits.push(SearchHit {
                file,
                score,
                lexical,
                semantic,
                recency,
            });
        }

        hits.sort_by(|left, right| {
            right
                .score
                .total_cmp(&left.score)
                .then_with(|| left.file.path.cmp(&right.file.path))
        });
        hits.truncate(limit);

        Ok(hits)
    }
}

fn unix_now() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs() as i64)
        .unwrap_or(0)
}

fn encode_embedding(values: &[f32]) -> Vec<u8> {
    let mut bytes = Vec::with_capacity(values.len() * 4);
    for value in values {
        bytes.extend_from_slice(&value.to_le_bytes());
    }
    bytes
}

fn decode_embedding(id: i64, blob: Option<Vec<u8>>) -> Result<Vec<f32>, IndexError> {
    let blob = match blob {
        Some(blob) => blob,
        None => return Ok(Vec::new()),
    };
    let (chunks, remainder) = blob.as_chunks::<4>();
    if !remainder.is_empty() {
        return Err(IndexError::CorruptEmbedding(id));
    }

    Ok(chunks
        .iter()
        .map(|chunk| f32::from_le_bytes(*chunk))
        .collect())
}

/// Reads a row in `FILE_COLUMNS` order. The outer `Result` is rusqlite's,
/// the inner one reports a corrupt embedding blob.
fn row_to_file(row: &rusqlite::Row<'_>) -> rusqlite::Result<Result<IndexedFile, IndexError>> {
    let id: i64 = row.get(0)?;
    let path: String = row.get(1)?;
    let name: String = row.get(2)?;
    let extension: Option<String> = row.get(3)?;
    let mime: Option<String> = row.get(4)?;
    let size: i64 = row.get(5)?;
    let modified: Option<i64> = row.get(6)?;
    let category: String = row.get(7)?;
    let confidence: f32 = row.get(8)?;
    let embedding: Option<Vec<u8>> = row.get(9)?;

    Ok(
        decode_embedding(id, embedding).map(|embedding| IndexedFile {
            id,
            path: PathBuf::from(path),
            name,
            extension,
            mime,
            size: size.max(0) as u64,
            modified,
            category: Category::from_slug(&category).unwrap_or(Category::Other),
            confidence,
            embedding,
        }),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use folderol_core::FileKind;
    use folderol_ml::HashingEmbedder;

    fn new_file(path: &str, category: Category) -> NewFile {
        let record = FileRecord {
            path: PathBuf::from(path),
            name: Path::new(path)
                .file_name()
                .unwrap()
                .to_string_lossy()
                .into_owned(),
            extension: Path::new(path)
                .extension()
                .map(|ext| ext.to_string_lossy().into_owned()),
            mime: None,
            size: 1_024,
            modified: Some(SystemTime::now()),
            kind: FileKind::File,
        };
        let tokens = folderol_core::features::feature_text(&record);
        let embedding = HashingEmbedder::default().embed(&tokens);

        NewFile {
            record,
            category,
            confidence: 0.9,
            embedding,
            tokens,
        }
    }

    fn catalog_with_files() -> Catalog {
        let mut catalog = Catalog::open_in_memory().expect("catalog");
        catalog
            .upsert_many(&[
                new_file(
                    "/home/sam/Finance/Invoice_Acme_2024.pdf",
                    Category::Documents,
                ),
                new_file("/home/sam/Pictures/holiday-beach.jpg", Category::Images),
                new_file("/home/sam/Code/folderol/src/main.rs", Category::Code),
            ])
            .expect("insert");
        catalog
    }

    #[test]
    fn upsert_is_idempotent_and_updates_rows() {
        let mut catalog = catalog_with_files();
        assert_eq!(catalog.count().unwrap(), 3);

        let mut updated = new_file(
            "/home/sam/Finance/Invoice_Acme_2024.pdf",
            Category::Documents,
        );
        updated.record.size = 99_999;
        catalog.upsert_many(&[updated]).unwrap();

        assert_eq!(catalog.count().unwrap(), 3);
        let file = catalog
            .get_by_path(Path::new("/home/sam/Finance/Invoice_Acme_2024.pdf"))
            .unwrap()
            .expect("file");
        assert_eq!(file.size, 99_999);
    }

    #[test]
    fn lexical_and_semantic_search_find_the_invoice() {
        let catalog = catalog_with_files();
        let embedder = HashingEmbedder::default();
        let hits = catalog
            .search("invoice", &embedder, SearchWeights::default(), 5)
            .unwrap();

        assert!(!hits.is_empty());
        assert_eq!(hits[0].file.name, "Invoice_Acme_2024.pdf");
        assert!(hits[0].lexical > 0.0);
    }

    #[test]
    fn labels_round_trip() {
        let catalog = catalog_with_files();
        let path = Path::new("/home/sam/Pictures/holiday-beach.jpg");
        catalog.set_label(path, Category::Documents).unwrap();

        let labels = catalog.labels().unwrap();
        assert_eq!(labels.len(), 1);
        assert_eq!(labels[0].1, Category::Documents);

        let file = catalog.get_by_path(path).unwrap().expect("file");
        assert_eq!(file.category, Category::Documents);
    }

    #[test]
    fn stats_are_grouped_by_category() {
        let catalog = catalog_with_files();
        let stats = catalog.stats().unwrap();
        let total: usize = stats.iter().map(|(_, count)| count).sum();
        assert_eq!(total, 3);
    }

    #[test]
    fn a_scan_does_not_overwrite_a_human_label() {
        let mut catalog = catalog_with_files();
        let path = Path::new("/home/sam/Pictures/holiday-beach.jpg");
        catalog.set_label(path, Category::Documents).unwrap();

        // Re-index the same file with its extension prior (Images).
        catalog
            .upsert_many(&[new_file(
                "/home/sam/Pictures/holiday-beach.jpg",
                Category::Images,
            )])
            .unwrap();

        let file = catalog.get_by_path(path).unwrap().expect("file");
        assert_eq!(file.category, Category::Documents);
        assert_eq!(file.confidence, 1.0);
    }

    #[test]
    fn meta_round_trips() {
        let catalog = catalog_with_files();
        assert_eq!(catalog.get_meta("embedder_id").unwrap(), None);
        catalog.set_meta("embedder_id", "hashing-v1").unwrap();
        assert_eq!(
            catalog.get_meta("embedder_id").unwrap().as_deref(),
            Some("hashing-v1")
        );
    }

    #[test]
    fn searching_for_an_unknown_word_returns_nothing() {
        let catalog = catalog_with_files();
        let embedder = HashingEmbedder::default();
        let hits = catalog
            .search("zzzzqqqq", &embedder, SearchWeights::default(), 5)
            .unwrap();
        assert!(hits.is_empty());
    }
}
