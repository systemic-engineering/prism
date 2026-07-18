//! Weight sets for the universal Fate algorithm.
//! One algorithm. Five weight sets. The weights are the physics.

use crate::fate::FEATURE_DIM;

/// Weights for one context.
#[derive(Clone, Debug)]
pub struct WeightSet {
    pub bias: [u8; 5],
    pub feature_weights: [[u8; FEATURE_DIM]; 5],
}

/// All five weight sets.
#[derive(Clone, Debug)]
pub struct Weights {
    pub sets: [WeightSet; 5],
}

// ---------------------------------------------------------------------------
// Tests (written first — red before green)
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_weights_produce_cycle() {
        // Abyss→Path→Cart→Expl→Fate→Abyss
        // For context i, the highest bias should be at index (i+1) % 5.
        let w = Weights::default_cycle();

        // context 0 (Abyss): bias peak at index 1 (Introject)
        let b0 = w.sets[0].bias;
        assert!(
            b0[1] > b0[0],
            "Abyss context: Introject bias should beat Abyss"
        );
        assert!(
            b0[1] > b0[2],
            "Abyss context: Introject bias should beat Cartographer"
        );
        assert!(
            b0[1] > b0[3],
            "Abyss context: Introject bias should beat Explorer"
        );
        assert!(
            b0[1] > b0[4],
            "Abyss context: Introject bias should beat Fate"
        );

        // context 1 (Introject): bias peak at index 2 (Cartographer)
        let b1 = w.sets[1].bias;
        assert!(b1[2] > b1[0]);
        assert!(b1[2] > b1[1]);
        assert!(b1[2] > b1[3]);
        assert!(b1[2] > b1[4]);

        // context 2 (Cartographer): bias peak at index 3 (Explorer)
        let b2 = w.sets[2].bias;
        assert!(b2[3] > b2[0]);
        assert!(b2[3] > b2[1]);
        assert!(b2[3] > b2[2]);
        assert!(b2[3] > b2[4]);

        // context 3 (Explorer): bias peak at index 4 (Fate)
        let b3 = w.sets[3].bias;
        assert!(b3[4] > b3[0]);
        assert!(b3[4] > b3[1]);
        assert!(b3[4] > b3[2]);
        assert!(b3[4] > b3[3]);

        // context 4 (Fate): bias peak at index 0 (Abyss)
        let b4 = w.sets[4].bias;
        assert!(b4[0] > b4[1]);
        assert!(b4[0] > b4[2]);
        assert!(b4[0] > b4[3]);
        assert!(b4[0] > b4[4]);
    }

    #[test]
    fn weights_serialize_roundtrip() {
        let original = Weights::default_cycle();
        let bytes = original.to_bytes();
        let recovered = Weights::from_bytes(&bytes).expect("from_bytes should succeed");

        for s in 0..5 {
            assert_eq!(
                original.sets[s].bias, recovered.sets[s].bias,
                "bias mismatch at set {s}"
            );
            for r in 0..5 {
                assert_eq!(
                    original.sets[s].feature_weights[r], recovered.sets[s].feature_weights[r],
                    "feature_weights mismatch at set {s} row {r}"
                );
            }
        }
    }

    #[test]
    fn weights_param_count() {
        let w = Weights::default_cycle();
        let count = w.param_count();
        assert_eq!(count, 425, "param_count should be 425");
        assert_eq!(
            w.to_bytes().len(),
            count,
            "to_bytes().len() should match param_count"
        );
    }

    #[test]
    fn from_bytes_wrong_length_returns_none() {
        assert!(
            Weights::from_bytes(&[]).is_none(),
            "empty slice should return None"
        );
        assert!(
            Weights::from_bytes(&[0u8; 424]).is_none(),
            "one byte short should return None"
        );
        assert!(
            Weights::from_bytes(&[0u8; 426]).is_none(),
            "one byte over should return None"
        );
    }
}

// ---------------------------------------------------------------------------
// Implementation
// ---------------------------------------------------------------------------

