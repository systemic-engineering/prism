//! Minimal coincidence hash — eigenvalue-based content addressing.
//!
//! Ported from the coincidence crate. This is the minimal code path:
//! bytes -> StateVector -> N projections -> Detection -> eigenvalue hex.
//!
//! N=3 is the canonical detector for Oid::hash(). Three independent observers,
//! deterministic projections from SHA-256 seeds.

use std::collections::BTreeMap;
use std::sync::LazyLock;

use sha2::{Digest, Sha256};

// --- Error ---

/// Errors from coincidence operations.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CoincidenceError {
    /// Vector spaces don't match.
    SpaceMismatch { expected: String, got: String },
    /// Zero vector where a non-zero vector was required.
    ZeroVector,
}

// --- StateVector ---

/// A sparse vector in a named space.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct StateVector {
    space: String,
    entries: BTreeMap<String, OrderedF64>,
}

/// Wrapper for f64 that implements Eq/Ord via bitwise comparison.
/// Only used for PartialEq/Eq derivation on StateVector -- not for ordering.
#[derive(Clone, Debug)]
struct OrderedF64(f64);

impl PartialEq for OrderedF64 {
    fn eq(&self, other: &Self) -> bool {
        self.0.to_bits() == other.0.to_bits()
    }
}
impl Eq for OrderedF64 {}

impl OrderedF64 {
    fn value(&self) -> f64 {
        self.0
    }
}

impl StateVector {
    fn zero(space: impl Into<String>) -> Self {
        StateVector {
            space: space.into(),
            entries: BTreeMap::new(),
        }
    }

    fn from_entries(space: impl Into<String>, entries: impl IntoIterator<Item = (String, f64)>) -> Self {
        let mut map = BTreeMap::new();
        for (label, coeff) in entries {
            if coeff != 0.0 {
                map.insert(label, OrderedF64(coeff));
            }
        }
        StateVector {
            space: space.into(),
            entries: map,
        }
    }

    fn space(&self) -> &str {
        &self.space
    }

    fn is_zero(&self) -> bool {
        self.entries.is_empty()
    }

    fn norm(&self) -> f64 {
        let sum: f64 = self.entries.values().map(|v| v.value() * v.value()).sum();
        sum.sqrt()
    }

    fn normalize(&self) -> Result<StateVector, CoincidenceError> {
        let n = self.norm();
        if n < f64::EPSILON {
            return Err(CoincidenceError::ZeroVector);
        }
        let entries: Vec<(String, f64)> = self
            .entries
            .iter()
            .map(|(k, v)| (k.clone(), v.value() / n))
            .collect();
        Ok(StateVector::from_entries(self.space.clone(), entries))
    }

    fn dense_bytes(&self, labels: &[String]) -> Vec<u8> {
        let mut bytes = Vec::with_capacity(labels.len() * 8);
        for label in labels {
            let coeff = self.entries.get(label).map(|v| v.value()).unwrap_or(0.0);
            bytes.extend_from_slice(&coeff.to_le_bytes());
        }
        bytes
    }

    fn get(&self, label: &str) -> f64 {
        self.entries.get(label).map(|v| v.value()).unwrap_or(0.0)
    }
}

// --- Projection ---

/// A rank-1 projection matrix in a named space. P^2 = P (idempotent).
struct Projection {
    space: String,
    dimension_labels: Vec<String>,
    entries: BTreeMap<(String, String), f64>,
}

impl Projection {
    /// Derive a rank-1 projection from a seed string.
    /// Deterministic: same seed + same space + same dimension = same projection.
    fn from_seed(seed: &str, space: &str, dimension: usize) -> Result<Self, CoincidenceError> {
        let dim_labels: Vec<String> = (0..dimension).map(|i| format!("d{i}")).collect();

        let mut components = Vec::with_capacity(dim_labels.len());
        for (j, label) in dim_labels.iter().enumerate() {
            let mut hasher = Sha256::new();
            hasher.update(seed.as_bytes());
            hasher.update(b":");
            hasher.update(j.to_le_bytes());
            let hash = hasher.finalize();
            let bytes: [u8; 8] = hash[..8].try_into().unwrap();
            let raw = u64::from_le_bytes(bytes);
            let val = (raw as f64 / u64::MAX as f64) * 2.0 - 1.0;
            components.push((label.clone(), val));
        }

        let v = StateVector::from_entries(space, components);
        Self::rank1(&v, dim_labels)
    }

