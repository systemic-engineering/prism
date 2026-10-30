//! `prismqueer::spectral::instrument` — the Hodge–Dirac operator of a graph.
//!
//! Punk names (ratified 2026-09-28): the operator is the [`Instrument`] (its
//! shape decides the sound); vertex values are pitches, edge values are
//! intervals (δ of the pitches); H⁰ is the tonic, H¹ is the comma (a
//! disagreement no tuning produces); e^{-tD²} is `decay`, e^{itD} is `ring`.
//!
//! H = C⁰ ⊕ C¹: values on vertices (assertions) and on oriented edges
//! (questions). D = δ + δ*, self-adjoint and odd: it maps vertices to edges
//! and back. D² = Δ₀ ⊕ Δ₁, and Δ₀ is the Laplacian `settle` flows under.
//!
//! ker D = H⁰ ⊕ H¹ is Frame Cohomology: where the recursion stops moving.
//! H⁰ holds settled assertions; H¹ holds disagreements that are not the
//! question of any assertion. Heat e^{-tD²} settles onto ker D and keeps both.
//! Rotation e^{itD} alternates assertion and question and never settles.
//! Both are functions of one D, so they commute.
//!
//! Spec: `docs/loop/CURRENT.md`; the correction to the Glasswall triple
//! (H is cochains, not cohomology) is argued there.

use std::collections::{BTreeMap, BTreeSet};

use crate::room::{self, Edges, Map, Modes, RoomError, Sum, Vertices};
use crate::spectral::commutator::Unmeasured;
use crate::spectral::kleinos::{EdgeKey, Graph, VertexId};
use crate::spectral::settle::Time;

/// How far to rotate: the parameter of e^{i·angle·D}.
#[derive(Copy, Clone, Debug, PartialEq, PartialOrd)]
pub struct Angle(pub f64);

/// A cochain named a vertex or edge the complex does not have.
#[derive(Copy, Clone, Debug, PartialEq)]
pub enum Outside {
    /// No such vertex.
    Vertex(VertexId),
    /// No such edge, in either orientation.
    Edge(EdgeKey),
    /// An operation on intervals was handed a pitch value.
    NotAnInterval(VertexId),
    /// An eigenvalue sits on a window edge, within LAPACK's error: whether
    /// its mode is inside the window can't be decided.
    Straddles(f64),
}

/// A spectral window `[low, high]` over D's eigenvalues: which frequencies a
/// restriction keeps.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Window {
    low: f64,
    high: f64,
}

impl Window {
    /// The closed window `[low, high]`. Panics if `low > high`.
    pub fn new(low: f64, high: f64) -> Self {
        assert!(low <= high, "a window is low ≤ high");
        Window { low, high }
    }
}

/// A restriction to a window: what it kept and what it erased. The pair is
/// lossless (`kept + erased` is the input); the restriction alone isn't.
#[derive(Clone, Debug, PartialEq)]
pub struct Restriction {
    kept: Cochain,
    erased: Cochain,
    lossless: bool,
}

impl Restriction {
    /// The modes inside the window.
    pub fn kept(&self) -> &Cochain {
        &self.kept
    }
    /// The modes outside it: the erasure.
    pub fn erased(&self) -> &Cochain {
        &self.erased
    }
    /// Whether nothing outside the window carried any weight.
    pub fn is_lossless(&self) -> bool {
        self.lossless
    }
}

/// Intervals split into what an assertion could ask (gradient, im δ) and
/// what no assertion answers (harmonic, ker δ*: the comma). Complementary
/// orthogonal projections; `gradient + harmonic` is the input.
#[derive(Clone, Debug, PartialEq)]
pub struct Hodge {
    gradient: Cochain,
    harmonic: Cochain,
}

impl Hodge {
    /// The part that is the question of some assertion.
    pub fn gradient(&self) -> &Cochain {
        &self.gradient
    }
    /// The part no assertion answers: what the edge settle keeps.
    pub fn harmonic(&self) -> &Cochain {
        &self.harmonic
    }
}

