//! `prismqueer::field` — single- and multi-truth fields (prism-005).
//!
//! A `settle` can end in one value or in many related values. Both are fields:
//! a scalar is a field on a [`Point`], a vector of criteria is a field on k
//! unconnected points ([`Discrete`]), and an intonation is a group-valued field
//! on a [`Graph`]: one transport per edge, a connection. The graph may have
//! parallel edges ([`Field::multigraph`]): two edges on one pair are two
//! transports, and the 2-loop between them holds `g·h⁻¹`. A loop is a hole;
//! filled by a face (a triangle, or a bigon on two parallel edges) it is
//! curvature on that face.
//!
//! Whether a field can be collapsed is decided by its value algebra and its
//! curvature (Mara, `ruler-and-pact` §10–§11, on mirror):
//! - **lattice-valued** ([`Lattice`]): a lawful collapse, the meet;
//! - **group-valued and flat**: collapses up to gauge into a potential
//!   ([`Flat::potential`]); the transports themselves are lost;
//! - **group-valued and curved**: no lawful collapse. [`Curved`] carries the
//!   holonomy of every independent loop; only an outside `temper` (mirror's)
//!   can flatten it, and that returns the curvature as erasure.
//!
//! The algebra is a type distinction: [`Lattice`] values get `meet`, [`Group`]
//! values get transports, and only [`Exact`] groups get [`check_flat`]. A float
//! group ([`Log2`]) is a measurement: it can be read only through a declared
//! [`Noise`] band ([`check_flat_within`]). No silent tolerance.
//!
//! Geometry only. Rulers, approvals and signatures live in mirror.
//!
//! A curved group field has no collapse: group values aren't a [`Lattice`],
//! so `meet` doesn't exist for them. The control compiles (same imports, a
//! group field as a type, `meet` on a lattice field):
//!
//! ```
//! use prismqueer::field::{Discrete, Field, Graph, Least, Ratio};
//! fn names_a_group_field(_: &Field<Graph, Ratio>) {}
//! fn collapse(f: &Field<Discrete, Least>) -> Least {
//!     f.meet()
//! }
//! ```
//!
//! and the only change, `meet` on group values, doesn't (rustdoc on stable
//! doesn't check the error code, which is why the control is here):
//!
//! ```compile_fail,E0599
//! use prismqueer::field::{Field, Graph, Ratio};
//! fn collapse(f: &Field<Graph, Ratio>) -> Ratio {
//!     f.meet()
//! }
//! ```

use std::collections::{BTreeMap, BTreeSet, VecDeque};
use std::fmt::Debug;

use crate::oid::{Oid, Scope};
use crate::spectral::kleinos::{EdgeKey, Graph as Shape, VertexId};
use crate::spectral::settle::Noise;

// --- The value algebra -------------------------------------------------------

/// A group: transports that compose and can be undone.
///
/// `a.compose(b)` is path order: first `a`, then `b`.
pub trait Group: Clone + PartialEq + Debug {
    /// The transport that changes nothing.
    fn identity() -> Self;
    /// First `self`, then `other`.
    fn compose(&self, other: &Self) -> Self;
    /// The transport back.
    fn inverse(&self) -> Self;
    /// Whether this is the identity, exactly.
    fn is_identity(&self) -> bool {
        *self == Self::identity()
    }
}

/// A group whose equality is exact, so a holonomy that isn't the identity
/// really isn't. Only these get [`check_flat`].
pub trait Exact: Group {}

/// A group read by measurement: how far a value is from the identity.
pub trait Measured: Group {
    /// Distance from the identity, in the group's own units.
    fn magnitude(&self) -> f64;
}

/// A meet-semilattice with a top: values that combine by taking the lower.
pub trait Lattice: Clone + PartialEq + Debug {
    /// The value that constrains nothing.
    fn top() -> Self;
    /// The greatest lower bound.
    fn meet(&self, other: &Self) -> Self;
}

/// An exact ratio as a vector of prime exponents: 3/2 is {2: −1, 3: 1}.
/// The group is multiplication of ratios, i.e. addition of exponents.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Ratio(BTreeMap<u64, i64>);

