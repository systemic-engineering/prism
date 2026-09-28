//! `prismqueer::spectral::instrument` — the Hodge–Dirac operator of a graph.
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

use std::collections::BTreeMap;

use crate::spectral::commutator::Unmeasured;
use crate::spectral::kleinos::{EdgeKey, Graph, VertexId};
use crate::spectral::settle::Time;

/// How far to rotate: the parameter of e^{i·angle·D}.
#[derive(Copy, Clone, Debug, PartialEq, PartialOrd)]
pub struct Angle(pub f64);

/// A cochain named a vertex or edge the complex does not have.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum Outside {
    /// No such vertex.
    Vertex(VertexId),
    /// No such edge, in either orientation.
    Edge(EdgeKey),
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

/// e^{i·angle·D} applied to a real cochain: its real and imaginary parts.
#[derive(Clone, Debug, PartialEq)]
pub struct Rotated {
    /// cos(angle·D) x.
    pub real: Cochain,
    /// sin(angle·D) x.
    pub imag: Cochain,
}

/// Below this fraction of the spectral scale an eigenvalue of D is zero.
///
/// D's smallest nonzero eigenvalue is √λ₂ of the Laplacian, so near zero D
/// is better separated than L: a connected unweighted graph has
/// √λ₂ ≥ 1/√(n(n−1)).
const ZERO: f64 = 1e-9;

/// The Hodge–Dirac operator of a graph, with its eigensystem.
pub struct Instrument {
    vertices: BTreeMap<VertexId, usize>,
    edges: BTreeMap<EdgeKey, usize>,
    matrix: Vec<f64>,
    values: Vec<f64>,
    /// Eigenvectors as columns: component c of eigenvector k is
    /// `vectors[c * n + k]` (per `ffi::eigensystem`).
    vectors: Vec<f64>,
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
        let n = n0 + edges.len();
        let mut matrix = vec![0.0_f64; n * n];
        for (&(u, v), &k) in &edges {
            // (δs)(u, v) = s(v) − s(u); D holds δ below and δ* above.
            for (w, sign) in [(u, -1.0), (v, 1.0)] {
                let i = vertices[&w];
                matrix[k * n + i] = sign;
                matrix[i * n + k] = sign;
            }
        }
        let (values, vectors) = crate::ffi::eigensystem(n, &matrix).map_err(Unmeasured)?;
        Ok(Self {
            vertices,
            edges,
            matrix,
            values,
            vectors,
        })
    }

    fn dim(&self) -> usize {
        self.vertices.len() + self.edges.len()
    }

    /// D, row-major, vertices first, then edges in canonical order.
    pub fn matrix(&self) -> &[f64] {
        &self.matrix
    }

    /// The spectrum of D.
    pub fn eigenvalues(&self) -> &[f64] {
        &self.values
    }

    /// dim ker D = b₀ + b₁: components plus independent cycles.
    pub fn kernel_dimension(&self) -> usize {
        let scale = self.values.iter().fold(1.0_f64, |m, x| m.max(x.abs()));
        self.values.iter().filter(|x| x.abs() <= ZERO * scale).count()
    }

    /// D x, exactly: an assertion's question lands on the edges, an edge
    /// cochain's answer lands on the vertices.
    pub fn question(&self, x: &Cochain) -> Result<Cochain, Outside> {
        let x = self.dense(x)?;
        let n = self.dim();
        let y: Vec<f64> = (0..n)
            .map(|i| (0..n).map(|j| self.matrix[i * n + j] * x[j]).sum())
            .collect();
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

    /// f(D) x through the eigenbasis.
    fn apply(&self, x: &[f64], f: impl Fn(f64) -> f64) -> Vec<f64> {
        let n = self.dim();
        let mut y = vec![0.0_f64; n];
        for (k, &l) in self.values.iter().enumerate() {
            let v: Vec<f64> = (0..n).map(|c| self.vectors[c * n + k]).collect();
            let weight = f(l) * v.iter().zip(x).map(|(a, b)| a * b).sum::<f64>();
            for (yc, vc) in y.iter_mut().zip(&v) {
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