/// A value per vertex and per oriented edge. Sparse: an absent entry is zero.
///
/// Edges are stored in canonical orientation (lower id first). A value given
/// on `(v, u)` is the negative of the same value on `(u, v)`.
#[derive(Clone, Debug, PartialEq)]
pub struct Cochain {
    vertex: BTreeMap<VertexId, f64>,
    edge: BTreeMap<EdgeKey, f64>,
}

fn oriented(e: EdgeKey) -> (EdgeKey, f64) {
    if e.0 <= e.1 {
        (e, 1.0)
    } else {
        ((e.1, e.0), -1.0)
    }
}

impl Cochain {
    /// Build a cochain from vertex values and oriented edge values.
    pub fn new(
        vertex: impl IntoIterator<Item = (VertexId, f64)>,
        edge: impl IntoIterator<Item = (EdgeKey, f64)>,
    ) -> Self {
        let mut edges = BTreeMap::new();
        for (e, x) in edge {
            let (key, sign) = oriented(e);
            *edges.entry(key).or_insert(0.0) += sign * x;
        }
        Self {
            vertex: vertex.into_iter().collect(),
            edge: edges,
        }
    }

    /// The value on a vertex, if present.
    pub fn pitch(&self, v: VertexId) -> Option<f64> {
        self.vertex.get(&v).copied()
    }

    /// The value on an oriented edge, if present.
    pub fn interval(&self, e: EdgeKey) -> Option<f64> {
        let (key, sign) = oriented(e);
        self.edge.get(&key).map(|x| sign * x)
    }

    /// Euclidean norm over vertices and edges.
    pub fn norm(&self) -> f64 {
        self.vertex
            .values()
            .chain(self.edge.values())
            .map(|x| x * x)
            .sum::<f64>()
            .sqrt()
    }

    /// Euclidean distance; absent entries count as zero.
    pub fn distance(&self, other: &Self) -> f64 {
        fn part<K: Ord + Copy>(a: &BTreeMap<K, f64>, b: &BTreeMap<K, f64>) -> f64 {
            let keys: std::collections::BTreeSet<K> = a.keys().chain(b.keys()).copied().collect();
            keys.iter()
                .map(|k| a.get(k).unwrap_or(&0.0) - b.get(k).unwrap_or(&0.0))
                .map(|d| d * d)
                .sum()
        }
        (part(&self.vertex, &other.vertex) + part(&self.edge, &other.edge)).sqrt()
    }
}

/// A region to attend to: the indicator of a set of vertices, an element of
/// the algebra A.
///
/// It acts on pitches by the indicator and on intervals by the mean of the
/// two endpoints. So it is self-adjoint, and it fixes every interval that lies
/// wholly inside or wholly outside the region. Only intervals that cross the
/// edge of attention are half-soloed.
///
/// So a solo **attenuates**; it doesn't restrict. On a crossing interval it
/// weighs ½, applied twice ¼: not idempotent, not a projector. To restrict
/// to frequencies, use a [`Window`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Solo(BTreeSet<VertexId>);

impl Solo {
    /// Solo these vertices.
    pub fn new(vertices: impl IntoIterator<Item = VertexId>) -> Self {
        Self(vertices.into_iter().collect())
    }
}

/// [Instrument, Solo]: where attending and questioning fail to commute.
///
/// Non-zero exactly on the intervals that cross the edge of attention.
pub struct Bleed {
    matrix: Vec<f64>,
    edges: BTreeSet<EdgeKey>,
}

impl Bleed {
    /// The commutator, row-major, in the instrument's basis.
    pub fn matrix(&self) -> &[f64] {
        &self.matrix
    }

    /// The intervals it bleeds across.
    pub fn edges(&self) -> BTreeSet<EdgeKey> {
        self.edges.clone()
    }
}

/// e^{i·angle·D} applied to a real cochain: its real and imaginary parts.
#[derive(Clone, Debug, PartialEq)]
pub struct Rotated {
    /// cos(angle·D) x.
    pub real: Cochain,
    /// sin(angle·D) x.
    pub imag: Cochain,
}