impl Ratio {
    /// The ratio `num / den`, factored exactly. Panics on zero.
    pub fn of(num: u64, den: u64) -> Self {
        assert!(num > 0 && den > 0, "a ratio of positive integers");
        let mut r = Self::default_identity();
        for (p, e) in factor(num) {
            r.add(p, e);
        }
        for (p, e) in factor(den) {
            r.add(p, -e);
        }
        r
    }

    /// A ratio from its prime exponents.
    pub fn from_primes(exponents: impl IntoIterator<Item = (u64, i64)>) -> Self {
        let mut r = Self::default_identity();
        for (p, e) in exponents {
            r.add(p, e);
        }
        r
    }

    /// The prime exponents (zero exponents never appear).
    pub fn exponents(&self) -> &BTreeMap<u64, i64> {
        &self.0
    }

    fn default_identity() -> Self {
        Ratio(BTreeMap::new())
    }

    fn add(&mut self, p: u64, e: i64) {
        let x = self.0.entry(p).or_insert(0);
        *x += e;
        if *x == 0 {
            self.0.remove(&p);
        }
    }
}

fn factor(mut n: u64) -> Vec<(u64, i64)> {
    let mut out = Vec::new();
    let mut p = 2;
    while p * p <= n {
        let mut e = 0;
        while n.is_multiple_of(p) {
            n /= p;
            e += 1;
        }
        if e > 0 {
            out.push((p, e));
        }
        p += 1;
    }
    if n > 1 {
        out.push((n, 1));
    }
    out
}

impl Group for Ratio {
    fn identity() -> Self {
        Self::default_identity()
    }
    fn compose(&self, other: &Self) -> Self {
        let mut r = self.clone();
        for (&p, &e) in &other.0 {
            r.add(p, e);
        }
        r
    }
    fn inverse(&self) -> Self {
        Ratio(self.0.iter().map(|(&p, &e)| (p, -e)).collect())
    }
}

impl Exact for Ratio {}

/// The integers mod `N` under addition (e.g. pitch classes, `Zn<12>`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Zn<const N: u64>(u64);

impl<const N: u64> Zn<N> {
    /// `x mod N`.
    pub fn new(x: u64) -> Self {
        Zn(x % N)
    }
    /// The residue.
    pub fn value(&self) -> u64 {
        self.0
    }
}

impl<const N: u64> Group for Zn<N> {
    fn identity() -> Self {
        Zn(0)
    }
    fn compose(&self, other: &Self) -> Self {
        Zn((self.0 + other.0) % N)
    }
    fn inverse(&self) -> Self {
        Zn((N - self.0) % N)
    }
}

impl<const N: u64> Exact for Zn<N> {}

/// A permutation: `image[i]` is where `i` goes. Points past the end are fixed,
/// and trailing fixed points are trimmed, so every permutation has one form.
/// Not commutative.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Perm(Vec<usize>);

impl Perm {
    /// A permutation from its images. Panics if `image` isn't a bijection.
    pub fn new(image: Vec<usize>) -> Self {
        let mut seen = vec![false; image.len()];
        for &i in &image {
            assert!(i < image.len() && !seen[i], "not a permutation: {image:?}");
            seen[i] = true;
        }
        Self::trimmed(image)
    }

    fn trimmed(mut image: Vec<usize>) -> Self {
        while image.last().is_some_and(|&x| x == image.len() - 1) {
            image.pop();
        }
        Perm(image)
    }

    fn at(&self, i: usize) -> usize {
        self.0.get(i).copied().unwrap_or(i)
    }
}

impl Group for Perm {
    fn identity() -> Self {
        Perm(Vec::new())
    }
    fn compose(&self, other: &Self) -> Self {
        let n = self.0.len().max(other.0.len());
        Self::trimmed((0..n).map(|i| other.at(self.at(i))).collect())
    }
    fn inverse(&self) -> Self {
        let mut inv = vec![0; self.0.len()];
        for (i, &j) in self.0.iter().enumerate() {
            inv[j] = i;
        }
        Self::trimmed(inv)
    }
}

impl Exact for Perm {}

