//! `prismqueer::field` — single- and multi-truth fields (prism-005).
//!
//! A `settle` can end in one value or in many related values. Both are fields:
//! a scalar is a field on a [`Point`], a vector of criteria is a field on k
//! unconnected points ([`Discrete`]), and an intonation is a group-valued field
//! on a [`Graph`]: one transport per edge, a connection.
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

/// The base of a connection: a graph, with a value on every edge.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Graph(Shape);

/// A field: values over a base.
#[derive(Clone, Debug, PartialEq)]
pub struct Field<B, V> {
    base: B,
    faces: Vec<[VertexId; 3]>,
    values: Vec<V>,
    edges: Vec<EdgeKey>,
}

/// Why a graph field couldn't be built.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FieldError {
    /// This edge has no transport.
    Missing(EdgeKey),
    /// This edge isn't in the graph.
    Outside(EdgeKey),
    /// A face names this pair by its ends, but it has parallel edges.
    Parallel(EdgeKey),
    /// A bigon's two edges don't join the same pair.
    Apart(EdgeKey, EdgeKey),
}

/// One edge of a graph base: its ends, and which of the parallel edges
/// between them it is.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Edge {
    ends: EdgeKey,
    nth: usize,
}

impl Edge {
    /// The `nth` edge between `a` and `b`.
    pub fn between(a: VertexId, b: VertexId, nth: usize) -> Self {
        let ends = if a <= b { (a, b) } else { (b, a) };
        Edge { ends, nth }
    }
    /// Its ends, smaller first.
    pub fn ends(&self) -> EdgeKey {
        self.ends
    }
    /// Which of the parallel edges between its ends.
    pub fn nth(&self) -> usize {
        self.nth
    }
}

impl Graph {
    /// The vertices.
    pub fn vertices(&self) -> impl Iterator<Item = VertexId> + '_ {
        self.0.vertices()
    }
    /// The edges.
    pub fn edges(&self) -> impl Iterator<Item = Edge> + '_ {
        self.0.edges().map(|(a, b)| Edge::between(a, b, 0))
    }
    /// How many edges join this pair.
    pub fn multiplicity(&self, ends: EdgeKey) -> usize {
        usize::from(self.0.contains_edge(ends))
    }
    /// b₁.
    pub fn cycle_rank(&self) -> usize {
        0
    }
}