/// Certify an exact kernel count against a measured spectrum.
///
/// `count` is b₀ + b₁ from union-find: exact, never in doubt. The spectrum
/// is a measurement, and it agrees when the `count` smallest |eigenvalues|
/// sit within LAPACK's error of zero and the rest clear the proven gap. D's
/// non-zero eigenvalues are ±√λ of a Laplacian, and every connected simple
/// graph on at most `n_vertices` vertices has λ₂ ≥ 1/(n(n−1)), so the gap
/// is 1/√(n(n−1)). Disagreement is `Partial(count, the ambiguous
/// magnitudes)`: the count stands, the measurement doesn't back it.
///
/// "Within LAPACK's error" is read through the declared band
/// [`zeros::instrument`](crate::band::zeros::instrument), never inlined: a
/// kernel mode is certified when the band reads it `Zero`, and a mode short
/// of the gap clears it when the band reads its shortfall `Zero`.
fn certify(
    values: &[f64],
    count: usize,
    n_vertices: usize,
    dim: usize,
) -> terni::Imperfect<usize, std::convert::Infallible, Vec<f64>> {
    use crate::band::{zeros, Verdict};
    let mut magnitudes: Vec<f64> = values.iter().map(|x| x.abs()).collect();
    magnitudes.sort_by(f64::total_cmp);
    let scale = magnitudes.last().copied().unwrap_or(0.0);
    let band = zeros::instrument(dim.max(1));
    let zero = |m: f64| band.read(m, scale) == Verdict::Zero;
    let gap = if n_vertices >= 2 {
        1.0 / ((n_vertices * (n_vertices - 1)) as f64).sqrt()
    } else {
        f64::INFINITY
    };
    let ambiguous: Vec<f64> = magnitudes
        .iter()
        .enumerate()
        .filter(|&(i, &m)| {
            if i < count {
                !zero(m)
            } else {
                m < gap && !zero(gap - m)
            }
        })
        .map(|(_, &m)| m)
        .collect();
    if ambiguous.is_empty() {
        terni::Imperfect::Success(count)
    } else {
        terni::Imperfect::Partial(count, ambiguous)
    }
}

/// C⁰ ⊕ C¹: where D lives.
type Chains = Sum<Vertices, Edges>;

/// The Hodge–Dirac operator of a graph, with its eigensystem.
///
/// D is [`room::dirac`], measured once; its modes come from
/// [`Symmetric::decompose`](crate::room::Symmetric::decompose) and are read
/// only through [`Modes::vector`]. The coordinate maps below are the room's
/// basis order: vertices first, then edges in canonical order.
pub struct Instrument {
    vertices: BTreeMap<VertexId, usize>,
    edges: BTreeMap<EdgeKey, usize>,
    dirac: Map<Chains, Chains, f64>,
    /// D row-major, for [`Instrument::matrix`] only: nothing inside reads it.
    matrix: Vec<f64>,
    modes: Modes<Chains>,
}

impl Instrument {
    /// Build D = δ + δ* for `graph` and measure its eigensystem.
    ///
    /// Fails closed: if LAPACK cannot measure, there is no operator.
    pub fn of(graph: &Graph) -> Result<Self, Unmeasured> {
        let vertices: BTreeMap<VertexId, usize> =
            graph.vertices().enumerate().map(|(i, v)| (v, i)).collect();
        let n0 = vertices.len();
        let edges: BTreeMap<EdgeKey, usize> = graph
            .edges()
            .enumerate()
            .map(|(k, e)| (e, n0 + k))
            .collect();
        // (δs)(u, v) = s(v) − s(u); D holds δ below and δ* above.
        let dirac = room::dirac(graph).measure();
        let modes = dirac
            .clone()
            .symmetric()
            .expect("D = δ + δ* is self-adjoint")
            .decompose()
            .map_err(|e| match e {
                RoomError::Unmeasured(info) => Unmeasured(info),
                other => unreachable!("decompose only fails to measure: {other:?}"),
            })?;
        let n = dirac.to().dim();
        let matrix = (0..n)
            .flat_map(|i| (0..n).map(move |j| (i, j)))
            .map(|(i, j)| *dirac.entry(i, j).expect("inside D"))
            .collect();
        Ok(Self {
            vertices,
            edges,
            dirac,
            matrix,
            modes,
        })
    }