/// A float group: logarithms (base 2) under addition. A measurement, not
/// [`Exact`]: read it through [`check_flat_within`] with a declared band.
#[derive(Clone, Copy, Debug, PartialEq, PartialOrd)]
pub struct Log2(pub f64);

impl Group for Log2 {
    fn identity() -> Self {
        Log2(0.0)
    }
    fn compose(&self, other: &Self) -> Self {
        Log2(self.0 + other.0)
    }
    fn inverse(&self) -> Self {
        Log2(-self.0)
    }
}

impl Measured for Log2 {
    fn magnitude(&self) -> f64 {
        self.0.abs()
    }
}

/// The least of some numbers: meet is `min`, top is `u64::MAX`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Least(pub u64);

impl Lattice for Least {
    fn top() -> Self {
        Least(u64::MAX)
    }
    fn meet(&self, other: &Self) -> Self {
        Least(self.0.min(other.0))
    }
}

impl Lattice for bool {
    fn top() -> Self {
        true
    }
    fn meet(&self, other: &Self) -> Self {
        *self && *other
    }
}

// --- Bases and fields --------------------------------------------------------

/// The base of a single truth: one point.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Point;

/// The base of k unconnected criteria.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Discrete;

/// A multigraph: prism's one graph type. The base of a connection (a value
/// on every edge), and the complex a [`room`](crate::room) is built on.
///
/// Two edges between the same pair are two edges ([`Edge::nth`] tells them
/// apart), each with its own transport and its own dimension of C¹. Cycle
/// space and b₁ count them both, so a doubled pair is one independent loop,
/// of length 2. A self-loop (v, v) is an edge. A simple graph
/// ([`spectral::Graph`](crate::spectral::Graph)) is a multigraph with every
/// multiplicity at most 1.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Graph {
    vertices: BTreeSet<VertexId>,
    /// Sorted: by ends, then by `nth`.
    edges: Vec<Edge>,
}

/// One edge of a graph base: its ends, and which of the parallel edges
/// between them it is (0 for the first given, then 1, ...). A simple graph's
/// edges are all `nth` 0.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Edge {
    ends: EdgeKey,
    nth: usize,
}

impl Edge {
    /// The `nth` edge between `a` and `b` (in either order).
    pub fn between(a: VertexId, b: VertexId, nth: usize) -> Self {
        Edge {
            ends: ordered(a, b),
            nth,
        }
    }
    /// Its ends, smaller first.
    pub fn ends(&self) -> EdgeKey {
        self.ends
    }
    /// Which of the parallel edges between its ends, counting from 0 in the
    /// order they were given.
    pub fn nth(&self) -> usize {
        self.nth
    }
}

fn ordered(a: VertexId, b: VertexId) -> EdgeKey {
    if a <= b {
        (a, b)
    } else {
        (b, a)
    }
}

/// The content address of the 1-complex with these vertices and edges, both
/// given ascending. Canonical serialization: vertices, a separator, edges,
/// each edge as its two ends, so `m` parallel edges are their ends `m`
/// times. Content-address per Rec #82 β-normal AST OID. With no edges it is
/// the address of a 0-skeleton, which is how [`room`](crate::room) addresses
/// a vertex room by its vertex set alone.
pub(crate) fn complex_oid(
    vertices: impl Iterator<Item = VertexId>,
    edges: impl Iterator<Item = EdgeKey>,
) -> Oid {
    let mut bytes = Vec::new();
    for v in vertices {
        bytes.extend_from_slice(&v.to_le_bytes());
    }
    bytes.push(0xFF); // separator
    for e in edges {
        bytes.extend_from_slice(&e.0.to_le_bytes());
        bytes.extend_from_slice(&e.1.to_le_bytes());
    }
    Oid::hash(&Scope::public(), &bytes)
}

/// A spanning forest of a graph: tree neighbours (with the edge's index) and
/// the chords, the edges that close a cycle.
type Forest = (BTreeMap<VertexId, Vec<(VertexId, usize)>>, Vec<usize>);