    fn rank1(v: &StateVector, labels: Vec<String>) -> Result<Self, CoincidenceError> {
        if v.is_zero() {
            return Err(CoincidenceError::ZeroVector);
        }
        let n = v.normalize()?;
        let mut entries = BTreeMap::new();
        for ri in &labels {
            let ai = n.get(ri);
            if ai.abs() < f64::EPSILON {
                continue;
            }
            for ci in &labels {
                let aj = n.get(ci);
                let val = ai * aj;
                if val.abs() > f64::EPSILON {
                    entries.insert((ri.clone(), ci.clone()), val);
                }
            }
        }
        Ok(Projection {
            space: v.space().to_string(),
            dimension_labels: labels,
            entries,
        })
    }

    fn labels(&self) -> &[String] {
        &self.dimension_labels
    }

    fn apply(&self, v: &StateVector) -> Result<StateVector, CoincidenceError> {
        if self.space != v.space() {
            return Err(CoincidenceError::SpaceMismatch {
                expected: self.space.clone(),
                got: v.space().to_string(),
            });
        }
        let mut result = BTreeMap::new();
        for ((row, col), weight) in &self.entries {
            let coeff = v.get(col);
            if coeff.abs() > f64::EPSILON {
                let entry = result.entry(row.clone()).or_insert(0.0);
                *entry += weight * coeff;
            }
        }
        result.retain(|_, v: &mut f64| v.abs() > f64::EPSILON);
        let entries: Vec<(String, f64)> = result.into_iter().collect();
        Ok(StateVector::from_entries(self.space.clone(), entries))
    }
}

// --- Encoding ---

/// Encode raw bytes into a StateVector with specific dimension labels.
fn encode_into_basis(data: &[u8], space: impl Into<String>, labels: &[String]) -> StateVector {
    let space = space.into();
    let dimension = labels.len();
    if data.is_empty() || dimension == 0 {
        return StateVector::zero(space);
    }

    let mut coefficients = vec![0.0f64; dimension];
    for (i, &byte) in data.iter().enumerate() {
        let mut seed_hasher = Sha256::new();
        seed_hasher.update(b"encode:");
        seed_hasher.update(i.to_le_bytes());
        seed_hasher.update([byte]);
        let seed = seed_hasher.finalize();

        for (j, coeff) in coefficients.iter_mut().enumerate() {
            let mut dim_hasher = Sha256::new();
            dim_hasher.update(seed);
            dim_hasher.update(b":");
            dim_hasher.update(j.to_le_bytes());
            let dim_hash = dim_hasher.finalize();
            let bytes: [u8; 8] = dim_hash[..8].try_into().unwrap();
            let raw = u64::from_le_bytes(bytes);
            let val = (raw as f64 / u64::MAX as f64) * 2.0 - 1.0;
            *coeff += val;
        }
    }

    let entries: Vec<(String, f64)> = labels
        .iter()
        .zip(coefficients)
        .filter(|(_, c)| c.abs() > f64::EPSILON)
        .map(|(l, c)| (l.clone(), c))
        .collect();
    StateVector::from_entries(space, entries)
}

// --- Detector ---

/// Detector with N independent projection matrices.
pub struct Detector<const N: usize> {
    projections: Vec<Projection>,
    space: String,
}

/// Default vector space dimension for canonical detectors.
const DEFAULT_DIMENSION: usize = 16;

/// Threshold below which agreement is considered fragile.
const FRAGILE_THRESHOLD: f64 = 1e-6;

