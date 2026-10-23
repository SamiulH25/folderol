//! `folderol` — the headless side of the browser.
//!
//! The same catalog the GTK app reads is built here, which keeps indexing,
//! search and training testable without a display server.

use std::env;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, anyhow};
use clap::{Parser, Subcommand};
use folderol_core::features::feature_text;
use folderol_core::{Category, FileRecord};
use folderol_index::{Catalog, NewFile, SearchWeights};
use folderol_ml::{Classifier, Embedder, HashingEmbedder};
use walkdir::WalkDir;

#[derive(Parser, Debug)]
#[command(
    name = "folderol",
    version,
    about = "A file browser that learns where things belong",
    long_about = None
)]
struct Cli {
    /// Catalog location. Defaults to $XDG_DATA_HOME/folderol/index.sqlite3.
    #[arg(long, global = true, env = "FOLDEROL_DB", value_name = "PATH")]
    db: Option<PathBuf>,

    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand, Debug)]
enum Command {
    /// Walk one or more directories and index their files.
    Scan {
        /// Roots to scan. Defaults to the current directory.
        #[arg(default_value = ".")]
        paths: Vec<PathBuf>,

        /// Include dotfiles and dot-directories.
        #[arg(long)]
        include_hidden: bool,

        /// Stop after this many files (useful for a first look).
        #[arg(long)]
        limit: Option<usize>,
    },

    /// Search the catalog with words, meaning, or both.
    Search {
        query: String,

        #[arg(long, default_value_t = 20)]
        limit: usize,
    },

    /// Correct a file's category and retrain this user's classifier.
    Learn {
        path: PathBuf,

        /// Category slug, e.g. documents, images, code, data.
        #[arg(long)]
        category: String,
    },

    /// Count indexed files by category.
    Stats,

    /// List the category slugs accepted by `learn`.
    Categories,
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    let db = cli.db.unwrap_or_else(default_db_path).to_path_buf();

    match cli.command {
        Command::Scan {
            paths,
            include_hidden,
            limit,
        } => scan(&db, &paths, include_hidden, limit),
        Command::Search { query, limit } => search(&db, &query, limit),
        Command::Learn { path, category } => learn(&db, &path, &category),
        Command::Stats => stats(&db),
        Command::Categories => {
            for category in Category::ALL {
                println!("{:<12} {}", category.slug(), category.label());
            }
            Ok(())
        }
    }
}

fn scan(db: &Path, roots: &[PathBuf], include_hidden: bool, limit: Option<usize>) -> Result<()> {
    let mut catalog = Catalog::open(db).with_context(|| format!("opening {}", db.display()))?;
    let embedder = HashingEmbedder::default();
    let db_absolute = db.canonicalize().unwrap_or_else(|_| db.to_path_buf());

    let mut batch: Vec<NewFile> = Vec::with_capacity(512);
    let mut written = 0usize;

    'roots: for root in roots {
        // Index absolute paths so `scan .` and `scan ~/x` agree, and so a
        // later `learn relative/path` can find the row.
        let root = root.canonicalize().unwrap_or_else(|_| root.clone());
        let walker = WalkDir::new(&root)
            .follow_links(false)
            .into_iter()
            .filter_entry(|entry| {
                entry.depth() == 0 || include_hidden || !is_hidden_name(entry.file_name())
            });

        for entry in walker {
            let entry = match entry {
                Ok(entry) => entry,
                Err(error) => {
                    eprintln!("skipping unreadable entry: {error}");
                    continue;
                }
            };

            if !entry.file_type().is_file() {
                continue;
            }
            if entry.path() == db_absolute
                || entry
                    .path()
                    .canonicalize()
                    .map(|path| path == db_absolute)
                    .unwrap_or(false)
            {
                continue;
            }

            let record = match FileRecord::from_path(entry.path()) {
                Ok(record) => record,
                Err(error) => {
                    eprintln!("skipping {}: {error}", entry.path().display());
                    continue;
                }
            };

            let (category, confidence) = Category::guess(&record);
            let tokens = feature_text(&record);
            let embedding = embedder.embed(&tokens);

            batch.push(NewFile {
                record,
                category,
                confidence,
                embedding,
                tokens,
            });

            if batch.len() >= 512 {
                written += catalog.upsert_many(&batch)?;
                batch.clear();
                eprintln!("indexed {written} files...");
            }

            if let Some(limit) = limit
                && written + batch.len() >= limit
            {
                break 'roots;
            }
        }
    }

    written += catalog.upsert_many(&batch)?;
    catalog.set_meta("embedder_id", embedder.model_id())?;
    catalog.set_meta("embedding_dim", &embedder.dim().to_string())?;

    println!(
        "Indexed {written} files into {} ({} total).",
        db.display(),
        catalog.count()?
    );
    print_stats(&catalog)?;
    Ok(())
}