impl Graph {
    /// The multigraph on `vertices` and the edges' ends, with every given
    /// pair its own edge: a pair given twice is two parallel edges, numbered
    /// by [`Edge::nth`] in the order given. Ends in either order.
    pub fn new(
        vertices: impl IntoIterator<Item = VertexId>,
        edges: impl IntoIterator<Item = EdgeKey>,
    ) -> Self {
        let mut vertices: BTreeSet<VertexId> = vertices.into_iter().collect();
        let mut seen: BTreeMap<EdgeKey, usize> = BTreeMap::new();
        let mut numbered = Vec::new();
        for (a, b) in edges {
            let ends = ordered(a, b);
            let nth = seen.entry(ends).or_insert(0);
            numbered.push(Edge { ends, nth: *nth });
            *nth += 1;
            vertices.extend([a, b]);
        }
        numbered.sort();
        Graph {
            vertices,
            edges: numbered,
        }
    }

    /// The vertices, in order.
    pub fn vertices(&self) -> impl Iterator<Item = VertexId> + '_ {
        self.vertices.iter().copied()
    }

    /// How many vertices.
    pub fn vertex_count(&self) -> usize {
        self.vertices.len()
    }

    /// Whether `v` is a vertex.
    pub fn contains_vertex(&self, v: VertexId) -> bool {
        self.vertices.contains(&v)
    }

    /// The edges, by ends and then by `nth`.
    pub fn edges(&self) -> impl Iterator<Item = Edge> + '_ {
        self.edges.iter().copied()
    }

    /// How many edges join this pair (given in either order).
    pub fn multiplicity(&self, ends: EdgeKey) -> usize {
        let ends = ordered(ends.0, ends.1);
        self.edges.iter().filter(|e| e.ends == ends).count()
    }

    /// b₁, the number of independent loops: edges − vertices + components.
    /// Every parallel edge counts.
    pub fn cycle_rank(&self) -> usize {
        self.spanning().1.len()
    }

    /// The spanning forest, walked: breadth-first order and every chord's
    /// canonical cycle.
    pub(crate) fn walked(&self) -> Spanning {
        let (tree, chords) = self.spanning();
        let mut order = Vec::with_capacity(self.vertices.len());
        let mut back: BTreeMap<VertexId, (VertexId, usize)> = BTreeMap::new();
        let mut seen: BTreeSet<VertexId> = BTreeSet::new();
        for start in self.vertices() {
            if !seen.insert(start) {
                continue;
            }
            order.push((start, None));
            let mut queue = VecDeque::from([start]);
            while let Some(u) = queue.pop_front() {
                for &(w, i) in &tree[&u] {
                    if seen.insert(w) {
                        order.push((w, Some((u, i))));
                        back.insert(w, (u, i));
                        queue.push_back(w);
                    }
                }
            }
        }
        let cycles = chords
            .into_iter()
            .map(|chord| canonical_loop(fundamental_cycle(&back, self.edges[chord].ends, chord)))
            .collect();
        Spanning { order, cycles }
    }

    fn index(&self, edge: Edge) -> Option<usize> {
        self.edges.binary_search(&edge).ok()
    }

    /// Spanning forest by union-find, edges in order.
    fn spanning(&self) -> Forest {
        let mut parent: BTreeMap<VertexId, VertexId> =
            self.vertices.iter().map(|&v| (v, v)).collect();
        fn root(p: &mut BTreeMap<VertexId, VertexId>, v: VertexId) -> VertexId {
            let up = p[&v];
            if up == v {
                v
            } else {
                let r = root(p, up);
                p.insert(v, r);
                r
            }
        }
        let mut tree: BTreeMap<VertexId, Vec<(VertexId, usize)>> =
            self.vertices.iter().map(|&v| (v, Vec::new())).collect();
        let mut chords = Vec::new();
        for (i, e) in self.edges.iter().enumerate() {
            let (u, v) = e.ends;
            let (ru, rv) = (root(&mut parent, u), root(&mut parent, v));
            if ru == rv {
                chords.push(i);
            } else {
                parent.insert(ru, rv);
                tree.get_mut(&u).unwrap().push((v, i));
                tree.get_mut(&v).unwrap().push((u, i));
            }
        }
        (tree, chords)
    }
}

