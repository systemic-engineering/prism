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
//! - `Partial`: here is the section now. [`Tail`] says what is still moving
//!   (ahead) and what faded below the observer's [`Noise`] (behind).
//!   Settled iff nothing is still moving.
//! - `Failure`: the instrument could not measure. Fails closed: an
//!   unmeasured answer is never reported as settled.
//!
//! Scope: vertices only (`H⁰`). The edge side (`H¹`, the comma) is
//! [`Instrument::decay`](crate::spectral::instrument::Instrument::decay) on the
//! full complex, and its exact split is
//! [`Instrument::hodge`](crate::spectral::instrument::Instrument::hodge).

use std::collections::BTreeMap;

use terni::{Imperfect, Loss};

use crate::room::{laplacian, Modes, RoomError, Vertices};
use crate::spectral::kleinos::{Graph, VertexId};

/// A value per vertex. Keyed by vertex, not by position.
#[derive(Clone, Debug, PartialEq)]
pub struct Pitches(BTreeMap<VertexId, f64>);

impl Pitches {
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
pub struct Noise(pub f64);

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
pub struct Tail {
    /// Ahead: modes still visible at the observer's resolution.
    pub moving: Vec<Mode>,
    /// Behind: modes that were visible and faded below the resolution.
    pub faded: Vec<Mode>,
    /// Nothing is known: the measurement itself failed.
    pub opaque: bool,
}

impl Tail {
    /// Whether the recursion has stopped moving at this resolution.
    pub fn settled(&self) -> bool {
        !self.opaque && self.moving.is_empty()
    }
}

impl Loss for Tail {
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
    /// LAPACK could not certify the kernel apart from the slowest mode: its
    /// error cannot resolve the spectral gap at this size, or its spectrum
    /// contradicts the component count.
    Unseparated {
        /// Connected components, counted exactly.
        components: usize,
    },
}

/// Connected components, counted by union-find: the exact size of the kernel.
fn components(graph: &Graph, index: &BTreeMap<VertexId, usize>) -> usize {
    fn root(parent: &mut [usize], i: usize) -> usize {
        let mut r = i;
        while parent[r] != r {
            r = parent[r];
        }
        let mut j = i;
        while parent[j] != r {
            let next = parent[j];
            parent[j] = r;
            j = next;
        }
        r
    }
    let mut parent: Vec<usize> = (0..index.len()).collect();
    let mut count = index.len();
    for (u, v) in graph.edges() {
        let (ru, rv) = (root(&mut parent, index[&u]), root(&mut parent, index[&v]));
        if ru != rv {
            parent[ru] = rv;
            count -= 1;
        }
    }
    count
}

/// How many eigenvalues are the kernel, certified.
///
/// `components` decides it: the first `components` eigenvalues are the kernel
/// by index, with no threshold. LAPACK only has to agree. Those eigenvalues
/// must sit within its error of zero, and the next must clear the gap every
/// connected simple graph on at most `n` vertices has: λ₂ ≥ 1/(n·diam) ≥
/// 1/(n(n−1)). If the error cannot resolve that gap, or LAPACK disagrees,
/// the kernel is not certified and nothing is called settled.
fn kernel(values: &[f64], components: usize, n: usize) -> Result<usize, Wall> {
    let unseparated = Err(Wall::Unseparated { components });
    if components > values.len() {
        return unseparated;
    }
    let scale = values.iter().fold(1.0_f64, |m, l| m.max(l.abs()));
    let error = 16.0 * f64::EPSILON * scale * n.max(1) as f64;
    if values[..components].iter().any(|l| l.abs() > error) {
        return unseparated;
    }
    if let Some(&slowest) = values.get(components) {
        let gap = 1.0 / (n as f64 * (n as f64 - 1.0));
        if error >= gap / 2.0 || slowest < gap - error {
            return unseparated;
        }
    }
    Ok(components)
}

/// A graph's Laplacian eigensystem: vertices in canonical order, and the
/// [`Modes`] of L₀ = δ*δ on C⁰, built exactly in [`room`](crate::room) and
/// measured once. Eigenvectors are read only through [`Modes::vector`].
struct Eigen {
    vertices: Vec<VertexId>,
    modes: Modes<Vertices>,
    /// The first `harmonic` eigenvalues are the kernel, certified.
    harmonic: usize,
}

impl Eigen {
    fn of(graph: &Graph) -> Result<Self, Wall> {
        let vertices: Vec<VertexId> = graph.vertices().collect();
        let n = vertices.len();
        let index: BTreeMap<VertexId, usize> =
            vertices.iter().enumerate().map(|(i, &v)| (v, i)).collect();
        let modes = laplacian(graph.multigraph())
            .measure()
            .decompose()
            .map_err(|e| match e {
                RoomError::Unmeasured(info) => Wall::Eigensystem(info),
                other => unreachable!("decompose fails only unmeasured: {other:?}"),
            })?;
        let harmonic = kernel(modes.spectrum(), components(graph, &index), n)?;
        Ok(Self {
            vertices,
            modes,
            harmonic,
        })
    }