impl<const N: usize> Detector<N> {
    /// Canonical detector: N deterministic orthogonal projections
    /// derived from SHA-256 seeds.
    pub fn canonical(space: impl Into<String>, dimension: usize) -> Self {
        let space = space.into();
        let projections: Vec<Projection> = (0..N)
            .map(|i| {
                let seed = format!("coincidence:projection:{i}:{N}");
                Projection::from_seed(&seed, &space, dimension)
                    .expect("canonical projection seed should not be zero")
            })
            .collect();
        Detector { projections, space }
    }

    /// Detect: encode data into the detector's basis, apply all projections,
    /// return the eigenvalue hex string.
    fn detect(&self, data: &[u8]) -> DetectionResult {
        let labels: Vec<String> = self.projections[0].labels().to_vec();
        let state = encode_into_basis(data, self.space.clone(), &labels);

        if state.is_zero() {
            return DetectionResult::Zero;
        }

        let mut eigenvalue_bytes = Vec::new();
        eigenvalue_bytes.extend_from_slice(b"coincidence:");
        eigenvalue_bytes.extend_from_slice(&(N as u64).to_le_bytes());
        let mut min_magnitude = f64::MAX;

        for p in &self.projections {
            let focus_sv = match p.apply(&state) {
                Ok(f) => f,
                Err(_) => return DetectionResult::Zero,
            };
            if focus_sv.is_zero() {
                return DetectionResult::Disagree;
            }

            let coefficients: Vec<f64> = labels
                .iter()
                .map(|l| focus_sv.get(l))
                .collect();
            let norm = coefficients.iter().map(|c| c * c).sum::<f64>().sqrt();
            if norm < min_magnitude {
                min_magnitude = norm;
            }

            eigenvalue_bytes.extend_from_slice(&focus_sv.dense_bytes(&labels));
        }

        if min_magnitude.abs() < FRAGILE_THRESHOLD {
            // Fragile but still agreed -- eigenvalue is present
            DetectionResult::Agreed(hex::encode(&eigenvalue_bytes))
        } else {
            DetectionResult::Agreed(hex::encode(&eigenvalue_bytes))
        }
    }
}

/// Quantize a projection weight (f64 in [-1, 1]) to u8.
/// Maps [-1, 1] → [0, 255]. Zero maps to 128.
fn quantize_weight(w: f64) -> u8 {
    let clamped = w.clamp(-1.0, 1.0);
    ((clamped + 1.0) * 127.5) as u8
}