/// A closed walk: `vertices[k]` to `vertices[k + 1]` along the edge with
/// index `edges[k]`.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct Walk {
    pub(crate) vertices: Vec<VertexId>,
    pub(crate) edges: Vec<usize>,
}

/// A spanning forest, walked: the part of an exact solve of δg = c on edges
/// that doesn't depend on what the values are. [`check_flat`] folds group
/// transports over it; [`room::solve`](crate::room::solve) folds ring
/// coefficients over it. One forest, so the two read the same potential
/// and the same loops on one graph.
pub(crate) struct Spanning {
    /// Every vertex once, breadth-first from each component's smallest
    /// vertex: a root has no parent; every other vertex comes after its
    /// tree parent, with the index of the tree edge between them.
    pub(crate) order: Vec<(VertexId, Option<(VertexId, usize)>)>,
    /// The cycle each chord closes, canonical ([`canonical_loop`]), chords
    /// in edge order.
    pub(crate) cycles: Vec<Walk>,
}

/// A field: values over a base.
#[derive(Clone, Debug, PartialEq)]
pub struct Field<B, V> {
    base: B,
    faces: Vec<Walk>,
    values: Vec<V>,
}

/// Why a graph field couldn't be built.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FieldError {
    /// This edge has no transport.
    Missing(EdgeKey),
    /// This edge isn't in the graph.
    Outside(EdgeKey),
    /// A face names this pair by its ends, but it has parallel edges: which
    /// one is meant? Name the edges ([`Field::with_bigons`]) instead.
    Parallel(EdgeKey),
    /// A bigon's two edges don't join the same pair.
    Apart(EdgeKey, EdgeKey),
}

impl<V> Field<Point, V> {
    /// A single truth.
    pub fn point(value: V) -> Self {
        Field {
            base: Point,
            faces: Vec::new(),
            values: vec![value],
        }
    }

    /// The one value of a single truth.
    pub fn value(&self) -> &V {
        &self.values[0]
    }
}

impl<V> Field<Discrete, V> {
    /// k unconnected criteria.
    pub fn discrete(values: Vec<V>) -> Self {
        Field {
            base: Discrete,
            faces: Vec::new(),
            values,
        }
    }
}

impl<B, V: Lattice> Field<B, V> {
    /// The lawful collapse of a lattice-valued field: the meet of its values
    /// (top when empty).
    pub fn meet(&self) -> V {
        self.values.iter().fold(V::top(), |acc, v| acc.meet(v))
    }
}

impl<V> Field<Graph, V> {
    /// The graph base.
    pub fn graph(&self) -> &Graph {
        &self.base
    }
}

impl<V: Group> Field<Graph, V> {
    /// A connection on a simple graph: one transport per edge, given in
    /// either orientation (a transport given on `(v, u)` is stored inverted
    /// on `(u, v)`). Every edge is `nth` 0; a pair given twice keeps the
    /// last. For parallel edges, use [`Field::multigraph`].
    pub fn transports(
        graph: &Shape,
        given: impl IntoIterator<Item = (EdgeKey, V)>,
    ) -> Result<Self, FieldError> {
        let mut by_edge: BTreeMap<EdgeKey, V> = BTreeMap::new();
        for ((a, b), t) in given {
            let (key, t) = if a <= b {
                ((a, b), t)
            } else {
                ((b, a), t.inverse())
            };
            if !graph.contains_edge(key) {
                return Err(FieldError::Outside(key));
            }
            by_edge.insert(key, t);
        }
        let base = graph.multigraph().clone();
        let mut values = Vec::with_capacity(base.edges.len());
        for e in &base.edges {
            values.push(by_edge.remove(&e.ends).ok_or(FieldError::Missing(e.ends))?);
        }
        Ok(Field {
            base,
            faces: Vec::new(),
            values,
        })
    }

