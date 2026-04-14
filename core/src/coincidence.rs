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

    fn review(&self, _input: &[u8]) -> String {
        todo!("HashPrism::review for Detector<N>")
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
pub fn canonical_hash(bytes: &[u8]) -> String {
    let detection = CANONICAL.detect(bytes);
    match detection.eigenvalue_hex() {
        Some(eigenvalue_hex) => {
            // The raw eigenvalue is variable-length (depends on N and dimension).
            // Hash it through SHA-256 to produce a fixed 32-byte / 64-char address.
            // The coincidence detection is preserved: the eigenvalue captures the
            // multi-observer agreement, SHA-256 just compresses it to fixed size.
            let eigenvalue_bytes = hex::decode(eigenvalue_hex)
                .expect("eigenvalue_hex should be valid hex");
            let mut hasher = Sha256::new();
            hasher.update(b"prism-core:coincidence:");
            hasher.update(&eigenvalue_bytes);
            hex::encode(hasher.finalize())
        }
        None => {
            // Fallback for degenerate input (empty bytes, zero state, disagreement).
            // SHA-256 with domain separation prefix.
            let mut hasher = Sha256::new();
            hasher.update(b"prism-core:dark:");
            hasher.update(bytes);
            hex::encode(hasher.finalize())
        }
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
}