fn search(db: &Path, query: &str, limit: usize) -> Result<()> {
    let catalog = Catalog::open(db).with_context(|| format!("opening {}", db.display()))?;
    let embedder = HashingEmbedder::default();

    if let Some(indexed_with) = catalog.get_meta("embedder_id")?
        && indexed_with != embedder.model_id()
    {
        eprintln!(
            "warning: this catalog was built with {indexed_with}; run `folderol scan` to rebuild the vectors"
        );
    }

    let hits = catalog.search(query, &embedder, SearchWeights::default(), limit)?;

    if hits.is_empty() {
        println!("No matches for {query:?}.");
        return Ok(());
    }

    println!("{:<6} {:<12} {:<6} PATH", "SCORE", "CATEGORY", "MEANING");
    for hit in hits {
        println!(
            "{:<6.3} {:<12} {:<6.2} {}",
            hit.score,
            hit.file.category.label(),
            hit.semantic,
            hit.file.path.display()
        );
    }
    Ok(())
}

fn learn(db: &Path, path: &Path, category_slug: &str) -> Result<()> {
    let category = Category::from_slug(category_slug)
        .ok_or_else(|| anyhow!("unknown category {category_slug:?}; try `folderol categories`"))?;

    let mut catalog = Catalog::open(db).with_context(|| format!("opening {}", db.display()))?;

    let indexed_path = resolve_indexed_path(&catalog, path)?;
    catalog.set_label(&indexed_path, category)?;

    let labels = catalog.labels()?;
    if labels.len() < 3 {
        eprintln!(
            "warning: only {} label(s) so far; the classifier needs a handful per category to be useful",
            labels.len()
        );
    }

    let embedder = HashingEmbedder::default();
    let samples: Vec<(Vec<f32>, Category)> = labels
        .iter()
        .filter(|(file, _)| file.has_embedding())
        .map(|(file, category)| (file.embedding.clone(), *category))
        .collect();

    let mut classifier = Classifier::with_default_classes(embedder.dim())?;
    let report = classifier.train(&samples, 80, 0.35)?;

    let model_path = db.with_file_name("model.json");
    classifier.save(&model_path)?;

    // With only a label or two the model memorises the examples and would
    // relabel the whole catalog. Persist the correction, save the model, but
    // wait for enough evidence before letting it touch other files.
    let mut changed = 0usize;
    if samples.len() >= 3 {
        let files = catalog.all_with_embeddings()?;
        let mut predictions = Vec::with_capacity(files.len());
        for file in &files {
            if let Some((predicted, confidence)) = classifier.predict_top(&file.embedding) {
                if predicted != file.category {
                    changed += 1;
                }
                predictions.push((file.id, predicted, confidence));
            }
        }
        catalog.apply_predictions(&predictions)?;
    } else {
        eprintln!("keeping other categories unchanged until at least 3 labels exist");
    }

    println!(
        "Learned {:?} for {}. {} labels, {} training samples, loss {:.3}, {} files reclassified.",
        category.label(),
        indexed_path.display(),
        labels.len(),
        report.samples,
        report.final_loss,
        changed
    );
    println!("Model saved to {}.", model_path.display());
    Ok(())
}

fn stats(db: &Path) -> Result<()> {
    let catalog = Catalog::open(db).with_context(|| format!("opening {}", db.display()))?;
    println!("{} files in {}", catalog.count()?, db.display());
    print_stats(&catalog)
}

fn print_stats(catalog: &Catalog) -> Result<()> {
    for (category, count) in catalog.stats()? {
        println!("  {:<12} {count}", category.label());
    }
    Ok(())
}

fn resolve_indexed_path(catalog: &Catalog, path: &Path) -> Result<PathBuf> {
    if catalog.get_by_path(path)?.is_some() {
        return Ok(path.to_path_buf());
    }

    if let Ok(canonical) = path.canonicalize()
        && catalog.get_by_path(&canonical)?.is_some()
    {
        return Ok(canonical);
    }

    Err(anyhow!(
        "{} is not indexed; run `folderol scan` on its directory first",
        path.display()
    ))
}

fn default_db_path() -> PathBuf {
    if let Some(data_home) = env::var_os("XDG_DATA_HOME") {
        return PathBuf::from(data_home).join("folderol/index.sqlite3");
    }

    let home = env::var_os("HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("."));
    home.join(".local/share/folderol/index.sqlite3")
}

fn is_hidden_name(name: &std::ffi::OsStr) -> bool {
    name.to_string_lossy().starts_with('.')
}