    /// A connection on a multigraph: every given transport is its own edge,
    /// so a pair given twice is two parallel edges, numbered by `nth` in the
    /// order given. Either orientation, as in [`Field::transports`]. The
    /// vertices are the edges' ends.
    pub fn multigraph(given: impl IntoIterator<Item = (EdgeKey, V)>) -> Self {
        let mut vertices = BTreeSet::new();
        let mut seen: BTreeMap<EdgeKey, usize> = BTreeMap::new();
        let mut by_edge: BTreeMap<Edge, V> = BTreeMap::new();
        for ((a, b), t) in given {
            let t = if a <= b { t } else { t.inverse() };
            let ends = ordered(a, b);
            let nth = seen.entry(ends).or_insert(0);
            by_edge.insert(Edge { ends, nth: *nth }, t);
            *nth += 1;
            vertices.extend([a, b]);
        }
        let (edges, values) = by_edge.into_iter().unzip();
        Field {
            base: Graph { vertices, edges },
            faces: Vec::new(),
            values,
        }
    }

    /// Declare filled triangles (2-cells), named by their corners. A loop
    /// that bounds faces is no longer a hole; a face that doesn't close is
    /// curvature on that face. Refused if a side isn't in the graph, or has
    /// parallel edges (name those with [`Field::with_bigons`]).
    pub fn with_faces(
        mut self,
        faces: impl IntoIterator<Item = [VertexId; 3]>,
    ) -> Result<Self, FieldError> {
        for [a, b, c] in faces {
            let mut edges = Vec::with_capacity(3);
            for (x, y) in [(a, b), (b, c), (c, a)] {
                let ends = ordered(x, y);
                match self.base.multiplicity(ends) {
                    0 => return Err(FieldError::Missing(ends)),
                    1 => {}
                    _ => return Err(FieldError::Parallel(ends)),
                }
                edges.extend(self.base.index(Edge { ends, nth: 0 }));
            }
            self.faces.push(Walk {
                vertices: vec![a, b, c, a],
                edges,
            });
        }
        Ok(self)
    }

    /// Declare filled bigons: 2-cells bounded by two parallel edges. A 2-loop
    /// that bounds one is no longer a hole; a bigon that doesn't close is
    /// curvature on that face. Refused if an edge isn't in the graph, or the
    /// two don't join the same pair.
    pub fn with_bigons(
        mut self,
        bigons: impl IntoIterator<Item = [Edge; 2]>,
    ) -> Result<Self, FieldError> {
        for [e, f] in bigons {
            let i = self.base.index(e).ok_or(FieldError::Missing(e.ends))?;
            let j = self.base.index(f).ok_or(FieldError::Missing(f.ends))?;
            if e.ends != f.ends {
                return Err(FieldError::Apart(e.ends, f.ends));
            }
            let (u, v) = e.ends;
            self.faces.push(Walk {
                vertices: vec![u, v, u],
                edges: vec![i, j],
            });
        }
        Ok(self)
    }

    /// The transport along edge `i`, leaving from its end `from`.
    fn step(&self, i: usize, from: VertexId) -> V {
        if self.base.edges[i].ends.0 == from {
            self.values[i].clone()
        } else {
            self.values[i].inverse()
        }
    }
}

// --- Prisms that settle into fields ------------------------------------------

/// A prism whose `settle` ends in a field: multi-truth over `B`.
///
/// Nothing to implement. Every [`Prism`](crate::Prism) whose refracted beam
/// carries a `Field<B, V>` is one, by what it settles into.
pub trait FieldPrism<B, V>: crate::Prism<Refracted: crate::Beam<Out = Field<B, V>>> {}

impl<P, B, V> FieldPrism<B, V> for P where P: crate::Prism<Refracted: crate::Beam<Out = Field<B, V>>>
{}

/// A field prism over a point: a single truth.
pub trait ScalarPrism<V>: FieldPrism<Point, V> {}

impl<P, V> ScalarPrism<V> for P where P: FieldPrism<Point, V> {}

// --- Flat or curved ----------------------------------------------------------

/// A flat connection, which only [`check_flat`] or [`check_flat_within`] can
/// produce. Its gauge class is absorbable: a potential `p` with
/// `transport(u → v) = p(u)⁻¹ · p(v)` on every edge, the identity at each
/// component's smallest vertex.
#[derive(Clone, Debug, PartialEq)]
pub struct Flat<V> {
    potential: BTreeMap<VertexId, V>,
}