    fn dim(&self) -> usize {
        self.vertices.len() + self.edges.len()
    }

    /// Entry `(row, col)` of D.
    fn d(&self, row: usize, col: usize) -> f64 {
        *self.dirac.entry(row, col).expect("inside D")
    }

    /// The modes as (eigenvalue, eigenvector), in spectrum order.
    fn modes(&self) -> impl Iterator<Item = (f64, &[f64])> {
        self.modes
            .spectrum()
            .iter()
            .copied()
            .zip(self.modes.vectors())
    }

    /// D, row-major, vertices first, then edges in canonical order.
    pub fn matrix(&self) -> &[f64] {
        &self.matrix
    }

    /// The spectrum of D.
    pub fn eigenvalues(&self) -> &[f64] {
        self.modes.spectrum()
    }

    /// dim ker D = b₀ + b₁: components plus independent cycles.
    ///
    /// Exact, by union-find; LAPACK certifies it. `Success(count)` when the
    /// measured spectrum separates at the proven gap, `Partial(count,
    /// ambiguous magnitudes)` when it doesn't. Never a count from a threshold.
    pub fn kernel_dimension(&self) -> terni::Imperfect<usize, std::convert::Infallible, Vec<f64>> {
        certify(
            self.modes.spectrum(),
            self.betti(),
            self.vertices.len(),
            self.dim(),
        )
    }

    /// D x, exactly: an assertion's question lands on the edges, an edge
    /// cochain's answer lands on the vertices.
    pub fn question(&self, x: &Cochain) -> Result<Cochain, Outside> {
        let x = self.dense(x)?;
        let y = self.dirac.apply(&x).expect("dense is D's dimension");
        Ok(self.sparse(&y))
    }

    /// e^{-tD²} x: settles onto ker D and keeps all of it.
    pub fn decay(&self, t: Time, x: &Cochain) -> Result<Cochain, Outside> {
        let x = self.dense(x)?;
        Ok(self.sparse(&self.apply(&x, |l| (-t.0 * l * l).exp())))
    }

    /// e^{i·angle·D} x: isometric, never settles.
    pub fn ring(&self, angle: Angle, x: &Cochain) -> Result<Rotated, Outside> {
        let x = self.dense(x)?;
        Ok(Rotated {
            real: self.sparse(&self.apply(&x, |l| (angle.0 * l).cos())),
            imag: self.sparse(&self.apply(&x, |l| (angle.0 * l).sin())),
        })
    }

    /// Attend: multiply by the solo, pitches by the indicator, intervals by
    /// the mean of their endpoints.
    pub fn solo(&self, solo: &Solo, x: &Cochain) -> Result<Cochain, Outside> {
        let w = self.weights(solo)?;
        let x = self.dense(x)?;
        Ok(self.sparse(&x.iter().zip(&w).map(|(a, b)| a * b).collect::<Vec<_>>()))
    }

    /// [D, solo]: exactly where attention and question fail to commute.
    pub fn bleed(&self, solo: &Solo) -> Result<Bleed, Outside> {
        let w = self.weights(solo)?;
        let n = self.dim();
        // [D, W]_ij = D_ij (w_j − w_i), one row per dimension of the room.
        let rows: Vec<Vec<f64>> = (0..n)
            .map(|i| (0..n).map(|j| self.d(i, j) * (w[j] - w[i])).collect())
            .collect();
        let edges = self
            .edges
            .iter()
            .filter(|(_, &k)| rows[k].iter().any(|&b| b != 0.0))
            .map(|(&e, _)| e)
            .collect();
        // Bleed's public matrix is row-major.
        let matrix = rows.into_iter().flatten().collect();
        Ok(Bleed { matrix, edges })
    }

