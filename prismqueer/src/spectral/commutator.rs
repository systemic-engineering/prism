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
//! Laplacians `L_A` and `L_B` exactly in [`room`](crate::room), both maps
//! of one C⁰(V) (identity is per skeleton: a vertex room is its vertex set
//! alone), and compose them exactly over Z; the commutator is measured once. The commutator
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
//! - `prismqueer::room` — C⁰(V), δ out of it, exact composition
//! - `prismqueer::ffi::singular_values` — LAPACK dgesdd via FLANG-compiled
//!   `native/spectral.f90`

use std::collections::BTreeSet;

use terni::Imperfect;

use crate::room::{coboundary_over, Map, Room, RoomError, Vertices, Z};
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
        if vertices.len() < 2 {
            return Imperfect::Success(Self {
                spectrum: Vec::new(),
            });
        }
        let (commutator, l_a, l_b) = exact_commutator(&Room::over(&vertices), a, b)
            .expect("V is the union of both graphs' vertices: every map is of C⁰(V)");
        let scale = l_a.measure().frobenius() * l_b.measure().frobenius();
        read(commutator.measure().singular_values(), scale)
    }
}

/// [L_a, L_b] = L_a·L_b − L_b·L_a over C⁰(V), exactly, with L_a and L_b.
/// `Err` only when a graph has a vertex outside V.
fn exact_commutator(
    room: &Room<Vertices>,
    a: &Graph,
    b: &Graph,
) -> Result<(Laplacian, Laplacian, Laplacian), RoomError> {
    let laplacian = |g: &Graph| -> Result<Laplacian, RoomError> {
        let delta = coboundary_over(room, g.multigraph())?;
        delta.then(&delta.adjoint())
    };
    let (l_a, l_b) = (laplacian(a)?, laplacian(b)?);
    // Path order: `l_b.then(&l_a)` is L_b first, then L_a: the matrix L_a·L_b.
    let ab = l_b.then(&l_a)?;
    let ba = l_a.then(&l_b)?;
    Ok((ab.plus(&ba.negate())?, l_a, l_b))
}

/// A map of C⁰ to itself, exactly.
type Laplacian = Map<Vertices, Vertices, Z>;

/// LAPACK could not compute the singular values (`info` code).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Unmeasured(pub i32);

/// Classify a singular-value measurement against the operators' scale
/// (product of their Frobenius norms) through the declared band
/// [`zeros::commutator`](crate::band::zeros::commutator): zero below `1e-12`,
/// non-zero from `1e-6`, both times `max(scale, 1)`. In between, the
/// instrument doesn't know yet.
fn read(
    measured: Result<Vec<f64>, i32>,
    scale: f64,
) -> Imperfect<Dissonance, Unmeasured, Vec<f64>> {
    let svs = match measured {
        Ok(svs) => svs,
        Err(info) => return Imperfect::Failure(Unmeasured(info), Vec::new()),
    };
    let band = crate::band::zeros::commutator();
    let mut spectrum = Vec::new();
    let mut ambiguous = Vec::new();
    for s in svs {
        match band.read(s, scale) {
            crate::band::Verdict::NonZero => spectrum.push(s),
            crate::band::Verdict::DontKnow => ambiguous.push(s),
            crate::band::Verdict::Zero => {}
        }
    }
    let cs = Dissonance { spectrum };
    if ambiguous.is_empty() {
        Imperfect::Success(cs)
    } else {
        Imperfect::Partial(cs, ambiguous)
    }
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