impl<V> Flat<V> {
    /// The potential: the field collapsed up to gauge.
    pub fn potential(&self) -> &BTreeMap<VertexId, V> {
        &self.potential
    }
}

/// One independent loop and the transport around it.
#[derive(Clone, Debug, PartialEq)]
pub struct Loop<V> {
    vertices: Vec<VertexId>,
    edges: Vec<Edge>,
    holonomy: V,
}

impl<V> Loop<V> {
    /// The loop, starting and ending at its smallest vertex, heading first
    /// toward the smaller of that vertex's two neighbours on the loop. On a
    /// 2-loop both neighbours are the same vertex: it heads out along the
    /// smaller edge and comes back along the other.
    pub fn vertices(&self) -> &[VertexId] {
        &self.vertices
    }
    /// The edges around the loop, in path order: `edges()[k]` joins
    /// `vertices()[k]` and `vertices()[k + 1]`. This is what tells two
    /// parallel edges apart.
    pub fn edges(&self) -> &[Edge] {
        &self.edges
    }
    /// The transport around the loop, composed in path order.
    pub fn holonomy(&self) -> &V {
        &self.holonomy
    }
}

/// A curved connection.
///
/// - `faces`: declared 2-cells whose boundary doesn't close. That is
///   curvature on that face, a local failure on one triple overlap (the
///   navigator's cocked hat).
/// - `loops`: cycle-basis loops that don't close and don't bound declared
///   faces. That is holonomy around a hole, which belongs to no observer.
///   A basis loop that goes around a face *and* a hole isn't in the faces'
///   span, so it's reported here, and its holonomy then includes that face's
///   curvature. Read it together with `faces`.
///
/// The gauge class is these holonomies (up to conjugacy).
#[derive(Clone, Debug, PartialEq)]
pub struct Curved<V> {
    loops: Vec<Loop<V>>,
    faces: Vec<Loop<V>>,
}

impl<V> Curved<V> {
    /// Holes: loops that don't close and bound no faces.
    pub fn loops(&self) -> &[Loop<V>] {
        &self.loops
    }
    /// Faces that don't close: curvature, located on the face.
    pub fn faces(&self) -> &[Loop<V>] {
        &self.faces
    }
}

/// Flat or curved: the witness of whether a connection can collapse.
#[derive(Clone, Debug, PartialEq)]
pub enum Flatness<V> {
    /// Every loop closes; a potential exists.
    Flat(Flat<V>),
    /// Some loops don't close; no lawful collapse.
    Curved(Curved<V>),
}

/// Flat or curved, decided exactly.
pub fn check_flat<V: Exact>(field: &Field<Graph, V>) -> Flatness<V> {
    flatness(field, |h| h.is_identity())
}

/// Flat or curved, read through a declared band: a loop closes when its
/// holonomy is within `noise` of the identity.
pub fn check_flat_within<V: Measured>(field: &Field<Graph, V>, noise: Noise) -> Flatness<V> {
    flatness(field, |h| h.magnitude() <= noise.0)
}

fn flatness<V: Group>(field: &Field<Graph, V>, closes: impl Fn(&V) -> bool) -> Flatness<V> {
    let graph = &field.base;
    let Spanning { order, cycles } = graph.walked();

    // Potential: breadth-first from each component's smallest vertex.
    let mut potential: BTreeMap<VertexId, V> = BTreeMap::new();
    for (w, parent) in order {
        let p = match parent {
            None => V::identity(),
            Some((u, i)) => potential[&u].compose(&field.step(i, u)),
        };
        potential.insert(w, p);
    }

    let around = |walk: Walk| {
        let holonomy = walk
            .vertices
            .iter()
            .zip(&walk.edges)
            .fold(V::identity(), |h, (&from, &i)| {
                h.compose(&field.step(i, from))
            });
        Loop {
            edges: walk.edges.iter().map(|&i| graph.edges[i]).collect(),
            vertices: walk.vertices,
            holonomy,
        }
    };
    // Over GF(2): an edge walked twice cancels.
    let edge_set = |walk: &Walk| -> BTreeSet<usize> {
        let mut set = BTreeSet::new();
        for &i in &walk.edges {
            if !set.remove(&i) {
                set.insert(i);
            }
        }
        set
    };

    // Face boundaries span the cycles that aren't holes (over GF(2): exact
    // unless H₁ of the complex has torsion).
    let mut span = Gf2::default();
    let mut faces = Vec::new();
    for face in &field.faces {
        let walk = canonical_loop(face.clone());
        span.insert(edge_set(&walk));
        let face = around(walk);
        if !closes(&face.holonomy) {
            faces.push(face);
        }
    }

    let mut loops = Vec::new();
    for walk in cycles {
        if span.contains(edge_set(&walk)) {
            continue;
        }
        let hole = around(walk);
        if !closes(&hole.holonomy) {
            loops.push(hole);
        }
    }
    if loops.is_empty() && faces.is_empty() {
        Flatness::Flat(Flat { potential })
    } else {
        Flatness::Curved(Curved { loops, faces })
    }
}