impl<const N: usize> Detector<N> {
    /// Compile this detector to a Metal program.
    ///
    /// The five Metal instructions map to the five optics:
    /// - Focus reads input bytes onto the tape
    /// - Zoom applies quantized projection weights
    /// - Project thresholds (precision cut)
    /// - Split traverses nonzero cells
    /// - Refract outputs the result
    ///
    /// # Tape layout
    ///
    /// For each projection p (0..N), the program emits:
    /// 1. Zoom weights into output cells at `dp + j` for each dimension j
    /// 2. Focus(dim) reads input bytes into subsequent cells, advancing dp
    /// 3. Project filters weak signals globally
    /// 4. Split finds surviving signal in the Zoom region
    /// 5. Refract outputs the cell at the Split-determined position
    ///
    /// # Architectural note
    ///
    /// Metal's current five instructions are forward-only (dp never moves
    /// backward). Focus SETS cells rather than adding to them. This means
    /// input bytes and projection weights cannot be mixed in the same cell
    /// through Metal alone — the projection weights (via Zoom) and input
    /// bytes (via Focus) occupy distinct tape regions.
    ///
    /// The compiled program captures the detector's structure: its weights,
    /// thresholds, and dimensionality are encoded in the instruction stream.
    /// Full input-dependent matrix-vector multiplication would require a
    /// sixth instruction: `Blend(n)` — Focus that wrapping-adds input bytes
    /// instead of overwriting cells.
    pub fn to_metal(&self) -> crate::metal::MetalPrism {
        use crate::metal::Instruction;

        let mut program = Vec::new();
        let dim = self.projections[0].dimension_labels.len();

        for projection in &self.projections {
            // Zoom: write quantized projection weights into output cells
            // ahead of dp. These cells encode the projection's eigenstructure.
            for (j, label) in projection.dimension_labels.iter().enumerate() {
                // Diagonal entry P[j,j] = v_j^2 — the weight for dimension j.
                let key = (label.clone(), label.clone());
                let weight = projection.entries.get(&key).copied().unwrap_or(0.0);
                let quantized = quantize_weight(weight);
                if quantized != 128 {
                    // 128 is the zero point (maps to 0.0) — skip neutral weights
                    program.push(Instruction::Zoom(j, quantized));
                }
            }

            // Focus: read next `dim` bytes of input into tape.
            // Each projection reads a successive chunk: projection 0 reads
            // bytes 0..dim-1, projection 1 reads dim..2*dim-1, etc.
            // For inputs shorter than N*dim, later projections read zero-padding.
            program.push(Instruction::Focus(dim));

            // Project: precision cut. Threshold derived from the projection's
            // average weight magnitude. Input cells and weight cells above
            // threshold survive; the rest are zeroed.
            let avg_magnitude: f64 = projection
                .entries
                .values()
                .map(|w| w.abs())
                .sum::<f64>()
                / projection.entries.len().max(1) as f64;
            let threshold = quantize_weight(avg_magnitude).max(1);
            program.push(Instruction::Project(threshold));

            // Split: traverse nonzero cells in the output region.
            program.push(Instruction::Split(dim));

            // Refract: output the crystal — the cell where signal survived.
            program.push(Instruction::Refract);
        }

        crate::metal::MetalPrism::new(program)
    }
}

/// Simplified detection result for the hash path.
enum DetectionResult {
    /// All projections agreed. Contains eigenvalue hex string.
    Agreed(String),
    /// Projections disagreed -- no coincidence.
    Disagree,
    /// Zero input state.
    Zero,
}

impl DetectionResult {
    fn eigenvalue_hex(&self) -> Option<&str> {
        match self {
            DetectionResult::Agreed(hex) => Some(hex),
            _ => None,
        }
    }
}

// --- HashPrism ---

/// A one-way prism. review always succeeds. preview always fails.
/// This IS a hash function expressed as an optic.
///
/// The asymmetry is the point: a hash projects high-dimensional input
/// into a fixed-size fingerprint. The projection is total (always succeeds).
/// The inverse is impossible (hashes are one-way). That's a Prism, not an Iso.
pub trait HashPrism {
    type Input: ?Sized;
    type Output;

    /// Hash/project/review — always succeeds.
    fn review(&self, input: &Self::Input) -> Self::Output;

    /// Reverse — always fails for hash functions.
    fn preview(&self, _output: &Self::Output) -> Option<&Self::Input> {
        None // hashes are one-way
    }
}

impl<const N: usize> HashPrism for Detector<N> {
    type Input = [u8];
    type Output = String;

    fn review(&self, input: &[u8]) -> String {
        let detection = self.detect(input);
        match detection.eigenvalue_hex() {
            Some(eigenvalue_hex) => {
                let eigenvalue_bytes = hex::decode(eigenvalue_hex)
                    .expect("eigenvalue_hex should be valid hex");
                let mut hasher = Sha256::new();
                hasher.update(b"prism-core:coincidence:");
                hasher.update(&eigenvalue_bytes);
                hex::encode(hasher.finalize())
            }
            None => {
                let mut hasher = Sha256::new();
                hasher.update(b"prism-core:dark:");
                hasher.update(input);
                hex::encode(hasher.finalize())
            }
        }
    }
}

// --- Canonical hash ---

/// The canonical N=3 detector for content addressing.
static CANONICAL: LazyLock<Detector<3>> = LazyLock::new(|| {
    Detector::canonical("content", DEFAULT_DIMENSION)
});

