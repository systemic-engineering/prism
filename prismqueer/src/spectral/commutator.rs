//! `prismqueer::spectral::commutator` — sheaf-Laplacian commutator + singular-value
//! spectrum reading.
//!
//! Per Alex 2026-09-05 verbatim ("each message a spectral commutator") + the
//! Q-α geometry reasoning: `flux::commutator` = OPERATOR at Transport level
//! (per prism::Bundle Fiber → Connection → Gauge → **Transport** → Closure →
//! Bundle); `spectral::*` = MEASUREMENT. This module lifts the commutator to
//! sheaf altitude via Laplacian representation and reads the resulting
//! spectrum.
//!
//! # Geometry
//!
//! Given two sheafs A and B over union vertex set V, build the graph
//! Laplacians `L_A` and `L_B` (symmetric, real n×n). The commutator
//! `[L_A, L_B] = L_A L_B - L_B L_A` is antisymmetric (since `(L_A L_B -
//! L_B L_A)^T = L_B L_A - L_A L_B = -[L_A, L_B]`). Real antisymmetric
//! matrices have pure-imaginary eigenvalues in conjugate pairs; the
//! singular values `σ_i = |eigenvalue_i|` are real non-negative and
//! measure the magnitude of non-commutativity per axis.
//!
//! # Substrate reading
//!
//! - **Empty spectrum** ⇔ A and B compose commutatively (closed-loop;
//!   observer-independent; softmax-happy-path per
//!   [[project-open-loop-autopoiesis-IS-bumblebee-buzz-through-assertions-
//!   closed-loop-IS-softmax-argmax]])
//! - **Non-empty spectrum** ⇔ A and B compose non-commutatively
//!   (open-loop; observer-inclusive; @bumblebee.buzz through assertions;
//!   Foerster-canonical LOVE-filter admissible substrate)
//!
//! # Composes over LANDED
//!
//! - `prismqueer::spectral::kleinos::Graph` — input carrier
//! - `prismqueer::ffi::singular_values` — LAPACK dgesdd via FLANG-compiled
//!   `native/spectral.f90`

use std::collections::{BTreeMap, BTreeSet};

use terni::Imperfect;

use crate::spectral::kleinos::{Graph, VertexId};

/// Singular-value spectrum of the sheaf-Laplacian commutator `[L_A, L_B]`.
///
/// Substrate-noun per Alex 2026-09-05 "Simple made Easy" naming discipline.
#[derive(Clone, Debug)]
pub struct Dissonance {
    pub spectrum: Vec<f64>,
}

impl Dissonance {
    /// Read the singular-value spectrum of `[L_A, L_B]` over the union of
    /// A and B vertex sets.
    ///
    /// Thresholds are relative to the operators' scale (product of Frobenius
    /// norms). Numerical zeros are dropped; an empty spectrum in `Success`
    /// witnesses commutative composition.
    ///
    /// Fails closed: if LAPACK cannot measure, the answer is
    /// `Failure(Unmeasured)`, never an empty spectrum (which would read as
    /// "commutes"). Singular values between numerical zero and clearly
    /// nonzero are returned as `Partial`: not dropped, not guessed.
    pub fn of(a: &Graph, b: &Graph) -> Imperfect<Self, Unmeasured, Vec<f64>> {
        let vertices: BTreeSet<VertexId> = a.vertices().chain(b.vertices()).collect();
        let n = vertices.len();
        if n < 2 {
            return Imperfect::Success(Self {
                spectrum: Vec::new(),
            });
        }
        let vertex_index: BTreeMap<VertexId, usize> = vertices
            .into_iter()
            .enumerate()
            .map(|(i, v)| (v, i))
            .collect();

        let l_a = build_laplacian(a, &vertex_index, n);
        let l_b = build_laplacian(b, &vertex_index, n);

        let ab = matmul(&l_a, &l_b, n);
        let ba = matmul(&l_b, &l_a, n);
        let commutator: Vec<f64> = ab.iter().zip(ba.iter()).map(|(x, y)| x - y).collect();

        let frobenius = |m: &[f64]| m.iter().map(|x| x * x).sum::<f64>().sqrt();
        let scale = frobenius(&l_a) * frobenius(&l_b);
        read(crate::ffi::singular_values(n, n, &commutator), scale)
    }
}

/// LAPACK could not compute the singular values (`info` code).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Unmeasured(pub i32);

/// Below this fraction of the scale, a singular value is numerical zero.
const NUMERICAL_ZERO: f64 = 1e-12;

/// At or above this fraction of the scale, a singular value is clearly nonzero.
/// Between the two thresholds the instrument does not know yet.
const CLEARLY_NONZERO: f64 = 1e-6;

/// Classify a singular-value measurement against the operators' scale
/// (product of their Frobenius norms, floored at 1).
fn read(
    measured: Result<Vec<f64>, i32>,
    scale: f64,
) -> Imperfect<Dissonance, Unmeasured, Vec<f64>> {
    let svs = match measured {
        Ok(svs) => svs,
        Err(info) => return Imperfect::Failure(Unmeasured(info), Vec::new()),
    };
    let scale = scale.max(1.0);
    let mut spectrum = Vec::new();
    let mut ambiguous = Vec::new();
    for s in svs {
        if s >= CLEARLY_NONZERO * scale {
            spectrum.push(s);
        } else if s > NUMERICAL_ZERO * scale {
            ambiguous.push(s);
        }
    }
    let cs = Dissonance { spectrum };
    if ambiguous.is_empty() {
        Imperfect::Success(cs)
    } else {
        Imperfect::Partial(cs, ambiguous)
    }
}

