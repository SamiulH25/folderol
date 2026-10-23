//! Folderol's local intelligence layer.
//!
//! Two pieces live here:
//!
//! * [`Embedder`] — turns a file's feature text into a vector. The bundled
//!   [`HashingEmbedder`] is dependency-free and deterministic, which makes it a
//!   useful floor and a good test fixture. A pretrained ONNX model implements
//!   the same trait later (see `docs/adr/0002-local-embeddings.md`).
//! * [`Classifier`] — one-vs-rest logistic regression trained on this user's
//!   corrections. Training is online and takes milliseconds, so the model can
//!   update on every "this is a receipt" action.
//!
//! Nothing here opens a socket. Model files are local JSON, readable by hand.

pub mod classifier;
pub mod embedder;

pub use classifier::{Classifier, TrainReport};
pub use embedder::{Embedder, HashingEmbedder};

/// Cosine similarity, clamped to `-1.0..=1.0`.
pub fn cosine(a: &[f32], b: &[f32]) -> f32 {
    if a.len() != b.len() {
        return 0.0;
    }

    let mut dot = 0.0;
    let mut norm_a = 0.0;
    let mut norm_b = 0.0;

    for (x, y) in a.iter().zip(b) {
        dot += x * y;
        norm_a += x * x;
        norm_b += y * y;
    }

    if norm_a == 0.0 || norm_b == 0.0 {
        return 0.0;
    }

    (dot / (norm_a.sqrt() * norm_b.sqrt())).clamp(-1.0, 1.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cosine_of_identical_vectors_is_one() {
        let vector = vec![0.2, -0.4, 0.8];
        assert!((cosine(&vector, &vector) - 1.0).abs() < 1e-6);
    }

    #[test]
    fn cosine_of_orthogonal_vectors_is_zero() {
        assert_eq!(cosine(&[1.0, 0.0], &[0.0, 1.0]), 0.0);
    }

    #[test]
    fn mismatched_dimensions_score_zero() {
        assert_eq!(cosine(&[1.0], &[1.0, 2.0]), 0.0);
    }
}
