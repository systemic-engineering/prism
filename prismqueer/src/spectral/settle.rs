//! `prismqueer::spectral::settle` — loop 1: where the recursion stops moving.
//!
//! Heat flow `e^{-tL}` on a graph's Laplacian (`D²` restricted to
//! 0-cochains). It converges to `ker L = H⁰`, the global sections
//! (mirror `docs/math/sheaf/laplacian.md`; Hansen & Ghrist 2019). Closed
//! form in the eigenbasis: the future is read off the spectrum, not iterated.
//!
//! The answer is ternary and honest about what it does not know yet:
//!
//! - `Success`: nothing moving, nothing lost. The section was already harmonic.
//! - `Partial`: here is the section now. [`Motion`] says what is still moving
//!   (ahead) and what faded below the observer's [`Resolution`] (behind).
//!   Settled iff nothing is still moving.
//! - `Failure`: the instrument could not measure. Fails closed: an
//!   unmeasured answer is never reported as settled.
//!
//! Scope: vertices only (`H⁰`). Cycles (`H¹`) need the edge Laplacian.

use std::collections::BTreeMap;

use terni::{Imperfect, Loss};

use crate::spectral::commutator::build_laplacian;
use crate::spectral::kleinos::{Graph, VertexId};

/// A value per vertex. Keyed by vertex, not by position.
#[derive(Clone, Debug, PartialEq)]
pub struct Section(BTreeMap<VertexId, f64>);

impl Section {
    /// Build a section from `(vertex, value)` pairs.
    pub fn new(values: impl IntoIterator<Item = (VertexId, f64)>) -> Self {
        Self(values.into_iter().collect())
    }

    /// The value at vertex `v`, if the section has one.
    pub fn get(&self, v: VertexId) -> Option<f64> {
        self.0.get(&v).copied()
    }
}

/// How long the section flows.
#[derive(Copy, Clone, Debug, PartialEq, PartialOrd)]
pub struct Time(pub f64);

/// The observer's resolution: amplitudes below it count as not visible.
#[derive(Copy, Clone, Debug, PartialEq, PartialOrd)]
pub struct Resolution(pub f64);

/// One eigenmode of the flow: its eigenvalue, and its amplitude before and
/// after flowing.
#[derive(Copy, Clone, Debug, PartialEq)]
pub struct Mode {
    /// Eigenvalue of the Laplacian. The mode decays at this rate.
    pub eigenvalue: f64,
    /// Amplitude before the flow.
    pub was: f64,
    /// Amplitude after the flow.
    pub now: f64,
}

/// What the flow has not settled, and what it lost getting here.
///
/// The loss carried by `Partial`. Both halves are kept; neither is averaged
/// into the other.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Motion {
    /// Ahead: modes still visible at the observer's resolution.
    pub moving: Vec<Mode>,
    /// Behind: modes that were visible and faded below the resolution.
    pub faded: Vec<Mode>,
    /// Nothing is known: the measurement itself failed.
    pub opaque: bool,
}

impl Motion {
    /// Whether the recursion has stopped moving at this resolution.
    pub fn settled(&self) -> bool {
        !self.opaque && self.moving.is_empty()
    }
}

impl Loss for Motion {
    fn zero() -> Self {
        Self::default()
    }

    fn total() -> Self {
        Self {
            opaque: true,
            ..Self::default()
        }
    }

    fn is_zero(&self) -> bool {
        !self.opaque && self.moving.is_empty() && self.faded.is_empty()
    }

    fn combine(mut self, other: Self) -> Self {
        if self.opaque || other.opaque {
            return Self::total();
        }
        self.moving.extend(other.moving);
        self.faded.extend(other.faded);
        self
    }
}

/// Why the flow could not be measured.
#[derive(Clone, Debug, PartialEq)]
pub enum Wall {
    /// LAPACK did not converge (`info` code).
    Eigensystem(i32),
    /// A value, the time, or the resolution was not finite, or out of range.
    NonFinite,
    /// The section has no value at this vertex.
    Missing(VertexId),
}

/// A graph's Laplacian eigensystem: vertices in canonical order, eigenvalues
/// ascending, eigenvectors as columns (`vectors[c * n + k]`, per `ffi`).
struct Eigen {
    vertices: Vec<VertexId>,
    values: Vec<f64>,
    vectors: Vec<f64>,
}

impl Eigen {
    fn of(graph: &Graph) -> Result<Self, Wall> {
        let vertices: Vec<VertexId> = graph.vertices().collect();
        let n = vertices.len();
        let index: BTreeMap<VertexId, usize> =
            vertices.iter().enumerate().map(|(i, &v)| (v, i)).collect();
        let laplacian = build_laplacian(graph, &index, n);
        let (values, vectors) =
            crate::ffi::eigensystem(n, &laplacian).map_err(Wall::Eigensystem)?;
        Ok(Self { vertices, values, vectors })
    }