    /// The eigenvalues, ascending.
    fn values(&self) -> &[f64] {
        self.modes.spectrum()
    }

    /// Whether mode `k` is in the kernel. Decided by index, not by threshold.
    fn is_harmonic(&self, k: usize) -> bool {
        k < self.harmonic
    }

    /// Eigenvector `k`, a vector of C⁰.
    fn vector(&self, k: usize) -> &[f64] {
        self.modes
            .vector(k)
            .expect("one eigenvector per eigenvalue")
    }
}

/// Flow `section` over `graph` for time `t` and report, at resolution `eps`,
/// what settled, what is still moving, and what faded getting here.
pub fn settle(
    graph: &Graph,
    section: &Pitches,
    t: Time,
    eps: Noise,
) -> Imperfect<Pitches, Wall, Tail> {
    if !t.0.is_finite() || t.0 < 0.0 || !eps.0.is_finite() || eps.0 <= 0.0 {
        return Imperfect::Failure(Wall::NonFinite, Tail::total());
    }
    let eigen = match Eigen::of(graph) {
        Ok(e) => e,
        Err(wall) => return Imperfect::Failure(wall, Tail::total()),
    };
    let mut psi = Vec::with_capacity(eigen.vertices.len());
    for &v in &eigen.vertices {
        match section.get(v) {
            Some(x) if x.is_finite() => psi.push(x),
            Some(_) => return Imperfect::Failure(Wall::NonFinite, Tail::total()),
            None => return Imperfect::Failure(Wall::Missing(v), Tail::total()),
        }
    }

    let n = psi.len();
    let mut out = vec![0.0_f64; n];
    let mut motion = Tail::zero();
    for (k, &eigenvalue) in eigen.values().iter().enumerate() {
        let v = eigen.vector(k);
        let was: f64 = v.iter().zip(&psi).map(|(a, b)| a * b).sum();
        let harmonic = eigen.is_harmonic(k);
        let now = if harmonic {
            was
        } else {
            was * (-eigenvalue * t.0).exp()
        };
        for (o, &vi) in out.iter_mut().zip(v) {
            *o += now * vi;
        }
        if harmonic {
            continue;
        }
        let mode = Mode {
            eigenvalue,
            was,
            now,
        };
        if now.abs() >= eps.0 {
            motion.moving.push(mode);
        } else if was.abs() >= eps.0 {
            motion.faded.push(mode);
        }
    }

    let now = Pitches::new(eigen.vertices.iter().copied().zip(out));
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
    Some(eigen.harmonic)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::spectral::kleinos::graph_from_edges;
    use crate::spectral::tension::{detect_tensions, TensionKind};

    const EPS: Noise = Noise(1e-6);

    fn section(values: &[f64]) -> Pitches {
        Pitches::new(values.iter().enumerate().map(|(i, &x)| (i as VertexId, x)))
    }

    fn values(s: &Pitches, n: usize) -> Vec<f64> {
        (0..n)
            .map(|i| s.get(i as VertexId).expect("vertex present"))
            .collect()
    }

    fn close(a: &[f64], b: &[f64]) -> bool {
        a.len() == b.len() && a.iter().zip(b).all(|(x, y)| (x - y).abs() < 1e-9)
    }

    fn flow(graph: &Graph, s: &Pitches, t: f64) -> Pitches {
        settle(graph, s, Time(t), EPS).ok().expect("flow measured")
    }

    fn path_centered_at_1() -> Graph {
        graph_from_edges(&[(0, 1), (1, 2)])
    }

    fn path_centered_at_0() -> Graph {
        graph_from_edges(&[(0, 1), (0, 2)])
    }

    // --- S3: the kernel is decided exactly, LAPACK only certifies it ---

    #[test]
    fn exact_components_decide_the_kernel() {
        assert_eq!(kernel(&[0.0, 1e-17, 0.7, 1.2], 2, 4), Ok(2));
    }

    #[test]
    fn beyond_certifiable_size_a_slow_mode_fails_closed() {
        // The old tolerance (1e-9 · λ_max = 4e-9) would call 1e-10 harmonic and
        // report "settled". At this size the gap 1/(n(n−1)) is below LAPACK's
        // error, so the only honest answer is that it cannot tell.
        assert_eq!(
            kernel(&[0.0, 1e-10, 4.0], 1, 100_000),
            Err(Wall::Unseparated { components: 1 })
        );
    }

    #[test]
    fn lapack_contradicting_the_components_fails_closed() {
        // One component, but a second eigenvalue far below the proven gap.
        assert_eq!(
            kernel(&[0.0, 3e-9, 4.0], 1, 3),
            Err(Wall::Unseparated { components: 1 })
        );
        // One component, but its kernel eigenvalue is not zero.
        assert_eq!(
            kernel(&[0.3, 1.0], 1, 2),
            Err(Wall::Unseparated { components: 1 })
        );
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
        let a = Tail {
            moving: vec![Mode {
                eigenvalue: 1.0,
                was: 1.0,
                now: 0.5,
            }],
            ..Tail::zero()
        };
        assert_eq!(Tail::zero().combine(a.clone()), a);
        assert_eq!(a.clone().combine(Tail::zero()), a);
        assert_eq!(Tail::total().combine(a.clone()), Tail::total());
        assert_eq!(a.combine(Tail::total()), Tail::total());
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
        let (g1, k3) = (
            path_centered_at_1(),
            graph_from_edges(&[(0, 1), (1, 2), (0, 2)]),
        );
        let start = section(&[3.0, 0.0, 1.0]);
        let ab = flow(&k3, &flow(&g1, &start, 0.3), 0.3);
        let ba = flow(&g1, &flow(&k3, &start, 0.3), 0.3);
        assert!(close(&values(&ab, 3), &values(&ba, 3)));
    }
}

/// Loop 4 around loop 1: the commutator claim over sampled graphs.
///
/// Theorem (symmetric A, B; s, t > 0): e^{-sA} e^{-tB} = e^{-tB} e^{-sA}
/// iff [A, B] = 0. exp is injective on the reals, so e^{-sA} has the same
/// eigenprojectors as A; commuting symmetric matrices share an eigenbasis.
/// So the graph commutator (measured on graphs) and flow commutation
/// (measured by execution, on every basis section) must agree on every pair.
#[cfg(all(test, feature = "bundle"))]
mod rewiring_property {
    use super::*;
    use crate::flux::pillar::{Arbitrary, Sample};
    use crate::spectral::commutator::Dissonance;
    use crate::spectral::kleinos::graph_from_edges;