impl WeightSet {
    /// All zeros. The untrained state.
    pub fn zero() -> Self {
        WeightSet {
            bias: [0u8; 5],
            feature_weights: [[0u8; FEATURE_DIM]; 5],
        }
    }
}

impl Weights {
    /// Default cycle: Abyss→Path→Cart→Expl→Fate→Abyss.
    ///
    /// For context i, bias[(i+1) % 5] = 10 (the next model in the cycle).
    /// Cartographer context (index 2) additionally gets feature_weights[2][0] = 1
    /// to give Cartographer (index 2) a slight boost when feature 0 is active,
    /// matching fate.bf's hardcoded feature contribution to cell 19 (output index 2).
    pub fn default_cycle() -> Self {
        let mut sets: [WeightSet; 5] = [
            WeightSet::zero(),
            WeightSet::zero(),
            WeightSet::zero(),
            WeightSet::zero(),
            WeightSet::zero(),
        ];

        // For each context i, set the bias for the next model in the cycle.
        for i in 0..5 {
            let next = (i + 1) % 5;
            sets[i].bias[next] = 10;
        }

        // Cartographer context (index 2): feature[0] boosts Cartographer (index 2).
        sets[2].feature_weights[2][0] = 1;

        Weights { sets }
    }

    /// Trained weights. Baked into the binary.
    /// Produced by: `pipeline(seed_examples, lr=0.1, epochs=1000)`.
    /// 100% accuracy on seed data. 425 bytes. The physics.
    pub fn trained() -> Self {
        const BYTES: [u8; 425] = [
            0, 20, 0, 0, 0, 0, 0, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 0, 0, 5, 5, 5, 5, 5, 5,
            5, 5, 5, 5, 5, 5, 5, 5, 5, 0, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 0, 5, 1, 1, 1,
            1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 0, 0, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 0, 0,
            20, 0, 0, 0, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 0, 5, 5, 5, 5, 5, 5, 5, 5, 5,
            5, 5, 5, 5, 5, 5, 5, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 5, 5, 5, 5, 5, 5,
            5, 5, 5, 5, 5, 5, 5, 5, 5, 0, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 0, 0, 0, 20,
            0, 0, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 0, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5,
            5, 5, 5, 5, 5, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 5, 5, 5, 5, 5, 5, 5, 5,
            5, 5, 5, 5, 5, 5, 5, 0, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 0, 0, 0, 0, 20, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 20, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        ];
        Weights::from_bytes(&BYTES).expect("trained weights are valid")
    }

    /// Total parameter count: 5 × (5 + 5×FEATURE_DIM) = 5 × 85 = 425.
    pub fn param_count(&self) -> usize {
        5 * (5 + 5 * FEATURE_DIM)
    }

    /// Serialize to a flat byte array.
    /// Layout per WeightSet: bias[5] then feature_weights[5][FEATURE_DIM].
    /// Five WeightSets concatenated.
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut out = Vec::with_capacity(self.param_count());
        for set in &self.sets {
            out.extend_from_slice(&set.bias);
            for row in &set.feature_weights {
                out.extend_from_slice(row);
            }
        }
        out
    }

    /// Deserialize from a flat byte array.
    /// Returns None if the slice length does not match param_count (425).
    pub fn from_bytes(bytes: &[u8]) -> Option<Self> {
        let expected = 5 * (5 + 5 * FEATURE_DIM);
        if bytes.len() != expected {
            return None;
        }

        let mut sets: [WeightSet; 5] = [
            WeightSet::zero(),
            WeightSet::zero(),
            WeightSet::zero(),
            WeightSet::zero(),
            WeightSet::zero(),
        ];

        let mut cursor = 0;
        for set in sets.iter_mut() {
            // Read bias: 5 bytes
            set.bias.copy_from_slice(&bytes[cursor..cursor + 5]);
            cursor += 5;

            // Read feature_weights: 5 rows × FEATURE_DIM bytes
            for row in set.feature_weights.iter_mut() {
                row.copy_from_slice(&bytes[cursor..cursor + FEATURE_DIM]);
                cursor += FEATURE_DIM;
            }
        }

        Some(Weights { sets })
    }
}