    fn weights(&self, solo: &Solo) -> Result<Vec<f64>, Outside> {
        if let Some(&v) = solo.0.iter().find(|v| !self.vertices.contains_key(v)) {
            return Err(Outside::Vertex(v));
        }
        let mut w = vec![0.0_f64; self.dim()];
        for (v, &i) in &self.vertices {
            if solo.0.contains(v) {
                w[i] = 1.0;
            }
        }
        for (&(u, v), &k) in &self.edges {
            w[k] = (w[self.vertices[&u]] + w[self.vertices[&v]]) / 2.0;
        }
        Ok(w)
    }

    /// The Hodge split of intervals: `(gradient, harmonic)`.
    ///
    /// The harmonic part is the projection onto ker D. An interval-only input
    /// projects only into H¹ (H⁰ lives on pitches), so the result doesn't
    /// depend on which eigenbasis LAPACK chose. The kernel is counted exactly,
    /// b₀ + b₁ by union-find, not by a threshold. A pitch value is refused.
    pub fn hodge(&self, x: &Cochain) -> Result<Hodge, Outside> {
        if let Some(&v) = x.vertex.iter().find(|(_, x)| **x != 0.0).map(|(v, _)| v) {
            return Err(Outside::NotAnInterval(v));
        }
        let dense = self.dense(x)?;
        let mut by_size: Vec<(f64, &[f64])> = self.modes().collect();
        by_size.sort_by(|a, b| a.0.abs().total_cmp(&b.0.abs()));
        let mut harmonic = vec![0.0_f64; self.dim()];
        for &(_, v) in by_size.iter().take(self.betti()) {
            let weight: f64 = v.iter().zip(&dense).map(|(a, b)| a * b).sum();
            for (h, vc) in harmonic.iter_mut().zip(v) {
                *h += weight * vc;
            }
        }
        let n0 = self.vertices.len();
        for h in harmonic.iter_mut().take(n0) {
            *h = 0.0;
        }
        let gradient: Vec<f64> = dense.iter().zip(&harmonic).map(|(a, h)| a - h).collect();
        Ok(Hodge {
            gradient: self.sparse_intervals(&gradient),
            harmonic: self.sparse_intervals(&harmonic),
        })
    }

    /// Restrict `x` to the modes of D inside `window`.
    ///
    /// `kept` and `erased` are each built from their own eigenmodes, so
    /// `kept + erased = x` is a check, not an identity by construction. The
    /// projector is a function of D, so it commutes with `decay` and `ring`.
    /// If an eigenvalue lies within LAPACK's error (`16·ε·dim·scale`, the
    /// `settle` band) of a window edge, the restriction is refused.
    pub fn restrict(&self, window: &Window, x: &Cochain) -> Result<Restriction, Outside> {
        let dense = self.dense(x)?;
        let n = self.dim();
        let scale = self
            .modes
            .spectrum()
            .iter()
            .fold(1.0_f64, |m, v| m.max(v.abs()));
        let error = 16.0 * f64::EPSILON * n as f64 * scale;
        let mut kept = vec![0.0_f64; n];
        let mut erased = vec![0.0_f64; n];
        let mut lossless = true;
        for (l, v) in self.modes() {
            if (l - window.low).abs() <= error || (l - window.high).abs() <= error {
                return Err(Outside::Straddles(l));
            }
            let weight: f64 = v.iter().zip(&dense).map(|(a, b)| a * b).sum();
            let inside = window.low <= l && l <= window.high;
            if !inside && weight != 0.0 {
                lossless = false;
            }
            let target = if inside { &mut kept } else { &mut erased };
            for (t, vc) in target.iter_mut().zip(v) {
                *t += weight * vc;
            }
        }
        Ok(Restriction {
            kept: self.sparse(&kept),
            erased: self.sparse(&erased),
            lossless,
        })
    }

    /// b₀ + b₁, exactly: components by union-find, then |E| − |V| + b₀.
    fn betti(&self) -> usize {
        let mut parent: Vec<usize> = (0..self.vertices.len()).collect();
        fn root(p: &mut [usize], i: usize) -> usize {
            if p[i] == i {
                i
            } else {
                let r = root(p, p[i]);
                p[i] = r;
                r
            }
        }
        let mut b0 = self.vertices.len();
        for &(u, v) in self.edges.keys() {
            let (ru, rv) = (
                root(&mut parent, self.vertices[&u]),
                root(&mut parent, self.vertices[&v]),
            );
            if ru != rv {
                parent[ru] = rv;
                b0 -= 1;
            }
        }
        let b1 = self.edges.len() + b0 - self.vertices.len();
        b0 + b1
    }