    const S: f64 = 0.5;
    const BUDGET: usize = 256;

    /// Two graphs on the same vertex set {0..n-1}, every vertex covered.
    #[derive(Debug)]
    struct Rewiring {
        n: usize,
        before: Graph,
        after: Graph,
    }

    /// Bernoulli(1/2) edges, then any uncovered vertex v gets edge (v, v+1 mod n).
    /// Biased toward covered graphs by design: a Graph cannot hold isolated vertices.
    fn covered_graph(sample: &mut Sample, n: usize) -> Graph {
        let mut edges = Vec::new();
        for i in 0..n {
            for j in (i + 1)..n {
                if sample.draw_bool() {
                    edges.push((i as VertexId, j as VertexId));
                }
            }
        }
        for v in 0..n as VertexId {
            if !edges.iter().any(|&(a, b)| a == v || b == v) {
                let w = ((v as usize + 1) % n) as VertexId;
                edges.push((v.min(w), v.max(w)));
            }
        }
        graph_from_edges(&edges)
    }

    impl Arbitrary for Rewiring {
        fn arbitrary(sample: &mut Sample) -> Self {
            let n = sample.draw_integer(2, 6) as usize;
            let before = covered_graph(sample, n);
            let after = covered_graph(sample, n);
            Rewiring { n, before, after }
        }
    }