    /// Eigenvalues this close to zero are harmonic. Scaled to the spectrum,
    /// not absolute.
    fn is_harmonic(&self, eigenvalue: f64) -> bool {
        let scale = self.values.iter().fold(1.0_f64, |m, &l| m.max(l.abs()));
        eigenvalue.abs() <= 1e-9 * scale
    }

    fn vector(&self, k: usize) -> Vec<f64> {
        let n = self.vertices.len();
        (0..n).map(|c| self.vectors[c * n + k]).collect()
    }
}

/// Flow `section` over `graph` for time `t` and report, at resolution `eps`,
/// what settled, what is still moving, and what faded getting here.
pub fn settle(
    graph: &Graph,
    section: &Section,
    t: Time,
    eps: Resolution,
) -> Imperfect<Section, Wall, Motion> {
    if !t.0.is_finite() || t.0 < 0.0 || !eps.0.is_finite() || eps.0 <= 0.0 {
        return Imperfect::Failure(Wall::NonFinite, Motion::total());
    }
    let eigen = match Eigen::of(graph) {
        Ok(e) => e,
        Err(wall) => return Imperfect::Failure(wall, Motion::total()),
    };
    let mut psi = Vec::with_capacity(eigen.vertices.len());
    for &v in &eigen.vertices {
        match section.get(v) {
            Some(x) if x.is_finite() => psi.push(x),
            Some(_) => return Imperfect::Failure(Wall::NonFinite, Motion::total()),
            None => return Imperfect::Failure(Wall::Missing(v), Motion::total()),
        }
    }

    let n = psi.len();
    let mut out = vec![0.0_f64; n];
    let mut motion = Motion::zero();
    for (k, &eigenvalue) in eigen.values.iter().enumerate() {
        let v = eigen.vector(k);
        let was: f64 = v.iter().zip(&psi).map(|(a, b)| a * b).sum();
        let harmonic = eigen.is_harmonic(eigenvalue);
        let now = if harmonic { was } else { was * (-eigenvalue * t.0).exp() };
        for (o, &vi) in out.iter_mut().zip(&v) {
            *o += now * vi;
        }
        if harmonic {
            continue;
        }
        let mode = Mode { eigenvalue, was, now };
        if now.abs() >= eps.0 {
            motion.moving.push(mode);
        } else if was.abs() >= eps.0 {
            motion.faded.push(mode);
        }
    }

    let now = Section::new(eigen.vertices.iter().copied().zip(out));
    if motion.is_zero() {
        Imperfect::Success(now)
    } else {
        Imperfect::Partial(now, motion)
    }
}