    fn sparse_intervals(&self, y: &[f64]) -> Cochain {
        Cochain {
            vertex: BTreeMap::new(),
            edge: self.edges.iter().map(|(&e, &k)| (e, y[k])).collect(),
        }
    }

    /// f(D) x through the eigenbasis.
    fn apply(&self, x: &[f64], f: impl Fn(f64) -> f64) -> Vec<f64> {
        let mut y = vec![0.0_f64; self.dim()];
        for (l, v) in self.modes() {
            let weight = f(l) * v.iter().zip(x).map(|(a, b)| a * b).sum::<f64>();
            for (yc, vc) in y.iter_mut().zip(v) {
                *yc += weight * vc;
            }
        }
        y
    }

    fn dense(&self, x: &Cochain) -> Result<Vec<f64>, Outside> {
        let mut out = vec![0.0_f64; self.dim()];
        for (&v, &value) in &x.vertex {
            let i = *self.vertices.get(&v).ok_or(Outside::Vertex(v))?;
            out[i] = value;
        }
        for (&e, &value) in &x.edge {
            let k = *self.edges.get(&e).ok_or(Outside::Edge(e))?;
            out[k] = value;
        }
        Ok(out)
    }

    fn sparse(&self, y: &[f64]) -> Cochain {
        Cochain {
            vertex: self.vertices.iter().map(|(&v, &i)| (v, y[i])).collect(),
            edge: self.edges.iter().map(|(&e, &k)| (e, y[k])).collect(),
        }
    }
}

#[cfg(test)]
mod certify_tests {
    use super::certify;
    use terni::Imperfect;

    #[test]
    fn a_separated_spectrum_certifies_the_exact_count() {
        // Triangle: b0 + b1 = 2, D's spectrum is {0, 0, ±sqrt3, ±sqrt3}.
        let s = 3f64.sqrt();
        assert_eq!(
            certify(&[-s, -s, 0.0, 1e-17, s, s], 2, 3, 6),
            Imperfect::Success(2)
        );
    }

    #[test]
    fn a_kernel_mode_that_is_not_zero_is_partial_not_a_guess() {
        let s = 3f64.sqrt();
        match certify(&[-s, -s, 0.0, 0.3, s, s], 2, 3, 6) {
            Imperfect::Partial(2, ambiguous) => assert_eq!(ambiguous, vec![0.3]),
            other => panic!("expected Partial(2, [0.3]), got {other:?}"),
        }
    }

    #[test]
    fn a_mode_inside_the_proven_gap_is_partial() {
        // n = 3: every non-kernel |eigenvalue| of D is at least 1/sqrt(6).
        match certify(&[0.0, 0.0, 0.01, 2.0], 2, 3, 4) {
            Imperfect::Partial(2, ambiguous) => assert_eq!(ambiguous, vec![0.01]),
            other => panic!("expected Partial(2, [0.01]), got {other:?}"),
        }
    }

    // The fixtures below sit on the thresholds, not far from them. Scale and
    // dim are powers of two, so the error bound 16 · ε · scale · dim is exact
    // in any association order, and so is `gap - error`: a reading can sit
    // exactly on the bound, and one ulp past it.
    const SCALE: f64 = 2.0;
    const DIM: usize = 4;

    /// 16 · 2⁻⁵² · 2 · 4 = 2⁻⁴⁵.
    fn error() -> f64 {
        2f64.powi(-45)
    }

    /// One kernel mode `m` beside an exact zero, two modes far past any gap.
    fn kernel_reading(m: f64) -> Imperfect<usize, std::convert::Infallible, Vec<f64>> {
        certify(&[0.0, m, SCALE, SCALE], 2, 2, DIM)
    }