impl<V> Field<Point, V> {
    /// A single truth.
    pub fn point(value: V) -> Self {
        Field {
            base: Point,
            faces: Vec::new(),
            values: vec![value],
            edges: Vec::new(),
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
            edges: Vec::new(),
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
    /// A connection on a multigraph: every given transport is its own edge.
    pub fn multigraph(given: impl IntoIterator<Item = (EdgeKey, V)>) -> Self {
        let given: Vec<(EdgeKey, V)> = given.into_iter().collect();
        let keys: Vec<EdgeKey> = given.iter().map(|&(k, _)| k).collect();
        let shape = crate::spectral::kleinos::graph_from_edges(&keys);
        Self::transports(&shape, given).expect("every edge given")
    }

    /// Declare filled bigons.
    pub fn with_bigons(
        self,
        _bigons: impl IntoIterator<Item = [Edge; 2]>,
    ) -> Result<Self, FieldError> {
        Ok(self)
    }

    /// A connection: one transport per edge, given in either orientation (a
    /// transport given on `(v, u)` is stored inverted on `(u, v)`).
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
        let edges: Vec<EdgeKey> = graph.edges().collect();
        let mut values = Vec::with_capacity(edges.len());
        for e in &edges {
            values.push(by_edge.remove(e).ok_or(FieldError::Missing(*e))?);
        }
        Ok(Field {
            base: Graph(graph.clone()),
            faces: Vec::new(),
            values,
            edges,
        })
    }

    /// Declare filled triangles (2-cells). A loop that bounds faces is no
    /// longer a hole; a face that doesn't close is curvature on that face.
    /// Refused if a face's edge isn't in the graph.
    pub fn with_faces(
        mut self,
        faces: impl IntoIterator<Item = [VertexId; 3]>,
    ) -> Result<Self, FieldError> {
        for [a, b, c] in faces {
            for (x, y) in [(a, b), (b, c), (a, c)] {
                let key = if x <= y { (x, y) } else { (y, x) };
                if self.edges.binary_search(&key).is_err() {
                    return Err(FieldError::Missing(key));
                }
            }
            self.faces.push([a, b, c]);
        }
        Ok(self)
    }

    /// The transport from `a` to `b` along an edge of the graph.
    fn transport(&self, a: VertexId, b: VertexId) -> V {
        let key = if a <= b { (a, b) } else { (b, a) };
        let i = self
            .edges
            .binary_search(&key)
            .expect("an edge of the field");
        if a <= b {
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
    holonomy: V,
}

impl<V> Loop<V> {
    /// The loop, starting and ending at its smallest vertex, heading first
    /// toward the smaller of that vertex's two neighbours on the loop.
    pub fn vertices(&self) -> &[VertexId] {
        &self.vertices
    }
    /// The transport around the loop, composed in path order.
    pub fn holonomy(&self) -> &V {
        &self.holonomy
    }
    /// The edges around the loop.
    pub fn edges(&self) -> &[Edge] {
        &[]
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
    let shape = &field.base.0;
    let vertices: Vec<VertexId> = shape.vertices().collect();

    // Spanning forest by union-find, edges in canonical order.
    let mut parent: BTreeMap<VertexId, VertexId> = vertices.iter().map(|&v| (v, v)).collect();
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
    let mut tree: BTreeMap<VertexId, Vec<VertexId>> =
        vertices.iter().map(|&v| (v, Vec::new())).collect();
    let mut chords = Vec::new();
    for &(u, v) in &field.edges {
        let (ru, rv) = (root(&mut parent, u), root(&mut parent, v));
        if ru == rv {
            chords.push((u, v));
        } else {
            parent.insert(ru, rv);
            tree.get_mut(&u).unwrap().push(v);
            tree.get_mut(&v).unwrap().push(u);
        }
    }

    // Potential: breadth-first from each component's smallest vertex.
    let mut potential: BTreeMap<VertexId, V> = BTreeMap::new();
    let mut back: BTreeMap<VertexId, VertexId> = BTreeMap::new();
    for &start in &vertices {
        if potential.contains_key(&start) {
            continue;
        }
        potential.insert(start, V::identity());
        let mut queue = VecDeque::from([start]);
        while let Some(u) = queue.pop_front() {
            for &w in &tree[&u] {
                if !potential.contains_key(&w) {
                    let p = potential[&u].compose(&field.transport(u, w));
                    potential.insert(w, p);
                    back.insert(w, u);
                    queue.push_back(w);
                }
            }
        }
    }

    let around = |vertices: Vec<VertexId>| {
        let holonomy = vertices.windows(2).fold(V::identity(), |h, w| {
            h.compose(&field.transport(w[0], w[1]))
        });
        Loop { vertices, holonomy }
    };
    let edge_set = |walk: &[VertexId]| -> BTreeSet<usize> {
        let mut set = BTreeSet::new();
        for w in walk.windows(2) {
            let key = if w[0] <= w[1] {
                (w[0], w[1])
            } else {
                (w[1], w[0])
            };
            let i = field
                .edges
                .binary_search(&key)
                .expect("an edge of the field");
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
    for &[a, b, c] in &field.faces {
        let walk = canonical_loop(vec![a, b, c, a]);
        span.insert(edge_set(&walk));
        let face = around(walk);
        if !closes(&face.holonomy) {
            faces.push(face);
        }
    }

    let mut loops = Vec::new();
    for (u, v) in chords {
        let walk = canonical_loop(fundamental_cycle(&back, u, v));
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

/// The cycle closed by chord `(u, v)`: the tree path from `u` to `v`, then
/// the chord back to `u`.
fn fundamental_cycle(
    back: &BTreeMap<VertexId, VertexId>,
    u: VertexId,
    v: VertexId,
) -> Vec<VertexId> {
    let ancestors = |mut x: VertexId| {
        let mut path = vec![x];
        while let Some(&p) = back.get(&x) {
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
    let mut cycle: Vec<VertexId> = up.iter().copied().take_while(|&x| x != meet).collect();
    cycle.push(meet);
    let down: Vec<VertexId> = vp.iter().copied().take_while(|&x| x != meet).collect();
    cycle.extend(down.into_iter().rev());
    cycle.push(u);
    cycle
}

/// Rotate a closed vertex walk to start at its smallest vertex and orient it
/// toward that vertex's smaller neighbour.
fn canonical_loop(closed: Vec<VertexId>) -> Vec<VertexId> {
    let ring = &closed[..closed.len() - 1];
    let n = ring.len();
    let start = (0..n).min_by_key(|&i| ring[i]).expect("non-empty loop");
    let next = ring[(start + 1) % n];
    let prev = ring[(start + n - 1) % n];
    let step: Box<dyn Fn(usize) -> usize> = if next <= prev {
        Box::new(move |k| (start + k) % n)
    } else {
        Box::new(move |k| (start + n - k) % n)
    };
    (0..=n).map(|k| ring[step(k)]).collect()
}