/// A subspace of edge sets over GF(2), kept in echelon form by pivot.
#[derive(Default)]
struct Gf2 {
    rows: BTreeMap<usize, BTreeSet<usize>>,
}

impl Gf2 {
    fn reduce(&self, mut v: BTreeSet<usize>) -> BTreeSet<usize> {
        while let Some(&p) = v.iter().next_back() {
            match self.rows.get(&p) {
                Some(row) => v = v.symmetric_difference(row).copied().collect(),
                None => break,
            }
        }
        v
    }

    fn insert(&mut self, v: BTreeSet<usize>) {
        let r = self.reduce(v);
        if let Some(&p) = r.iter().next_back() {
            self.rows.insert(p, r);
        }
    }

    fn contains(&self, v: BTreeSet<usize>) -> bool {
        self.reduce(v).is_empty()
    }
}

/// The cycle closed by chord `(u, v)` (edge index `chord`): the tree path
/// from `u` to `v`, then the chord back to `u`.
fn fundamental_cycle(
    back: &BTreeMap<VertexId, (VertexId, usize)>,
    (u, v): EdgeKey,
    chord: usize,
) -> Walk {
    let ancestors = |mut x: VertexId| {
        let mut path = vec![x];
        while let Some(&(p, _)) = back.get(&x) {
            path.push(p);
            x = p;
        }
        path
    };
    let (up, vp) = (ancestors(u), ancestors(v));
    let on_v: BTreeSet<VertexId> = vp.iter().copied().collect();
    let meet = *up
        .iter()
        .find(|x| on_v.contains(x))
        .expect("same component");
    let mut vertices = Vec::new();
    let mut edges = Vec::new();
    for &x in up.iter().take_while(|&&x| x != meet) {
        vertices.push(x);
        edges.push(back[&x].1);
    }
    vertices.push(meet);
    let down: Vec<VertexId> = vp.iter().copied().take_while(|&x| x != meet).collect();
    for &x in down.iter().rev() {
        edges.push(back[&x].1);
        vertices.push(x);
    }
    edges.push(chord);
    vertices.push(u);
    Walk { vertices, edges }
}

/// Rotate a closed walk to start at its smallest vertex and orient it toward
/// that vertex's smaller neighbour; when both neighbours are the same vertex
/// (a 2-loop), out along the smaller edge.
fn canonical_loop(walk: Walk) -> Walk {
    let n = walk.edges.len();
    let ring = &walk.vertices[..n];
    let start = (0..n).min_by_key(|&i| ring[i]).expect("non-empty loop");
    let ahead = (ring[(start + 1) % n], walk.edges[start]);
    let behind = (ring[(start + n - 1) % n], walk.edges[(start + n - 1) % n]);
    let (vertices, edges) = if ahead <= behind {
        (
            (0..=n).map(|k| ring[(start + k) % n]).collect(),
            (0..n).map(|k| walk.edges[(start + k) % n]).collect(),
        )
    } else {
        (
            (0..=n).map(|k| ring[(start + n - k) % n]).collect(),
            (0..n)
                .map(|k| walk.edges[(start + n - k - 1) % n])
                .collect(),
        )
    };
    Walk { vertices, edges }
}