/// Build the graph Laplacian of `sheaf` over the given vertex-index basis.
/// Edges whose endpoints are outside the basis are skipped.
pub(crate) fn build_laplacian(
    sheaf: &Graph,
    vertex_index: &BTreeMap<VertexId, usize>,
    n: usize,
) -> Vec<f64> {
    let mut lap = vec![0.0_f64; n * n];
    for (u, v) in sheaf.edges() {
        if let (Some(&i), Some(&j)) = (vertex_index.get(&u), vertex_index.get(&v)) {
            lap[i * n + j] -= 1.0;
            lap[j * n + i] -= 1.0;
            lap[i * n + i] += 1.0;
            lap[j * n + j] += 1.0;
        }
    }
    lap
}

/// Row-major n×n dense matrix multiplication. Naive O(n³); acceptable for
/// small sheafs. Lift to BLAS `dgemm` if profiling shows this on the hot path.
fn matmul(a: &[f64], b: &[f64], n: usize) -> Vec<f64> {
    let mut c = vec![0.0_f64; n * n];
    for i in 0..n {
        for j in 0..n {
            let mut sum = 0.0_f64;
            for k in 0..n {
                sum += a[i * n + k] * b[k * n + j];
            }
            c[i * n + j] = sum;
        }
    }
    c
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::spectral::kleinos::{graph_complete_of_order, graph_from_edges};

    /// `[L_A, L_A] = L_A L_A - L_A L_A = 0`. Self-commutator vanishes.
    /// Substrate-reading: composition of A with itself is trivially commutative.
    fn measured(a: &Graph, b: &Graph) -> Dissonance {
        match Dissonance::of(a, b) {
            terni::Imperfect::Success(cs) => cs,
            other => panic!("expected a clean measurement, got {other:?}"),
        }
    }

    // --- Fail closed, scale-relative, honest about the ambiguous band ---

    #[test]
    fn lapack_failure_is_unmeasured_not_commuting() {
        match read(Err(7), 1.0) {
            terni::Imperfect::Failure(Unmeasured(7), _) => {}
            other => panic!("expected Failure(Unmeasured(7)), got {other:?}"),
        }
    }

    #[test]
    fn numerical_zero_is_dropped_relative_to_scale() {
        // 1e-10 is noise next to a scale of 1e3 (threshold 1e-9), so it is zero.
        match read(Ok(vec![1e-10]), 1e3) {
            terni::Imperfect::Success(cs) => assert!(cs.spectrum.is_empty()),
            other => panic!("expected Success(empty), got {other:?}"),
        }
    }

    #[test]
    fn ambiguous_singular_values_say_dont_know_yet() {
        // Above numerical zero, below clearly nonzero: carried, not dropped.
        match read(Ok(vec![1e-8, 2.0]), 1.0) {
            terni::Imperfect::Partial(cs, ambiguous) => {
                assert_eq!(cs.spectrum, vec![2.0]);
                assert_eq!(ambiguous, vec![1e-8]);
            }
            other => panic!("expected Partial, got {other:?}"),
        }
    }

    #[test]
    fn commutator_of_sheaf_with_itself_is_zero() {
        let k3 = graph_complete_of_order(3);
        let cs = measured(&k3, &k3);
        assert!(
            cs.spectrum.is_empty(),
            "self-commutator must be zero; got {:?}",
            cs.spectrum
        );
    }

    /// Disjoint edges `(0,1)` and `(1,2)` share vertex 1 → non-commutative
    /// composition. Commutator has non-zero singular values.
    ///
    /// Algebra: over V = {0,1,2}, L_ab has edge 0-1; L_bc has edge 1-2.
    /// `[L_ab, L_bc]` is antisymmetric; eigenvalues `{0, ±i√3}`; singular
    /// values `{0, √3, √3}`; filtered spectrum `{√3, √3}`.
    #[test]
    fn commutator_of_disjoint_edges_is_nonzero() {
        let ab = graph_from_edges(&[(0, 1)]);
        let bc = graph_from_edges(&[(1, 2)]);
        let cs = measured(&ab, &bc);
        assert!(
            !cs.spectrum.is_empty(),
            "disjoint-edge commutator must be non-zero (shared vertex 1); got empty"
        );
        for &s in &cs.spectrum {
            assert!(
                (s - 3.0_f64.sqrt()).abs() < 1e-9,
                "non-zero singular value must equal √3 for this topology; got {s}"
            );
        }
    }

    /// Empty sheafs → empty vertex-union → empty commutator spectrum.
    #[test]
    fn commutator_of_empty_sheaves_is_empty() {
        let empty = graph_from_edges(&[]);
        let cs = measured(&empty, &empty);
        assert!(cs.spectrum.is_empty());
    }

    /// Commuting sheafs on disjoint vertex sets: `[L_A, L_B] = 0` because
    /// L_A and L_B act on orthogonal subspaces (their supports don't
    /// overlap). Substrate-reading: no shared substrate ⇒ trivially
    /// commutative composition (nothing to observe).
    #[test]
    fn commutator_of_disjoint_vertex_sets_is_zero() {
        let ab = graph_from_edges(&[(0, 1)]);
        let cd = graph_from_edges(&[(2, 3)]);
        let cs = measured(&ab, &cd);
        assert!(
            cs.spectrum.is_empty(),
            "disjoint-vertex-set commutator must be zero; got {:?}",
            cs.spectrum
        );
    }
}