    fn seeded(seed: u64) -> Rewiring {
        Rewiring::arbitrary(&mut Sample::from_bytes(seed.to_le_bytes().to_vec()))
    }

    fn flow(g: &Graph, s: &Pitches) -> Pitches {
        settle(g, s, Time(S), Noise(1e-6))
            .ok()
            .expect("flow measured")
    }

    fn basis(n: usize, i: usize) -> Pitches {
        Pitches::new((0..n).map(|v| (v as VertexId, if v == i { 1.0 } else { 0.0 })))
    }

    /// Operator equality, checked on every basis section.
    fn flows_commute(r: &Rewiring) -> bool {
        (0..r.n).all(|i| {
            let e = basis(r.n, i);
            let ab = flow(&r.after, &flow(&r.before, &e));
            let ba = flow(&r.before, &flow(&r.after, &e));
            (0..r.n as VertexId).all(|v| (ab.get(v).unwrap() - ba.get(v).unwrap()).abs() < 1e-9)
        })
    }

    /// Partial((commuting, not), budget): checked `budget` pairs, universal
    /// claim not settled by sampling. Failure: a counterexample, or the
    /// instrument could not decide, or the sample was vacuous.
    fn check(budget: usize) -> Imperfect<(usize, usize), String, usize> {
        let (mut commuting, mut not, mut nontrivial) = (0, 0, 0);
        for seed in 0..budget as u64 {
            let r = seeded(seed);
            let by_graph = match Dissonance::of(&r.before, &r.after) {
                Imperfect::Success(cs) => cs.spectrum.is_empty(),
                other => {
                    return Imperfect::Failure(
                        format!("seed {seed}: instrument undecided: {other:?}"),
                        budget,
                    )
                }
            };
            let by_execution = flows_commute(&r);
            if by_graph != by_execution {
                return Imperfect::Failure(
                    format!(
                        "seed {seed}: graph says {by_graph}, execution says {by_execution}: {r:?}"
                    ),
                    budget,
                );
            }
            if by_graph {
                commuting += 1;
                // Trivially commuting: two vertices (one possible graph) or identical graphs.
                if r.n > 2 && r.before != r.after {
                    nontrivial += 1;
                }
            } else {
                not += 1;
            }
        }
        eprintln!("commuting={commuting} (nontrivial={nontrivial}), not={not}");
        if nontrivial == 0 || not == 0 {
            return Imperfect::Failure(
                format!("vacuous: nontrivial commuting={nontrivial}, not={not}"),
                budget,
            );
        }
        Imperfect::Partial((commuting, not), budget)
    }

    #[test]
    fn sampler_covers_every_vertex() {
        for seed in 0..BUDGET as u64 {
            let r = seeded(seed);
            assert_eq!(r.before.vertex_count(), r.n, "seed {seed}");
            assert_eq!(r.after.vertex_count(), r.n, "seed {seed}");
        }
    }

    #[test]
    fn graph_commutator_and_execution_order_agree() {
        match check(BUDGET) {
            Imperfect::Partial((commuting, not), budget) => {
                assert_eq!(commuting + not, budget);
                eprintln!("agreed on {budget} pairs: {commuting} commuting, {not} not");
            }
            Imperfect::Failure(why, _) => panic!("falsified: {why}"),
            Imperfect::Success(_) => unreachable!("sampling never proves a universal claim"),
        }
    }
}