/// Dimension of the harmonic (settled) subspace: `dim ker L`.
#[cfg(test)]
fn harmonic_dimension(graph: &Graph) -> Option<usize> {
    let eigen = Eigen::of(graph).ok()?;
    Some(eigen.values.iter().filter(|&&l| eigen.is_harmonic(l)).count())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::spectral::kleinos::graph_from_edges;
    use crate::spectral::tension::{detect_tensions, TensionKind};

    const EPS: Resolution = Resolution(1e-6);

    fn section(values: &[f64]) -> Section {
        Section::new(values.iter().enumerate().map(|(i, &x)| (i as VertexId, x)))
    }

    fn values(s: &Section, n: usize) -> Vec<f64> {
        (0..n).map(|i| s.get(i as VertexId).expect("vertex present")).collect()
    }

    fn close(a: &[f64], b: &[f64]) -> bool {
        a.len() == b.len() && a.iter().zip(b).all(|(x, y)| (x - y).abs() < 1e-9)
    }

    fn flow(graph: &Graph, s: &Section, t: f64) -> Section {
        settle(graph, s, Time(t), EPS).ok().expect("flow measured")
    }

    fn path_centered_at_1() -> Graph {
        graph_from_edges(&[(0, 1), (1, 2)])
    }

    fn path_centered_at_0() -> Graph {
        graph_from_edges(&[(0, 1), (0, 2)])
    }

    // --- Loop 1: where the recursion stops moving ---

    #[test]
    fn harmonic_start_is_success() {
        let g = path_centered_at_1();
        match settle(&g, &section(&[2.0, 2.0, 2.0]), Time(1.0), EPS) {
            Imperfect::Success(s) => assert!(close(&values(&s, 3), &[2.0, 2.0, 2.0])),
            other => panic!("expected Success, got {other:?}"),
        }
    }

    #[test]
    fn connected_graph_settles_to_the_mean_and_reports_what_faded() {
        let g = path_centered_at_1();
        match settle(&g, &section(&[3.0, 0.0, 0.0]), Time(100.0), EPS) {
            Imperfect::Partial(s, motion) => {
                assert!(motion.settled(), "nothing should still be moving");
                assert!(!motion.faded.is_empty(), "the non-harmonic detail faded");
                assert!(close(&values(&s, 3), &[1.0, 1.0, 1.0]));
            }
            other => panic!("expected Partial, got {other:?}"),
        }
    }

    #[test]
    fn short_flow_says_i_dont_know_yet() {
        let g = path_centered_at_1();
        match settle(&g, &section(&[3.0, 0.0, 0.0]), Time(0.01), EPS) {
            Imperfect::Partial(_, motion) => {
                assert!(!motion.settled(), "the recursion is still moving");
                assert!(!motion.moving.is_empty());
            }
            other => panic!("expected Partial, got {other:?}"),
        }
    }

    #[test]
    fn resuming_equals_not_stopping() {
        let g = path_centered_at_1();
        let start = section(&[3.0, 0.0, 0.0]);
        let resumed = flow(&g, &flow(&g, &start, 0.3), 0.5);
        let straight = flow(&g, &start, 0.8);
        assert!(close(&values(&resumed, 3), &values(&straight, 3)));
    }

    #[test]
    fn disconnected_graph_settles_per_component() {
        let g = graph_from_edges(&[(0, 1), (2, 3)]);
        let s = flow(&g, &section(&[1.0, 0.0, 0.0, 0.0]), 100.0);
        assert!(close(&values(&s, 4), &[0.5, 0.5, 0.0, 0.0]));
    }

    #[test]
    fn settled_dimension_agrees_with_tension_components() {
        // Two instruments, one fact: dim ker L == number of components.
        let g = graph_from_edges(&[(0, 1), (2, 3), (4, 5)]);
        let components = detect_tensions(&g)
            .ok()
            .expect("tension measured")
            .into_iter()
            .find_map(|t| match t.kind {
                TensionKind::Disconnection { component_count } => Some(component_count),
                _ => None,
            })
            .expect("disconnection reported");
        assert_eq!(harmonic_dimension(&g), Some(components));
        assert_eq!(components, 3);
    }

    #[test]
    fn non_finite_input_fails_closed() {
        let g = path_centered_at_1();
        match settle(&g, &section(&[f64::NAN, 0.0, 0.0]), Time(1.0), EPS) {
            Imperfect::Failure(Wall::NonFinite, motion) => assert!(motion.opaque),
            other => panic!("expected Failure(NonFinite), got {other:?}"),
        }
    }

    #[test]
    fn missing_vertex_fails_closed() {
        let g = path_centered_at_1();
        match settle(&g, &section(&[1.0, 0.0]), Time(1.0), EPS) {
            Imperfect::Failure(Wall::Missing(2), motion) => assert!(motion.opaque),
            other => panic!("expected Failure(Missing(2)), got {other:?}"),
        }
    }

    #[test]
    fn motion_is_a_lawful_loss() {
        let a = Motion {
            moving: vec![Mode { eigenvalue: 1.0, was: 1.0, now: 0.5 }],
            ..Motion::zero()
        };
        assert_eq!(Motion::zero().combine(a.clone()), a);
        assert_eq!(a.clone().combine(Motion::zero()), a);
        assert_eq!(Motion::total().combine(a.clone()), Motion::total());
        assert_eq!(a.combine(Motion::total()), Motion::total());
    }

    // --- Standing question: the smallest nonlinear falsification in linear tests ---
    //
    // Claim: the process is nonlinear because the operator itself changes
    // (rewiring the graph changes L). Witness: the order of two flows matters.
    // Refutation: if flows under different operators always commuted, one
    // linear flow would explain everything, and the claim is false.

    #[test]
    fn control_same_operator_flows_commute() {
        let g = path_centered_at_1();
        let start = section(&[3.0, 0.0, 1.0]);
        let ab = flow(&g, &flow(&g, &start, 0.2), 0.7);
        let ba = flow(&g, &flow(&g, &start, 0.7), 0.2);
        assert!(close(&values(&ab, 3), &values(&ba, 3)));
    }

    #[test]
    fn rewiring_makes_order_matter() {
        let (g1, g2) = (path_centered_at_1(), path_centered_at_0());
        let start = section(&[3.0, 0.0, 1.0]);
        let ab = flow(&g2, &flow(&g1, &start, 0.3), 0.3);
        let ba = flow(&g1, &flow(&g2, &start, 0.3), 0.3);
        assert!(
            !close(&values(&ab, 3), &values(&ba, 3)),
            "flows under different operators commuted: no nonlinearity witnessed"
        );
    }

    #[test]
    fn control_the_complete_graph_commutes_with_any_rewiring() {
        // L(K_n) = nI - J commutes with every Laplacian on the same vertices,
        // because J·L = L·J = 0. At the Splinter pole, order stops mattering.
        let (g1, k3) = (path_centered_at_1(), graph_from_edges(&[(0, 1), (1, 2), (0, 2)]));
        let start = section(&[3.0, 0.0, 1.0]);
        let ab = flow(&k3, &flow(&g1, &start, 0.3), 0.3);
        let ba = flow(&g1, &flow(&k3, &start, 0.3), 0.3);
        assert!(close(&values(&ab, 3), &values(&ba, 3)));
    }
}
