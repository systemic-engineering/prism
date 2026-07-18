//! Training pipeline: gradient descent → quantize → weight arrays.
//! The algorithm never changes. Only the weights.

use crate::fate::weights::{WeightSet, Weights};
use crate::fate::FEATURE_DIM;
use serde::Deserialize;

#[derive(Clone, Debug, Deserialize)]
pub struct Example {
    pub features: Vec<f64>,
    pub context: usize, // current model index 0-4
    pub target: usize,  // correct next model index 0-4
}

pub fn load_examples(path: &str) -> Result<Vec<Example>, String> {
    let json = std::fs::read_to_string(path).map_err(|e| format!("{}: {}", path, e))?;
    serde_json::from_str(&json).map_err(|e| format!("parse: {}", e))
}

// ---------------------------------------------------------------------------
// Training configuration
// ---------------------------------------------------------------------------

pub struct TrainConfig {
    pub learning_rate: f64,
    pub epochs: usize,
}

// ---------------------------------------------------------------------------
// F64Weights — trainable weight set before quantization
// ---------------------------------------------------------------------------

/// f64 weight set for training (before quantization).
pub struct F64Weights {
    /// bias[context][output] — 5 contexts × 5 outputs
    pub bias: [[f64; 5]; 5],
    /// feature_w[context][output][feature] — 5 × 5 × FEATURE_DIM
    pub feature_w: [[[f64; FEATURE_DIM]; 5]; 5],
}

impl F64Weights {
    pub fn zero() -> Self {
        F64Weights {
            bias: [[0.0; 5]; 5],
            feature_w: [[[0.0; FEATURE_DIM]; 5]; 5],
        }
    }

    /// Compute logits for a given context and features.
    pub fn forward(&self, context: usize, features: &[f64; FEATURE_DIM]) -> [f64; 5] {
        let mut logits = self.bias[context];
        for i in 0..5 {
            for j in 0..FEATURE_DIM {
                logits[i] += self.feature_w[context][i][j] * features[j];
            }
        }
        logits
    }

    /// Softmax of 5 logits.
    pub fn softmax(logits: &[f64; 5]) -> [f64; 5] {
        let max = logits.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
        let mut exps = [0.0; 5];
        let mut sum = 0.0;
        for i in 0..5 {
            exps[i] = (logits[i] - max).exp();
            sum += exps[i];
        }
        for i in 0..5 {
            exps[i] /= sum;
        }
        exps
    }

    /// Predict: return argmax of forward pass.
    pub fn predict(&self, context: usize, features: &[f64; FEATURE_DIM]) -> usize {
        let logits = self.forward(context, features);
        let mut best = 0;
        for i in 1..5 {
            if logits[i] > logits[best] {
                best = i;
            }
        }
        best
    }
}

// ---------------------------------------------------------------------------
// Training loop
// ---------------------------------------------------------------------------

/// Train Fate weights from labeled examples.
/// Uses cross-entropy loss with softmax gradient descent.
pub fn train(examples: &[Example], config: &TrainConfig) -> F64Weights {
    let mut w = F64Weights::zero();

    for _epoch in 0..config.epochs {
        for ex in examples {
            // Convert features vec to array
            let mut features = [0.0; FEATURE_DIM];
            for (i, &v) in ex.features.iter().enumerate().take(FEATURE_DIM) {
                features[i] = v;
            }

            // Forward: compute logits and softmax
            let logits = w.forward(ex.context, &features);
            let probs = F64Weights::softmax(&logits);

            // Gradient: dL/d_logit_i = probs[i] - (1 if i == target else 0)
            let mut grad = probs;
            grad[ex.target] -= 1.0;

            // Update biases and feature weights
            for i in 0..5 {
                w.bias[ex.context][i] -= config.learning_rate * grad[i];
                for j in 0..FEATURE_DIM {
                    w.feature_w[ex.context][i][j] -= config.learning_rate * grad[i] * features[j];
                }
            }
        }
    }

    w
}

// ---------------------------------------------------------------------------
// Evaluation
// ---------------------------------------------------------------------------

/// Evaluate f64 weights on examples. Returns accuracy 0.0-1.0.
pub fn evaluate_f64(w: &F64Weights, examples: &[Example]) -> f64 {
    let mut correct = 0;
    for ex in examples {
        let mut features = [0.0; FEATURE_DIM];
        for (i, &v) in ex.features.iter().enumerate().take(FEATURE_DIM) {
            features[i] = v;
        }
        if w.predict(ex.context, &features) == ex.target {
            correct += 1;
        }
    }
    correct as f64 / examples.len() as f64
}

// ---------------------------------------------------------------------------
// Quantization + pipeline
// ---------------------------------------------------------------------------

pub struct PipelineConfig {
    pub learning_rate: f64,
    pub epochs: usize,
}