    #[test]
    fn a_kernel_mode_at_the_error_bound_is_zero_and_one_ulp_past_it_is_not() {
        let e = error();
        assert_eq!(e, 16.0 * f64::EPSILON * SCALE * DIM as f64);
        assert_eq!(kernel_reading(e / 2.0), Imperfect::Success(2));
        assert_eq!(kernel_reading(e), Imperfect::Success(2), "the bound is in");
        assert_eq!(
            kernel_reading(e.next_up()),
            Imperfect::Partial(2, vec![e.next_up()]),
            "one ulp past the bound is out"
        );
        assert_eq!(
            kernel_reading(2.0 * e),
            Imperfect::Partial(2, vec![2.0 * e])
        );
    }

    #[test]
    fn the_error_bound_scales_from_a_floor_of_one() {
        // Largest magnitude 0.5: the scale is max(0.5, 1) = 1, the bound 2⁻⁴⁶.
        // n = 100 puts the gap (≈ 0.01) below the 0.5 modes.
        let e = 2f64.powi(-46);
        let read = |m: f64| certify(&[0.0, m, 0.5, 0.5], 2, 100, DIM);
        assert_eq!(read(e), Imperfect::Success(2));
        assert_eq!(read(e.next_up()), Imperfect::Partial(2, vec![e.next_up()]));
    }

    /// The proven gap for n vertices, from n(n−1) given as a literal.
    const GAPS: [(usize, f64); 5] = [(2, 2.0), (3, 6.0), (4, 12.0), (5, 20.0), (10, 90.0)];

    #[test]
    fn the_gap_is_one_over_root_n_times_n_minus_one() {
        for (n, n_n_minus_1) in GAPS {
            let gap = 1.0 / f64::sqrt(n_n_minus_1);
            let read = |m: f64| certify(&[0.0, m, SCALE, SCALE], 1, n, DIM);
            assert_eq!(
                read(gap * 0.99),
                Imperfect::Partial(1, vec![gap * 0.99]),
                "n = {n}: 1% inside the gap is ambiguous"
            );
            assert_eq!(
                read(gap * 1.01),
                Imperfect::Success(1),
                "n = {n}: 1% past the gap clears it"
            );
        }
    }

    #[test]
    fn the_error_margin_widens_what_clears_the_gap() {
        // A mode within LAPACK's error below the gap may be a mode on the gap:
        // it clears. One ulp further in, it doesn't.
        let e = error();
        for (n, n_n_minus_1) in GAPS {
            let gap = 1.0 / f64::sqrt(n_n_minus_1);
            let read = |m: f64| certify(&[0.0, m, SCALE, SCALE], 1, n, DIM);
            let edge = gap - e;
            assert_eq!(gap - edge, e, "n = {n}: the margin is exact");
            assert_eq!(read(gap - e / 2.0), Imperfect::Success(1), "n = {n}");
            assert_eq!(read(edge), Imperfect::Success(1), "n = {n}: on the margin");
            assert_eq!(
                read(edge.next_down()),
                Imperfect::Partial(1, vec![edge.next_down()]),
                "n = {n}: one ulp inside the margin"
            );
            assert_eq!(
                read(gap - 2.0 * e),
                Imperfect::Partial(1, vec![gap - 2.0 * e]),
                "n = {n}"
            );
        }
    }

    /// The one rule, against what runs: wherever certify reads a kernel mode,
    /// its verdict is the declared `zeros::instrument` band's, and that band
    /// is `zeros::settle`'s.
    #[test]
    fn certify_reads_the_declared_instrument_band() {
        use crate::band::{zeros, Verdict};
        let band = zeros::instrument(DIM);
        assert_eq!(band, zeros::settle(DIM), "one rule since c7de979");
        let e = error();
        for m in [0.0, e / 2.0, e, e.next_up(), 2.0 * e, 1e-3] {
            assert_eq!(
                kernel_reading(m) == Imperfect::Success(2),
                band.read(m, SCALE) == Verdict::Zero,
                "certify and the declared band disagree at {m:e}"
            );
        }
    }
}
