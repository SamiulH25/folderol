//! Domain types shared by every Folderol crate.
//!
//! This crate is deliberately free of I/O beyond stat-ing a path: the index,
//! the ML layer and the UI all agree on these types instead of inventing their
//! own.

pub mod category;
pub mod features;
pub mod file;

pub use category::Category;
pub use file::{FileKind, FileRecord};