/// Quantize f64 weights to u8 Weights.
/// Maps biases to [0, 20] range. Maps feature weights to [0, 5] range.
/// Preserves relative ordering (argmax) within each context.
pub fn quantize(f64w: &F64Weights) -> Weights {
    let mut sets = [
        WeightSet::zero(),
        WeightSet::zero(),
        WeightSet::zero(),
        WeightSet::zero(),
        WeightSet::zero(),
    ];

    for ctx in 0..5 {
        // Quantize biases: map to [0, 20] range
        let biases = &f64w.bias[ctx];
        let min = biases.iter().cloned().fold(f64::INFINITY, f64::min);
        let max = biases.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
        let range = (max - min).max(1e-9);
        for i in 0..5 {
            sets[ctx].bias[i] = ((biases[i] - min) / range * 20.0).round() as u8;
        }

        // Quantize feature weights: map to [0, 5] range
        for i in 0..5 {
            let fw = &f64w.feature_w[ctx][i];
            let fmin = fw.iter().cloned().fold(f64::INFINITY, f64::min);
            let fmax = fw.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
            let frange = (fmax - fmin).max(1e-9);
            for j in 0..FEATURE_DIM {
                sets[ctx].feature_weights[i][j] = ((fw[j] - fmin) / frange * 5.0).round() as u8;
            }
        }
    }

    Weights { sets }
}

/// Full pipeline: train → quantize → Weights.
pub fn pipeline(examples: &[Example], config: &PipelineConfig) -> Weights {
    let trained = train(
        examples,
        &TrainConfig {
            learning_rate: config.learning_rate,
            epochs: config.epochs,
        },
    );
    quantize(&trained)
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_example() {
        let json = r#"{"features": [0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0], "context": 0, "target": 1}"#;
        let ex: Example = serde_json::from_str(json).unwrap();
        assert_eq!(ex.context, 0);
        assert_eq!(ex.target, 1);
        assert_eq!(ex.features.len(), 16);
    }

    #[test]
    fn load_seed_examples() {
        let examples = load_examples("training/examples.json").unwrap();
        assert!(examples.len() >= 5);
        assert_eq!(examples[0].features.len(), 16);
        assert_eq!(examples[0].context, 0);
        assert_eq!(examples[0].target, 1);
    }

    #[test]
    fn f64_weights_zero() {
        let w = F64Weights::zero();
        assert_eq!(w.bias[0][0], 0.0);
        assert_eq!(w.feature_w[0][0][0], 0.0);
    }

    #[test]
    fn softmax_sums_to_one() {
        let result = F64Weights::softmax(&[1.0, 2.0, 3.0, 4.0, 5.0]);
        let sum: f64 = result.iter().sum();
        assert!((sum - 1.0).abs() < 1e-9);
    }

    #[test]
    fn train_on_seed_data() {
        let examples = load_examples("training/examples.json").unwrap();
        let trained = train(
            &examples,
            &TrainConfig {
                learning_rate: 0.1,
                epochs: 500,
            },
        );
        let accuracy = evaluate_f64(&trained, &examples);
        eprintln!("  Training accuracy: {:.1}%", accuracy * 100.0);
        assert!(
            accuracy >= 0.9,
            "should achieve >=90%, got {:.1}%",
            accuracy * 100.0
        );
    }

    #[test]
    fn untrained_is_random() {
        let w = F64Weights::zero();
        let examples = load_examples("training/examples.json").unwrap();
        let accuracy = evaluate_f64(&w, &examples);
        // Untrained should be ~20% (random among 5 classes)
        assert!(
            accuracy < 0.5,
            "untrained should be near-random, got {:.1}%",
            accuracy * 100.0
        );
    }

    #[test]
    fn quantize_preserves_argmax() {
        let examples = load_examples("training/examples.json").unwrap();
        let trained = train(
            &examples,
            &TrainConfig {
                learning_rate: 0.1,
                epochs: 500,
            },
        );
        let quantized = quantize(&trained);

        // For each context, the argmax of f64 biases should match argmax of u8 biases
        for ctx in 0..5 {
            let f64_argmax = trained.bias[ctx]
                .iter()
                .enumerate()
                .max_by(|a, b| a.1.partial_cmp(b.1).unwrap())
                .unwrap()
                .0;
            let u8_argmax = quantized.sets[ctx]
                .bias
                .iter()
                .enumerate()
                .max_by(|a, b| a.1.cmp(b.1))
                .unwrap()
                .0;
            assert_eq!(
                f64_argmax, u8_argmax,
                "quantized argmax should match for context {}",
                ctx
            );
        }
    }

    #[test]
    fn pipeline_produces_weights() {
        let examples = load_examples("training/examples.json").unwrap();
        let weights = pipeline(
            &examples,
            &PipelineConfig {
                learning_rate: 0.1,
                epochs: 500,
            },
        );

        // Should produce valid weights
        assert_eq!(weights.param_count(), 425);
        // Biases should be non-zero (training should have learned something)
        let total_bias: u16 = weights
            .sets
            .iter()
            .flat_map(|s| s.bias.iter())
            .map(|&b| b as u16)
            .sum();
        assert!(
            total_bias > 0,
            "trained weights should have non-zero biases"
        );
    }

    #[test]
    fn pipeline_weights_serializable() {
        let examples = load_examples("training/examples.json").unwrap();
        let weights = pipeline(
            &examples,
            &PipelineConfig {
                learning_rate: 0.1,
                epochs: 500,
            },
        );

        let bytes = weights.to_bytes();
        let restored = Weights::from_bytes(&bytes).unwrap();
        assert_eq!(weights.sets[0].bias, restored.sets[0].bias);
    }
}