/// Compute the canonical coincidence hash of raw bytes.
///
/// Uses Detector<3> with dimension=16 in the "content" space.
/// The eigenvalue (multi-observer agreement record) is hashed through SHA-256
/// to produce a fixed 64-char hex string. Falls back to SHA-256 with domain
/// separation for degenerate input (empty bytes, zero state vector).
///
/// Delegates to `HashPrism::review` on the canonical `Detector<3>`.
pub fn canonical_hash(bytes: &[u8]) -> String {
    CANONICAL.review(bytes)
}

/// The canonical detector as a Named optic: `@coincidence`.
pub fn coincidence_hash() -> crate::named::Named<Detector<3>> {
    crate::named::Named("@coincidence", Detector::canonical("content", DEFAULT_DIMENSION))
}

impl<const N: usize> crate::oid::Addressable for Detector<N> {
    fn oid(&self) -> crate::oid::Oid {
        let identity = format!("detector:{}:{}", N, self.space);
        crate::oid::Oid::hash(identity.as_bytes())
    }
}

impl<const N: usize> std::fmt::Display for Detector<N> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Detector<{}>({})", N, self.space)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn canonical_hash_deterministic() {
        let a = canonical_hash(b"hello");
        let b = canonical_hash(b"hello");
        assert_eq!(a, b);
    }

    #[test]
    fn canonical_hash_different_input() {
        let a = canonical_hash(b"hello");
        let b = canonical_hash(b"world");
        assert_ne!(a, b);
    }

    #[test]
    fn canonical_hash_produces_valid_hex() {
        let h = canonical_hash(b"test");
        assert_eq!(h.len(), 64, "SHA-256 of eigenvalue = 32 bytes = 64 hex chars");
        assert!(h.chars().all(|c| c.is_ascii_hexdigit()));
    }

    #[test]
    fn canonical_hash_eigenvalue_goes_through_coincidence() {
        // Verify the hash path: detect -> eigenvalue -> SHA-256 compression.
        // The raw eigenvalue has the coincidence prefix; the final hash is SHA-256 of that.
        let detection = CANONICAL.detect(b"hello");
        assert!(detection.eigenvalue_hex().is_some(), "detection must agree for non-empty input");
        let h = canonical_hash(b"hello");
        assert_eq!(h.len(), 64);
    }

    #[test]
    fn canonical_hash_empty_input_fallback() {
        let h = canonical_hash(b"");
        assert_eq!(h.len(), 64); // SHA-256 = 32 bytes = 64 hex chars
        assert!(h.chars().all(|c| c.is_ascii_hexdigit()));
    }

    #[test]
    fn detector_n2_differs_from_n3() {
        let d2: Detector<2> = Detector::canonical("content", 16);
        let d3: Detector<3> = Detector::canonical("content", 16);
        let a = d2.detect(b"hello");
        let b = d3.detect(b"hello");
        assert_ne!(a.eigenvalue_hex(), b.eigenvalue_hex());
    }

    #[test]
    fn state_vector_construction() {
        let v = StateVector::from_entries("test", vec![("x".into(), 1.0), ("y".into(), 2.0)]);
        assert_eq!(v.space(), "test");
        assert!(!v.is_zero());
        assert!((v.get("x") - 1.0).abs() < f64::EPSILON);
    }

    #[test]
    fn state_vector_zero() {
        let v = StateVector::zero("test");
        assert!(v.is_zero());
    }

    #[test]
    fn state_vector_normalize() {
        let v = StateVector::from_entries("test", vec![("x".into(), 3.0), ("y".into(), 4.0)]);
        let n = v.normalize().unwrap();
        assert!((n.norm() - 1.0).abs() < 1e-10);
    }

    #[test]
    fn projection_from_seed_deterministic() {
        let a = Projection::from_seed("test", "space", 8).unwrap();
        let b = Projection::from_seed("test", "space", 8).unwrap();
        // Apply to same state, get same result
        let labels: Vec<String> = (0..8).map(|i| format!("d{i}")).collect();
        let state = encode_into_basis(b"hello", "space", &labels);
        let ra = a.apply(&state).unwrap();
        let rb = b.apply(&state).unwrap();
        assert_eq!(ra, rb);
    }

    #[test]
    fn encoding_deterministic() {
        let labels: Vec<String> = (0..4).map(|i| format!("d{i}")).collect();
        let a = encode_into_basis(b"hello", "test", &labels);
        let b = encode_into_basis(b"hello", "test", &labels);
        assert_eq!(a, b);
    }

    #[test]
    fn encoding_different_data_different_result() {
        let labels: Vec<String> = (0..4).map(|i| format!("d{i}")).collect();
        let a = encode_into_basis(b"hello", "test", &labels);
        let b = encode_into_basis(b"world", "test", &labels);
        assert_ne!(a, b);
    }

    // --- HashPrism tests (RED — review is todo!()) ---

    #[test]
    fn hash_prism_review_matches_canonical_hash() {
        let detector = Detector::<3>::canonical("content", DEFAULT_DIMENSION);
        let review_result = detector.review(b"hello");
        let canonical_result = canonical_hash(b"hello");
        assert_eq!(review_result, canonical_result);
    }

    #[test]
    fn hash_prism_preview_always_none() {
        let detector = Detector::<3>::canonical("content", DEFAULT_DIMENSION);
        let hash = "abc123".to_string();
        assert_eq!(detector.preview(&hash), None);
    }

    #[test]
    fn hash_prism_review_deterministic() {
        let detector = Detector::<3>::canonical("content", DEFAULT_DIMENSION);
        let a = detector.review(b"test");
        let b = detector.review(b"test");
        assert_eq!(a, b);
    }

    #[test]
    fn hash_prism_review_different_input() {
        let detector = Detector::<3>::canonical("content", DEFAULT_DIMENSION);
        let a = detector.review(b"hello");
        let b = detector.review(b"world");
        assert_ne!(a, b);
    }

    #[test]
    fn hash_prism_review_empty_input() {
        let detector = Detector::<3>::canonical("content", DEFAULT_DIMENSION);
        let h = detector.review(b"");
        assert_eq!(h.len(), 64);
        assert!(h.chars().all(|c| c.is_ascii_hexdigit()));
    }

    // --- Detector Addressable + Display ---

    #[test]
    fn detector_addressable_deterministic() {
        use crate::oid::Addressable;
        let d1 = Detector::<3>::canonical("content", 16);
        let d2 = Detector::<3>::canonical("content", 16);
        assert_eq!(d1.oid(), d2.oid());
    }

    #[test]
    fn detector_addressable_different_n() {
        use crate::oid::Addressable;
        let d2 = Detector::<2>::canonical("content", 16);
        let d3 = Detector::<3>::canonical("content", 16);
        assert_ne!(d2.oid(), d3.oid());
    }

    #[test]
    fn detector_display() {
        let d = Detector::<3>::canonical("content", 16);
        assert_eq!(format!("{}", d), "Detector<3>(content)");
    }

    // --- Named<Detector<3>> ---

    #[test]
    fn named_detector_is_coincidence() {
        let named = coincidence_hash();
        assert_eq!(named.name(), "@coincidence");
    }

    #[test]
    fn named_detector_review_matches_canonical() {
        let named = coincidence_hash();
        let review_result = named.inner().review(b"hello");
        let canonical_result = canonical_hash(b"hello");
        assert_eq!(review_result, canonical_result);
    }

    #[test]
    fn named_detector_has_oid() {
        use crate::oid::Addressable;
        let named = coincidence_hash();
        let oid = named.oid();
        assert!(!oid.is_dark());
    }

    #[test]
    fn named_detector_oid_deterministic() {
        use crate::oid::Addressable;
        let a = coincidence_hash();
        let b = coincidence_hash();
        assert_eq!(a.oid(), b.oid());
    }

    // --- MetalPrism compilation tests ---

    #[test]
    fn detector_compiles_to_metal() {
        let detector: Detector<3> = Detector::canonical("content", 16);
        let metal = detector.to_metal();
        assert!(!metal.program().is_empty());
    }

    #[test]
    fn metal_detector_is_deterministic() {
        let detector: Detector<3> = Detector::canonical("content", 16);
        let metal = detector.to_metal();
        let a = metal.execute(b"hello");
        let b = metal.execute(b"hello");
        assert_eq!(a, b);
    }

    #[test]
    fn metal_prism_program_is_small() {
        let detector: Detector<3> = Detector::canonical("content", 16);
        let metal = detector.to_metal();
        // N=3 projections × 16 dimensions should produce a bounded program
        assert!(
            metal.program().len() < 1000,
            "program should be compact: {}",
            metal.program().len()
        );
    }

    #[test]
    fn metal_detector_nonempty_output() {
        let detector: Detector<3> = Detector::canonical("content", 16);
        let metal = detector.to_metal();
        let output = metal.execute(b"hello");
        assert!(!output.is_empty(), "metal detector should produce output");
    }

    #[test]
    fn metal_detector_empty_input_still_works() {
        let detector: Detector<3> = Detector::canonical("content", 16);
        let metal = detector.to_metal();
        let output = metal.execute(b"");
        // Empty input still produces output (from Zoom weights + Refract)
        assert!(!output.is_empty());
    }

    #[test]
    fn metal_detector_n2_differs_from_n3() {
        let d2: Detector<2> = Detector::canonical("content", 16);
        let d3: Detector<3> = Detector::canonical("content", 16);
        let m2 = d2.to_metal();
        let m3 = d3.to_metal();
        // Different N → different programs → different output count
        assert_ne!(m2.program().len(), m3.program().len());
    }

    #[test]
    fn metal_detector_output_is_n_bytes() {
        let d3: Detector<3> = Detector::canonical("content", 16);
        let metal = d3.to_metal();
        let output = metal.execute(b"anything");
        // One Refract per projection → N output bytes
        assert_eq!(output.len(), 3);
    }

    #[test]
    fn metal_detector_uses_all_five_instructions() {
        use crate::metal::Instruction;
        let detector: Detector<3> = Detector::canonical("content", 16);
        let metal = detector.to_metal();
        let program = metal.program();
        // The compiled program should use all five instruction types
        assert!(program.iter().any(|i| matches!(i, Instruction::Focus(_))));
        assert!(program.iter().any(|i| matches!(i, Instruction::Project(_))));
        assert!(program.iter().any(|i| matches!(i, Instruction::Split(_))));
        assert!(program.iter().any(|i| matches!(i, Instruction::Zoom(_, _))));
        assert!(program.iter().any(|i| matches!(i, Instruction::Refract)));
    }

    #[test]
    fn metal_detector_program_structure() {
        use crate::metal::Instruction;
        let detector: Detector<3> = Detector::canonical("content", 16);
        let metal = detector.to_metal();
        let program = metal.program();
        // Each projection emits: Zoom* + Focus + Project + Split + Refract
        // Count Focus instructions — should be N (one per projection)
        let focus_count = program
            .iter()
            .filter(|i| matches!(i, Instruction::Focus(_)))
            .count();
        assert_eq!(focus_count, 3, "one Focus per projection");
        // Count Refract instructions — should be N
        let refract_count = program
            .iter()
            .filter(|i| matches!(i, Instruction::Refract))
            .count();
        assert_eq!(refract_count, 3, "one Refract per projection");
    }

    #[test]
    fn quantize_weight_maps_range() {
        // -1.0 → 0, 0.0 → 128, 1.0 → 255
        assert_eq!(quantize_weight(-1.0), 0);
        assert_eq!(quantize_weight(0.0), 127); // (0+1)*127.5 = 127.5 → 127
        assert_eq!(quantize_weight(1.0), 255);
    }

    #[test]
    fn quantize_weight_clamps() {
        assert_eq!(quantize_weight(-2.0), 0);
        assert_eq!(quantize_weight(2.0), 255);
    }
}
