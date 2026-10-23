use folderol_core::FileRecord;
use folderol_core::features::tokenize;

/// Turns feature text into a vector of fixed dimension.
///
/// Implementations must be deterministic for a given text and dimension: the
/// index stores vectors forever, and a changing embedder silently breaks
/// similarity search.
pub trait Embedder: Send + Sync {
    fn dim(&self) -> usize;

    /// Embed already-tokenized, already-normalized text.
    fn embed(&self, text: &str) -> Vec<f32>;

    /// Convenience wrapper used by callers that have a [`FileRecord`].
    fn embed_record(&self, record: &FileRecord) -> Vec<f32> {
        self.embed(&folderol_core::features::feature_text(record))
    }

    /// Stable identifier written into the index so a model swap can invalidate
    /// stale vectors instead of comparing incompatible spaces.
    fn model_id(&self) -> &str;

    fn embed_query(&self, query: &str) -> Vec<f32> {
        self.embed(query)
    }

    /// Minimum cosine similarity for a hit that has no lexical support.
    ///
    /// Sparse hashing vectors collide by construction, so this baseline needs a
    /// high bar to avoid returning nonsense for an unknown word. A dense
    /// pretrained model can use a much lower one.
    fn semantic_floor(&self) -> f32 {
        0.20
    }
}

/// A deterministic hashing embedder (a.k.a. the "hashing trick").
///
/// Each token is hashed into one of `dim` buckets with a signed second hash to
/// keep collisions from always adding. Term frequencies are compressed with
/// `ln(1 + tf)` and the result is L2-normalized, which is the same shape a
/// sentence-transformer produces.
///
/// This is not a neural network. It is the baseline that proves the pipeline
/// and the fallback that works with zero downloads. The semantic quality step
/// up arrives with the ONNX embedder in milestone 2.
#[derive(Debug, Clone)]
pub struct HashingEmbedder {
    dim: usize,
}

impl HashingEmbedder {
    pub const DEFAULT_DIM: usize = 384;

    pub fn new(dim: usize) -> Self {
        assert!(dim > 0, "embedding dimension must be positive");
        Self { dim }
    }
}

impl Default for HashingEmbedder {
    fn default() -> Self {
        Self::new(Self::DEFAULT_DIM)
    }
}

impl Embedder for HashingEmbedder {
    fn dim(&self) -> usize {
        self.dim
    }

    fn embed(&self, text: &str) -> Vec<f32> {
        let mut counts = vec![0.0f32; self.dim];

        for token in tokenize(text) {
            let bytes = token.as_bytes();
            let first = fnv1a(bytes);
            let second = fnv1a_seeded(bytes, 0x9e37_79b9_7f4a_7c15);

            let index = (first % self.dim as u64) as usize;
            let sign = if second & 1 == 0 { 1.0 } else { -1.0 };
            counts[index] += sign;
        }

        let mut vector: Vec<f32> = counts
            .into_iter()
            .map(|count| count.signum() * (1.0 + count.abs()).ln())
            .collect();

        let norm = vector.iter().map(|value| value * value).sum::<f32>().sqrt();
        if norm > 0.0 {
            for value in &mut vector {
                *value /= norm;
            }
        }

        vector
    }

    fn model_id(&self) -> &str {
        "hashing-v1"
    }

    fn semantic_floor(&self) -> f32 {
        0.50
    }
}

const FNV_OFFSET: u64 = 0xcbf2_9ce4_8422_2325;
const FNV_PRIME: u64 = 0x0000_0100_0000_01b3;

fn fnv1a(bytes: &[u8]) -> u64 {
    let mut hash = FNV_OFFSET;
    for byte in bytes {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(FNV_PRIME);
    }
    hash
}

fn fnv1a_seeded(bytes: &[u8], seed: u64) -> u64 {
    let mut hash = FNV_OFFSET ^ seed;
    for byte in bytes {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(FNV_PRIME);
    }
    hash
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn embeddings_are_normalized_and_deterministic() {
        let embedder = HashingEmbedder::default();
        let first = embedder.embed("invoice acme 2024 finance pdf");
        let second = embedder.embed("invoice acme 2024 finance pdf");

        assert_eq!(first, second);
        let norm = first.iter().map(|value| value * value).sum::<f32>().sqrt();
        assert!((norm - 1.0).abs() < 1e-5);
    }

    #[test]
    fn similar_text_scores_higher_than_unrelated_text() {
        let embedder = HashingEmbedder::default();
        let invoice = embedder.embed("invoice acme finance 2024 pdf");
        let receipt = embedder.embed("receipt acme finance 2024 pdf");
        let photo = embedder.embed("vacation beach photo jpeg camera");

        let near = crate::cosine(&invoice, &receipt);
        let far = crate::cosine(&invoice, &photo);
        assert!(near > far, "near={near} far={far}");
    }

    #[test]
    fn empty_text_produces_a_zero_vector() {
        let embedder = HashingEmbedder::default();
        let vector = embedder.embed("");
        assert!(vector.iter().all(|value| *value == 0.0));
    }
}
