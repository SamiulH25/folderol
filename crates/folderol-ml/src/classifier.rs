use std::fs;
use std::path::Path;

use folderol_core::Category;
use serde::{Deserialize, Serialize};

use crate::cosine;

#[derive(Debug, thiserror::Error)]
pub enum MlError {
    #[error("embedding has dimension {actual}, classifier expects {expected}")]
    DimensionMismatch { expected: usize, actual: usize },

    #[error("classifier needs at least one class")]
    NoClasses,

    #[error("classifier needs at least one training sample")]
    NoSamples,

    #[error("model file is invalid: {0}")]
    InvalidModel(String),

    #[error("could not read or write the model: {0}")]
    Io(#[from] std::io::Error),

    #[error("could not decode the model: {0}")]
    Json(#[from] serde_json::Error),
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TrainReport {
    pub epochs: u32,
    pub samples: usize,
    pub final_loss: f32,
}

/// One-vs-rest logistic regression over category embeddings.
///
/// Deliberately small: a few thousand floats, JSON-serializable, trainable in
/// milliseconds. The point is not to beat a transformer; it is to learn *this
/// user's* filing habits and to explain every prediction.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Classifier {
    dim: usize,
    classes: Vec<Category>,
    /// `weights[class][feature]`.
    weights: Vec<Vec<f32>>,
    biases: Vec<f32>,
}

impl Classifier {
    pub fn new(dim: usize, classes: Vec<Category>) -> Result<Self, MlError> {
        if dim == 0 {
            return Err(MlError::DimensionMismatch {
                expected: 1,
                actual: 0,
            });
        }
        if classes.is_empty() {
            return Err(MlError::NoClasses);
        }

        let class_count = classes.len();
        Ok(Self {
            dim,
            classes,
            weights: vec![vec![0.0; dim]; class_count],
            biases: vec![0.0; class_count],
        })
    }

    pub fn with_default_classes(dim: usize) -> Result<Self, MlError> {
        Self::new(dim, Category::ALL.to_vec())
    }

    pub fn dim(&self) -> usize {
        self.dim
    }

    pub fn classes(&self) -> &[Category] {
        &self.classes
    }

    pub fn sample_count(&self) -> usize {
        0
    }

    /// Train with stochastic gradient descent on the logistic loss.
    ///
    /// Samples may repeat between calls; the classifier is incremental by
    /// construction, though the CLI simply retrains from all stored labels so
    /// that removing a label is reflected too.
    pub fn train(
        &mut self,
        samples: &[(Vec<f32>, Category)],
        epochs: u32,
        learning_rate: f32,
    ) -> Result<TrainReport, MlError> {
        if samples.is_empty() {
            return Err(MlError::NoSamples);
        }

        for (embedding, _) in samples {
            if embedding.len() != self.dim {
                return Err(MlError::DimensionMismatch {
                    expected: self.dim,
                    actual: embedding.len(),
                });
            }
        }

        let mut loss = 0.0;
        let mut order: Vec<usize> = (0..samples.len()).collect();

        for _ in 0..epochs {
            // Deterministic shuffle: reproducible runs matter more than
            // cryptographic randomness for a local model.
            for index in (1..order.len()).rev() {
                let swap_with = (lcg(index as u64) as usize) % (index + 1);
                order.swap(index, swap_with);
            }

            loss = 0.0;

            for &sample_index in &order {
                let (embedding, label) = &samples[sample_index];
                let probabilities = self.probabilities(embedding);

                for (class_index, class) in self.classes.iter().enumerate() {
                    let target = if class == label { 1.0 } else { 0.0 };
                    let probability = probabilities[class_index];
                    let error = probability - target;

                    // Binary cross-entropy, for reporting only.
                    let clamped = probability.clamp(1e-6, 1.0 - 1e-6);
                    loss += if target > 0.5 {
                        -clamped.ln()
                    } else {
                        -(1.0 - clamped).ln()
                    };

                    let gradient = error * learning_rate;
                    for (weight, value) in self.weights[class_index].iter_mut().zip(embedding) {
                        *weight -= gradient * value;
                    }
                    self.biases[class_index] -= gradient;
                }
            }

            loss /= samples.len() as f32 * self.classes.len() as f32;
        }

        Ok(TrainReport {
            epochs,
            samples: samples.len(),
            final_loss: loss,
        })
    }

    /// Probabilities for every category, sorted from most to least likely.
    pub fn predict(&self, embedding: &[f32]) -> Vec<(Category, f32)> {
        let mut scored: Vec<(Category, f32)> = self
            .classes
            .iter()
            .copied()
            .zip(self.probabilities(embedding))
            .collect();
        scored.sort_by(|left, right| right.1.total_cmp(&left.1));
        scored
    }

    pub fn predict_top(&self, embedding: &[f32]) -> Option<(Category, f32)> {
        self.predict(embedding).into_iter().next()
    }

    /// A short, human-readable reason. UI surfaces this instead of a black box.
    pub fn explain(&self, embedding: &[f32], category: Category) -> Option<String> {
        let index = self.classes.iter().position(|class| *class == category)?;
        let probabilities = self.probabilities(embedding);
        let probability = probabilities[index];

        Some(format!(
            "{}: {:.0}% from {} learned feature weights",
            category.label(),
            probability * 100.0,
            self.weights[index]
                .iter()
                .filter(|weight| **weight != 0.0)
                .count()
        ))
    }

    fn probabilities(&self, embedding: &[f32]) -> Vec<f32> {
        let mut scores: Vec<f32> = self
            .classes
            .iter()
            .enumerate()
            .map(|(index, _)| {
                if embedding.len() != self.dim {
                    // A mismatch is a bug, not a data condition. Returning a
                    // neutral score keeps the browser usable while the index
                    // rebuilds with the current embedder.
                    return 0.0;
                }
                let dot: f32 = self.weights[index]
                    .iter()
                    .zip(embedding)
                    .map(|(weight, value)| weight * value)
                    .sum();
                dot + self.biases[index]
            })
            .collect();

        for score in &mut scores {
            *score = sigmoid(*score);
        }

        let total: f32 = scores.iter().sum();
        if total > 0.0 {
            for score in &mut scores {
                *score /= total;
            }
        } else {
            let uniform = 1.0 / scores.len().max(1) as f32;
            scores.fill(uniform);
        }

        scores
    }

    pub fn save(&self, path: impl AsRef<Path>) -> Result<(), MlError> {
        let path = path.as_ref();
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        let json = serde_json::to_string_pretty(self)?;
        fs::write(path, json)?;
        Ok(())
    }

    pub fn load(path: impl AsRef<Path>) -> Result<Self, MlError> {
        let json = fs::read_to_string(path)?;
        let model: Classifier = serde_json::from_str(&json)?;

        if model.classes.is_empty() {
            return Err(MlError::NoClasses);
        }
        if model.weights.len() != model.classes.len()
            || model.biases.len() != model.classes.len()
            || model
                .weights
                .iter()
                .any(|weights| weights.len() != model.dim)
        {
            return Err(MlError::InvalidModel(
                "weight matrix does not match classes or dimension".into(),
            ));
        }

        Ok(model)
    }

    /// Rank existing embeddings against a category, for the "why is this here?"
    /// panel and for neighbour suggestions.
    pub fn score_category(&self, embedding: &[f32], category: Category) -> f32 {
        let index = match self.classes.iter().position(|class| *class == category) {
            Some(index) => index,
            None => return 0.0,
        };
        sigmoid(
            self.biases[index]
                + self.weights[index]
                    .iter()
                    .zip(embedding)
                    .map(|(w, x)| w * x)
                    .sum::<f32>(),
        )
    }

    pub fn similarity(a: &[f32], b: &[f32]) -> f32 {
        cosine(a, b)
    }
}

fn sigmoid(value: f32) -> f32 {
    1.0 / (1.0 + (-value).exp())
}

fn lcg(seed: u64) -> u64 {
    let mut state = seed.wrapping_add(0x9e37_79b9_7f4a_7c15);
    state = (state ^ (state >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
    state = (state ^ (state >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
    state ^ (state >> 31)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn one_hot(index: usize, dim: usize) -> Vec<f32> {
        let mut vector = vec![0.0; dim];
        vector[index] = 1.0;
        vector
    }

    #[test]
    fn learns_separable_one_hot_examples() {
        let dim = 8;
        let mut classifier =
            Classifier::new(dim, vec![Category::Documents, Category::Images]).expect("classifier");

        let samples = vec![
            (one_hot(0, dim), Category::Documents),
            (one_hot(1, dim), Category::Documents),
            (one_hot(2, dim), Category::Images),
            (one_hot(3, dim), Category::Images),
        ];

        let report = classifier.train(&samples, 300, 0.5).expect("training");
        assert!(report.final_loss < 0.2, "loss={}", report.final_loss);

        let top = classifier
            .predict_top(&one_hot(0, dim))
            .expect("prediction");
        assert_eq!(top.0, Category::Documents);
        assert!(top.1 > 0.5, "confidence={}", top.1);
    }

    #[test]
    fn predictions_sum_to_one() {
        let mut classifier = Classifier::with_default_classes(16).expect("classifier");
        classifier
            .train(&[(vec![0.1; 16], Category::Code)], 10, 0.2)
            .expect("training");

        let total: f32 = classifier
            .predict(&[0.1f32; 16])
            .iter()
            .map(|(_, probability)| probability)
            .sum();
        assert!((total - 1.0).abs() < 1e-5);
    }

    #[test]
    fn rejects_dimension_mismatch() {
        let mut classifier = Classifier::with_default_classes(4).expect("classifier");
        let error = classifier
            .train(&[(vec![1.0; 3], Category::Code)], 1, 0.1)
            .expect_err("must fail");
        assert!(matches!(error, MlError::DimensionMismatch { .. }));
    }

    #[test]
    fn model_round_trips_through_json() {
        let directory = std::env::temp_dir().join("folderol-model-test");
        std::fs::create_dir_all(&directory).expect("temp dir");
        let path = directory.join("model.json");

        let mut classifier = Classifier::with_default_classes(8).expect("classifier");
        classifier
            .train(&[(vec![0.5; 8], Category::Audio)], 20, 0.2)
            .expect("training");
        classifier.save(&path).expect("save");

        let loaded = Classifier::load(&path).expect("load");
        assert_eq!(loaded.dim(), classifier.dim());
        assert_eq!(loaded.classes(), classifier.classes());

        std::fs::remove_file(path).ok();
    }
}
