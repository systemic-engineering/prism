//! `prismqueer::room` — the room in which a thing resonates.
//!
//! A [`Room`] is a space: the space a value lives in, and the space an
//! operator's resonances live in. On one graph there are two rooms, C⁰ (the
//! vertex room, [`Vertices`]) and C¹ (the edge room, [`Edges`]), and their
//! sum C⁰ ⊕ C¹ ([`Sum`]) where the Dirac operator D lives. Operators move
//! between rooms: δ ([`coboundary`]) is a door from C⁰ into C¹, and δ*
//! ([`Map::adjoint`]) is the door back.
//!
//! A self-adjoint map from a room to itself has a spectrum, and that
//! spectrum is the room's resonances: its [`Modes`], literally, the way a
//! physical room's acoustic modes are its Laplacian's eigenfunctions under
//! its boundary conditions.
//!
//! What `room` keeps from collapsing early:
//! - **The kind of a room is a type.** [`Map<A, B, R>`] goes from a room of
//!   kind `A` to one of kind `B`; it only composes ([`Map::then`]) with a map
//!   out of `B`. The dimension and the address ([`Oid`]) are runtime values
//!   the room carries; a mismatch there is a checked [`RoomError::Mismatch`],
//!   never a panic or a silent wrong answer.
//! - **Identity is per skeleton.** C⁰ is addressed by its vertex set alone,
//!   and C¹ sits over a C⁰ by relation ([`Room::skeleton`]), so the
//!   Laplacians of two graphs on one vertex set compose: they are maps of one
//!   room. [`coboundary_over`] takes δ out of a larger C⁰(V), for graphs on
//!   different vertices compared over their union.
//! - **Exact topology stays exact.** δ is built over [`Z`], integers that are
//!   an [`Exact`] group in `field`'s terms. They are arbitrary-precision, so
//!   exact maps compose exactly and never overflow. Moving to `f64` is one
//!   explicit step, [`Map::measure`]; entry types never mix implicitly.
//! - **Only resonance gets a spectrum.** Only a map from a room to itself
//!   can be checked [`Map::symmetric`], and only a [`Symmetric`] measured
//!   map can be [`decompose`](Symmetric::decompose)d. Its eigenvectors are
//!   read as columns ([`Modes::vector`]), never by raw index.
//! - **Layout is private.** Entries are stored column-major, the way LAPACK
//!   speaks, so a decomposition hands the matrix over without a copy and a
//!   mode comes back as a contiguous slice.
//!
//! Composition is checked by kind. The control compiles:
//!
//! ```
//! use prismqueer::room::coboundary;
//! use prismqueer::spectral::graph_from_edges;
//! let delta = coboundary(&graph_from_edges(&[(0, 1), (1, 2), (0, 2)]));
//! let laplacian = delta.then(&delta.adjoint()).unwrap(); // C⁰ → C¹ → C⁰
//! let _ = delta.measure().then(&delta.adjoint().measure());
//! let _ = laplacian.symmetric();
//! ```
//!
//! Each of the following changes one line of it, and doesn't compile
//! (rustdoc on stable doesn't check the error code, which is why the control
//! is here). δ then δ: the second door leads out of C⁰, but the first one
//! ended in C¹.
//!
//! ```compile_fail,E0308
//! use prismqueer::room::coboundary;
//! use prismqueer::spectral::graph_from_edges;
//! let delta = coboundary(&graph_from_edges(&[(0, 1), (1, 2), (0, 2)]));
//! let laplacian = delta.then(&delta).unwrap(); // C⁰ → C¹, then out of C⁰
//! let _ = delta.measure().then(&delta.adjoint().measure());
//! let _ = laplacian.symmetric();
//! ```
//!
//! An exact map then a measured one: the move to `f64` is never implicit.
//!
//! ```compile_fail,E0308
//! use prismqueer::room::coboundary;
//! use prismqueer::spectral::graph_from_edges;
//! let delta = coboundary(&graph_from_edges(&[(0, 1), (1, 2), (0, 2)]));
//! let laplacian = delta.then(&delta.adjoint()).unwrap(); // C⁰ → C¹ → C⁰
//! let _ = delta.then(&delta.adjoint().measure());
//! let _ = laplacian.symmetric();
//! ```
//!
//! A door isn't a room: δ goes from C⁰ to C¹, so it has no resonances.
//!
//! ```compile_fail,E0599
//! use prismqueer::room::coboundary;
//! use prismqueer::spectral::graph_from_edges;
//! let delta = coboundary(&graph_from_edges(&[(0, 1), (1, 2), (0, 2)]));
//! let laplacian = delta.then(&delta.adjoint()).unwrap(); // C⁰ → C¹ → C⁰
//! let _ = delta.measure().then(&delta.adjoint().measure());
//! let _ = delta.symmetric();
//! ```

use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Debug;
use std::marker::PhantomData;

use num_bigint::{BigInt, Sign};
use num_traits::{One, ToPrimitive, Zero};

use crate::field::{Exact, Group};
use crate::oid::{Oid, Scope};
use crate::spectral::kleinos::{complex_oid, Graph, VertexId};

// --- Kinds of room -----------------------------------------------------------

mod sealed {
    pub trait Sealed {}
}

/// The kind of a room: which space it is, as a type. Sealed: the kinds are
/// [`Vertices`], [`Edges`] and their [`Sum`]s.
pub trait Kind: sealed::Sealed + Clone + PartialEq + Debug + 'static {
    /// What a room of this kind carries besides its dimension and address:
    /// its basis for C⁰, nothing for C¹, its two summands for a sum.
    type Parts: Clone + PartialEq + Debug;
}

/// C⁰: one dimension per vertex, in ascending vertex id.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Vertices;

/// C¹: one dimension per edge, in canonical order and orientation (lower
/// vertex id first).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Edges;

/// A ⊕ B: `A`'s dimensions first, then `B`'s.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Sum<A, B>(PhantomData<(A, B)>);

impl sealed::Sealed for Vertices {}
impl sealed::Sealed for Edges {}
impl<A, B> sealed::Sealed for Sum<A, B> {}

impl Kind for Vertices {
    /// The basis: the vertex ids, ascending.
    type Parts = Vec<VertexId>;
}
impl Kind for Edges {
    type Parts = ();
}
impl<A: Kind, B: Kind> Kind for Sum<A, B> {
    type Parts = Box<(Room<A>, Room<B>)>;
}

// --- Rooms -------------------------------------------------------------------

/// A room: a space of kind `K`, with its dimension, its address and the
/// 0-skeleton it sits over. Two rooms are the same room when kind,
/// dimension, address, skeleton and parts (C⁰'s basis, a sum's summands)
/// agree.
///
/// Identity is per skeleton level, not the whole complex. C⁰ is addressed
/// by its vertex set alone, so two graphs on one vertex set share one C⁰.
/// C¹ is addressed by its complex and sits over a C⁰ by relation: its
/// [`skeleton`](Room::skeleton) is that C⁰'s address. Rewiring the edges
/// changes C¹ over a fixed C⁰, not the vertex space.
#[derive(Clone, Debug, PartialEq)]
pub struct Room<K: Kind> {
    /// C⁰: the 0-skeleton (V, ∅)'s address. C¹: the complex's. A sum: its
    /// summands', in order.
    complex: Oid,
    /// The address of the C⁰ this room sits over.
    skeleton: Oid,
    dim: usize,
    parts: K::Parts,
    kind: PhantomData<K>,
}

/// Where a room is, for an error report: its dimension and address.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Place {
    /// The room's dimension.
    pub dim: usize,
    /// The room's address ([`Room::complex`]).
    pub complex: Oid,
}

/// Why a room operation refused.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RoomError {
    /// Two rooms that had to be one aren't (dimension, address, skeleton or
    /// summands), or a graph doesn't fit in a vertex room.
    Mismatch {
        /// The room the operation was in.
        expected: Place,
        /// The room it was handed.
        found: Place,
    },
    /// A vector or row of the wrong length.
    Length {
        /// The length the room needs.
        expected: usize,
        /// The length given.
        found: usize,
    },
    /// The map isn't symmetric: entry `(row, col)` differs from `(col, row)`.
    Asymmetric {
        /// The first differing row (above the diagonal).
        row: usize,
        /// Its column.
        col: usize,
    },
    /// LAPACK couldn't measure (dsyev's `info`).
    Unmeasured(i32),
}

impl Room<Vertices> {
    /// C⁰(V): one dimension per vertex of `vertices`, ascending. Addressed
    /// by V alone: the address of the 0-skeleton (V, ∅).
    pub fn over(vertices: &BTreeSet<VertexId>) -> Self {
        let address = complex_oid(vertices.iter().copied(), std::iter::empty());
        Room {
            skeleton: address.clone(),
            complex: address,
            dim: vertices.len(),
            parts: vertices.iter().copied().collect(),
            kind: PhantomData,
        }
    }

    /// C⁰ of `graph`: the room over its vertex set.
    pub fn vertices(graph: &Graph) -> Self {
        Room::over(&graph.vertices().collect())
    }

    /// `Ok` when every vertex of `graph` is a dimension of this room.
    fn holds(&self, graph: &Graph) -> Result<(), RoomError> {
        if graph
            .vertices()
            .all(|v| self.parts.binary_search(&v).is_ok())
        {
            Ok(())
        } else {
            Err(RoomError::Mismatch {
                expected: self.place(),
                found: Room::vertices(graph).place(),
            })
        }
    }
}

impl Room<Edges> {
    /// C¹ of `graph`, over its own C⁰.
    pub fn edges(graph: &Graph) -> Self {
        Room::edges_over(&Room::vertices(graph), graph)
    }

    /// C¹ of `graph` over `skeleton`, whose vertices hold the graph's: the
    /// complex (V, E). Unchecked; [`Room::holds`] checks.
    fn edges_over(skeleton: &Room<Vertices>, graph: &Graph) -> Self {
        Room {
            complex: complex_oid(skeleton.parts.iter().copied(), graph.edges()),
            skeleton: skeleton.complex.clone(),
            dim: graph.edges().count(),
            parts: (),
            kind: PhantomData,
        }
    }
}

impl<K: Kind> Room<K> {
    /// The dimension.
    pub fn dim(&self) -> usize {
        self.dim
    }

    /// This room's content address: for C⁰ its vertex set's, for C¹ its
    /// complex's, for a sum its summands' in order.
    pub fn complex(&self) -> &Oid {
        &self.complex
    }

    /// The content address of the C⁰ this room sits over: C⁰'s own, C¹'s
    /// 0-skeleton, a sum's shared one.
    pub fn skeleton(&self) -> &Oid {
        &self.skeleton
    }

    fn place(&self) -> Place {
        Place {
            dim: self.dim,
            complex: self.complex.clone(),
        }
    }

    /// `Ok` when `other` is this room.
    fn admits(&self, other: &Self) -> Result<(), RoomError> {
        if self == other {
            Ok(())
        } else {
            Err(RoomError::Mismatch {
                expected: self.place(),
                found: other.place(),
            })
        }
    }
}

impl<A: Kind, B: Kind> Room<Sum<A, B>> {
    /// `left ⊕ right`. Refused unless both sit over one C⁰.
    pub fn sum(left: &Room<A>, right: &Room<B>) -> Result<Self, RoomError> {
        if left.skeleton != right.skeleton {
            return Err(RoomError::Mismatch {
                expected: left.place(),
                found: right.place(),
            });
        }
        let summands = format!("{}⊕{}", left.complex.as_str(), right.complex.as_str());
        Ok(Room {
            complex: Oid::hash(&Scope::public(), summands.as_bytes()),
            skeleton: left.skeleton.clone(),
            dim: left.dim + right.dim,
            parts: Box::new((left.clone(), right.clone())),
            kind: PhantomData,
        })
    }

    /// The first summand.
    pub fn left(&self) -> &Room<A> {
        &self.parts.0
    }

    /// The second summand.
    pub fn right(&self) -> &Room<B> {
        &self.parts.1
    }
}

// --- Entries -----------------------------------------------------------------

/// What a map's entries are: a commutative ring.
///
/// Exact entries ([`Z`]) are arbitrary-precision, so their arithmetic is
/// total; measured entries (`f64`) round.
pub trait Entry: Clone + PartialEq + Debug {
    /// The additive identity.
    fn zero() -> Self;
    /// The multiplicative identity.
    fn one() -> Self;
    /// `self + other`.
    fn add(&self, other: &Self) -> Self;
    /// `self · other`.
    fn mul(&self, other: &Self) -> Self;
    /// `−self`.
    fn neg(&self) -> Self;
}

/// Exact entries that can be measured: the one explicit step to `f64`.
pub trait Measure: Entry + Exact {
    /// This entry as a float.
    fn measure(&self) -> f64;
}

/// An integer, exactly and of any size: the entries of exact topology (δ is
/// ±1 and 0), and of everything composed from them.
///
/// In `field`'s terms an [`Exact`] group under addition: equality is exact,
/// so a non-zero entry really isn't zero. Arbitrary-precision, so no sum,
/// product or negation leaves its range.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Z(BigInt);

impl Z {
    /// The integer itself.
    pub fn value(&self) -> &BigInt {
        &self.0
    }
}

impl From<i64> for Z {
    fn from(n: i64) -> Self {
        Z(BigInt::from(n))
    }
}

impl From<BigInt> for Z {
    fn from(n: BigInt) -> Self {
        Z(n)
    }
}

impl Group for Z {
    fn identity() -> Self {
        Z::zero()
    }
    /// Addition.
    fn compose(&self, other: &Self) -> Self {
        self.add(other)
    }
    /// Negation.
    fn inverse(&self) -> Self {
        self.neg()
    }
}

impl Exact for Z {}

impl Entry for Z {
    fn zero() -> Self {
        Z(BigInt::from(0))
    }
    fn one() -> Self {
        Z(BigInt::from(1))
    }
    fn add(&self, other: &Self) -> Self {
        Z(&self.0 + &other.0)
    }
    fn mul(&self, other: &Self) -> Self {
        Z(&self.0 * &other.0)
    }
    fn neg(&self) -> Self {
        Z(-&self.0)
    }
}

impl Measure for Z {
    /// Exact up to 2⁵³ in magnitude; beyond, the nearest float, and ±∞ past
    /// `f64`'s range.
    fn measure(&self) -> f64 {
        self.0.to_f64().expect("every BigInt has a float reading")
    }
}

impl Entry for f64 {
    fn zero() -> Self {
        0.0
    }
    fn one() -> Self {
        1.0
    }
    fn add(&self, other: &Self) -> Self {
        self + other
    }
    fn mul(&self, other: &Self) -> Self {
        self * other
    }
    fn neg(&self) -> Self {
        -self
    }
}

// --- Maps --------------------------------------------------------------------

/// A linear map from room `A` to room `B` with entries `R`: a door.
///
/// As a matrix it has `to().dim()` rows and `from().dim()` columns. The
/// storage is column-major and private; read it through [`Map::entry`],
/// [`Map::apply`] or [`Modes`].
#[derive(Clone, Debug, PartialEq)]
pub struct Map<A: Kind, B: Kind, R> {
    from: Room<A>,
    to: Room<B>,
    /// Column-major: entry `(row, col)` is at `col * to.dim + row`.
    entries: Vec<R>,
}

impl<A: Kind, B: Kind, R: Entry> Map<A, B, R> {
    /// The map that sends everything to zero.
    pub fn zero(from: &Room<A>, to: &Room<B>) -> Self {
        Map {
            from: from.clone(),
            to: to.clone(),
            entries: vec![R::zero(); from.dim * to.dim],
        }
    }

    /// A map from its rows: `to.dim()` rows of `from.dim()` entries each.
    pub fn from_rows(from: &Room<A>, to: &Room<B>, rows: &[Vec<R>]) -> Result<Self, RoomError> {
        let length = |expected: usize, found: usize| {
            if expected == found {
                Ok(())
            } else {
                Err(RoomError::Length { expected, found })
            }
        };
        length(to.dim, rows.len())?;
        for row in rows {
            length(from.dim, row.len())?;
        }
        let entries = (0..from.dim)
            .flat_map(|col| rows.iter().map(move |row| row[col].clone()))
            .collect();
        Ok(Map {
            from: from.clone(),
            to: to.clone(),
            entries,
        })
    }

    /// The room this map leaves.
    pub fn from(&self) -> &Room<A> {
        &self.from
    }

    /// The room this map enters.
    pub fn to(&self) -> &Room<B> {
        &self.to
    }

    /// Entry `(row, col)`: how much of `from`'s dimension `col` lands in
    /// `to`'s dimension `row`. `None` outside the map.
    pub fn entry(&self, row: usize, col: usize) -> Option<&R> {
        (row < self.to.dim && col < self.from.dim).then(|| &self.entries[self.at(row, col)])
    }

    /// The map applied to a vector of `from`.
    pub fn apply(&self, x: &[R]) -> Result<Vec<R>, RoomError> {
        if x.len() != self.from.dim {
            return Err(RoomError::Length {
                expected: self.from.dim,
                found: x.len(),
            });
        }
        let mut y = vec![R::zero(); self.to.dim];
        for (column, xj) in self.columns().zip(x) {
            for (yi, aij) in y.iter_mut().zip(column) {
                *yi = yi.add(&aij.mul(xj));
            }
        }
        Ok(y)
    }

    /// First this map, then `next`: path order, as [`Group::compose`].
    /// `next` must leave the room this map enters: by kind at compile time,
    /// by dimension and complex here.
    pub fn then<C: Kind>(&self, next: &Map<B, C, R>) -> Result<Map<A, C, R>, RoomError> {
        self.to.admits(&next.from)?;
        let mut entries = Vec::with_capacity(self.from.dim * next.to.dim);
        for column in self.columns() {
            entries.extend(next.apply(column)?);
        }
        Ok(Map {
            from: self.from.clone(),
            to: next.to.clone(),
            entries,
        })
    }

    /// `self + other`, between the same two rooms.
    pub fn plus(&self, other: &Self) -> Result<Self, RoomError> {
        self.from.admits(&other.from)?;
        self.to.admits(&other.to)?;
        let entries = self
            .entries
            .iter()
            .zip(&other.entries)
            .map(|(a, b)| a.add(b))
            .collect();
        Ok(self.with(entries))
    }

    /// `−self`.
    pub fn negate(&self) -> Self {
        self.with(self.entries.iter().map(Entry::neg).collect())
    }

    /// The door back: the transpose, which for real entries is the adjoint
    /// under each room's standard inner product.
    pub fn adjoint(&self) -> Map<B, A, R> {
        let (m, n) = (self.to.dim, self.from.dim);
        // Column `c` of the adjoint is row `c` of this map.
        let entries = (0..m)
            .flat_map(|c| (0..n).map(move |r| self.entries[r * m + c].clone()))
            .collect();
        Map {
            from: self.to.clone(),
            to: self.from.clone(),
            entries,
        }
    }

    fn at(&self, row: usize, col: usize) -> usize {
        col * self.to.dim + row
    }

    /// Column `j` is where `from`'s dimension `j` goes.
    fn columns(&self) -> impl Iterator<Item = &[R]> {
        let m = self.to.dim;
        (0..self.from.dim).map(move |j| &self.entries[j * m..(j + 1) * m])
    }

    fn with<S>(&self, entries: Vec<S>) -> Map<A, B, S> {
        Map {
            from: self.from.clone(),
            to: self.to.clone(),
            entries,
        }
    }

    fn set(&mut self, row: usize, col: usize, value: R) {
        let i = self.at(row, col);
        self.entries[i] = value;
    }
}

impl<A: Kind, B: Kind, R: Measure> Map<A, B, R> {
    /// The one explicit step from exact to measured: every entry as `f64`.
    pub fn measure(&self) -> Map<A, B, f64> {
        self.with(self.entries.iter().map(Measure::measure).collect())
    }
}

impl<A: Kind, B: Kind> Map<A, B, f64> {
    /// The Frobenius norm: the square root of the sum of squared entries.
    pub(crate) fn frobenius(&self) -> f64 {
        self.entries.iter().map(|x| x * x).sum::<f64>().sqrt()
    }

    /// The singular values, descending, by LAPACK's dgesdd. The matrix goes
    /// over as stored (column-major). `Err` is dgesdd's `info`.
    pub(crate) fn singular_values(&self) -> Result<Vec<f64>, i32> {
        crate::ffi::singular_values_columns(self.to.dim, self.from.dim, &self.entries)
    }
}

impl<A: Kind, R: Entry> Map<A, A, R> {
    /// The map that changes nothing.
    pub fn identity(room: &Room<A>) -> Self {
        let mut map = Map::zero(room, room);
        for i in 0..room.dim {
            map.set(i, i, R::one());
        }
        map
    }

    /// This map as a [`Symmetric`] one, if it is: every entry equals its
    /// mirror, exactly. Only a map from a room to itself can be.
    pub fn symmetric(self) -> Result<Symmetric<A, R>, RoomError> {
        for col in 0..self.from.dim {
            for row in 0..col {
                if self.entries[self.at(row, col)] != self.entries[self.at(col, row)] {
                    return Err(RoomError::Asymmetric { row, col });
                }
            }
        }
        Ok(Symmetric(self))
    }
}

impl<A: Kind, B: Kind, R: Entry> Map<A, Sum<A, B>, R> {
    /// A into A ⊕ B.
    pub fn inject_left(room: &Room<Sum<A, B>>) -> Self {
        let mut map = Map::zero(room.left(), room);
        for i in 0..room.left().dim {
            map.set(i, i, R::one());
        }
        map
    }
}

impl<A: Kind, B: Kind, R: Entry> Map<B, Sum<A, B>, R> {
    /// B into A ⊕ B.
    pub fn inject_right(room: &Room<Sum<A, B>>) -> Self {
        let offset = room.left().dim;
        let mut map = Map::zero(room.right(), room);
        for i in 0..room.right().dim {
            map.set(offset + i, i, R::one());
        }
        map
    }
}

impl<A: Kind, B: Kind, R: Entry> Map<Sum<A, B>, A, R> {
    /// A ⊕ B onto A.
    pub fn project_left(room: &Room<Sum<A, B>>) -> Self {
        Map::inject_left(room).adjoint()
    }
}

impl<A: Kind, B: Kind, R: Entry> Map<Sum<A, B>, B, R> {
    /// A ⊕ B onto B.
    pub fn project_right(room: &Room<Sum<A, B>>) -> Self {
        Map::inject_right(room).adjoint()
    }
}

impl<A: Kind, B: Kind, C: Kind, D: Kind, R: Entry> Map<Sum<A, B>, Sum<C, D>, R> {
    /// `f ⊕ g`: `f` on the first summands, `g` on the second, nothing across.
    pub fn diagonal(f: &Map<A, C, R>, g: &Map<B, D, R>) -> Result<Self, RoomError> {
        let from = Room::sum(&f.from, &g.from)?;
        let to = Room::sum(&f.to, &g.to)?;
        let first = Map::project_left(&from)
            .then(f)?
            .then(&Map::inject_left(&to))?;
        let second = Map::project_right(&from)
            .then(g)?
            .then(&Map::inject_right(&to))?;
        first.plus(&second)
    }
}

impl<A: Kind, B: Kind, R: Entry> Map<Sum<A, B>, Sum<A, B>, R> {
    /// γ, the grading of A ⊕ B: +1 on `A`, −1 on `B`. A self-adjoint
    /// involution; D anticommutes with it on C⁰ ⊕ C¹, because D only moves
    /// between the summands.
    pub fn grading(room: &Room<Sum<A, B>>) -> Self {
        let mut map = Map::identity(room);
        for i in room.left().dim..room.dim {
            map.set(i, i, R::one().neg());
        }
        map
    }
}

// --- Resonance ---------------------------------------------------------------

/// A map from a room to itself that equals its own adjoint, checked: a
/// room's resonator. Only [`Map::symmetric`] makes one.
#[derive(Clone, Debug, PartialEq)]
pub struct Symmetric<A: Kind, R>(Map<A, A, R>);

impl<A: Kind, R: Measure> Symmetric<A, R> {
    /// Measured: every entry as `f64`. A mirror stays a mirror.
    pub fn measure(&self) -> Symmetric<A, f64> {
        Symmetric(self.0.measure())
    }
}

impl<A: Kind> Symmetric<A, f64> {
    /// The room's resonances: eigenvalues and eigenvectors, by LAPACK's dsyev.
    /// The matrix goes over as stored (column-major), without a copy.
    pub fn decompose(&self) -> Result<Modes<A>, RoomError> {
        let map = &self.0;
        let (spectrum, vectors) = crate::ffi::eigensystem_columns(map.from.dim, &map.entries)
            .map_err(RoomError::Unmeasured)?;
        Ok(Modes {
            room: map.from.clone(),
            spectrum,
            vectors,
        })
    }

    /// The room's resonant values alone, ascending: its spectrum without the
    /// modes, by dsyev('N'). Not [`decompose`](Self::decompose)'s spectrum to
    /// the bit: dsyev('V') solves the tridiagonal problem another way.
    pub fn spectrum(&self) -> Result<Vec<f64>, RoomError> {
        let map = &self.0;
        crate::ffi::eigenvalues_columns(map.from.dim, &map.entries).map_err(RoomError::Unmeasured)
    }
}

impl<A: Kind> Symmetric<A, Z> {
    /// The characteristic polynomial det(xI − M), exactly, by Berkowitz's
    /// algorithm: no division, so every step stays in Z.
    ///
    /// Grow M one leading principal block at a time. With Mᵣ₊₁ =
    /// [[Mᵣ, s], [r, a]], det(xI − Mᵣ₊₁) is a lower-triangular Toeplitz
    /// matrix with first column (1, −a, −rs, −rMᵣs, …, −rMᵣʳ⁻¹s) applied to
    /// det(xI − Mᵣ)'s coefficients. The powers Mᵣᵏs are matrix–vector
    /// products over M's non-zero entries only, so the cost is about
    /// n² · nnz(M) multiplications: n⁴/4 dense, far fewer for a graph.
    pub fn charpoly(&self) -> Charpoly {
        let m = &self.0;
        let n = m.from.dim;
        // Each column's non-zero entries, by row: Mᵣ is the rows below r.
        let columns: Vec<Vec<(usize, &BigInt)>> = (0..n)
            .map(|j| {
                (0..n)
                    .filter_map(|i| {
                        let x = &m.entries[m.at(i, j)].0;
                        (x.sign() != Sign::NoSign).then_some((i, x))
                    })
                    .collect()
            })
            .collect();
        let mut p = vec![BigInt::one()];
        for r in 0..n {
            // t = (1, −a, −rs, −rMᵣs, …): r + 2 entries.
            let mut t = Vec::with_capacity(r + 2);
            t.push(BigInt::one());
            t.push(-&m.entries[m.at(r, r)].0);
            let mut v: Vec<BigInt> = (0..r).map(|i| m.entries[m.at(i, r)].0.clone()).collect();
            for k in 0..r {
                let rv: BigInt = (0..r).map(|j| &m.entries[m.at(r, j)].0 * &v[j]).sum();
                t.push(-rv);
                if k + 1 < r {
                    let mut w = vec![BigInt::zero(); r];
                    for (j, vj) in v.iter().enumerate() {
                        if vj.sign() == Sign::NoSign {
                            continue;
                        }
                        for &(i, a) in columns[j].iter().take_while(|(i, _)| *i < r) {
                            w[i] += a * vj;
                        }
                    }
                    v = w;
                }
            }
            // p ← T·p: (r + 2) × (r + 1), T[i][j] = t[i − j].
            let mut q = vec![BigInt::zero(); r + 2];
            for (j, pj) in p.iter().enumerate() {
                for (i, ti) in t.iter().enumerate().take(r + 2 - j) {
                    q[i + j] += ti * pj;
                }
            }
            p = q;
        }
        Charpoly {
            coefficients: p.into_iter().map(Z).collect(),
        }
    }
}

/// A characteristic polynomial det(xI − M) of an n × n integer map: monic,
/// degree n, with integer coefficients.
///
/// Coefficients are stored **leading first**: `coefficients()[k]` is the
/// coefficient of xⁿ⁻ᵏ, so `coefficients()[0]` is 1 and the last is
/// det(−M) = (−1)ⁿ det M.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Charpoly {
    coefficients: Vec<Z>,
}

impl Charpoly {
    /// The coefficients, leading first: n + 1 of them, the first 1.
    pub fn coefficients(&self) -> &[Z] {
        &self.coefficients
    }

    /// The degree n: the dimension of the room.
    pub fn degree(&self) -> usize {
        self.coefficients.len() - 1
    }
}

/// The modes of a room: how a self-adjoint map on it resonates.
///
/// The spectrum is sorted ascending; mode `k` is eigenvalue `k` with
/// eigenvector `k`. Eigenvectors are read only whole, as columns.
#[derive(Clone, Debug, PartialEq)]
pub struct Modes<A: Kind> {
    room: Room<A>,
    spectrum: Vec<f64>,
    /// Column-major: eigenvector `k` is `vectors[k * n..(k + 1) * n]`.
    vectors: Vec<f64>,
}

impl<A: Kind> Modes<A> {
    /// The room that resonates.
    pub fn room(&self) -> &Room<A> {
        &self.room
    }

    /// The eigenvalues, ascending.
    pub fn spectrum(&self) -> &[f64] {
        &self.spectrum
    }

    /// Eigenvector `k`, belonging to `spectrum()[k]`: a vector of the room.
    pub fn vector(&self, k: usize) -> Option<&[f64]> {
        let n = self.room.dim;
        (k < n).then(|| &self.vectors[k * n..(k + 1) * n])
    }

    /// Every eigenvector, in spectrum order.
    pub fn vectors(&self) -> impl Iterator<Item = &[f64]> {
        (0..self.room.dim).filter_map(move |k| self.vector(k))
    }
}

// --- The doors of a graph ----------------------------------------------------

/// δ: C⁰ → C¹, exactly. `(δs)(u, v) = s(v) − s(u)` on each edge in
/// canonical orientation: the edge's row holds −1 at `u`, +1 at `v`.
///
/// A self-loop (v, v) has the zero row: s(v) − s(v) = 0. The two ends are
/// summed, not written over each other.
pub fn coboundary(graph: &Graph) -> Map<Vertices, Edges, Z> {
    delta(&Room::vertices(graph), graph)
}

/// δ of `graph` out of a given C⁰(V), V ⊇ the graph's vertices, into C¹ of
/// the complex (V, E) over it. A vertex of V outside the graph has a zero
/// column. Refused ([`RoomError::Mismatch`]) when the graph has a vertex
/// outside V.
pub fn coboundary_over(
    skeleton: &Room<Vertices>,
    graph: &Graph,
) -> Result<Map<Vertices, Edges, Z>, RoomError> {
    skeleton.holds(graph)?;
    Ok(delta(skeleton, graph))
}

/// δ out of `skeleton`, which holds the graph's vertices.
fn delta(skeleton: &Room<Vertices>, graph: &Graph) -> Map<Vertices, Edges, Z> {
    let index: BTreeMap<_, _> = skeleton
        .parts
        .iter()
        .enumerate()
        .map(|(i, &v)| (v, i))
        .collect();
    let mut delta: Map<Vertices, Edges, Z> =
        Map::zero(skeleton, &Room::edges_over(skeleton, graph));
    for (row, (u, v)) in graph.edges().enumerate() {
        for (w, sign) in [(u, Z::one().neg()), (v, Z::one())] {
            let at = delta.at(row, index[&w]);
            delta.entries[at] = delta.entries[at].add(&sign);
        }
    }
    delta
}

/// L₀ = δ*δ on C⁰ of `graph`, exactly: symmetric by construction.
pub fn laplacian(graph: &Graph) -> Symmetric<Vertices, Z> {
    let room = Room::vertices(graph);
    Symmetric(Map::zero(&room, &room))
}

/// D = δ + δ* on C⁰ ⊕ C¹, exactly: δ from the vertex summand into the edge
/// summand, δ* back. The same matrix, in the same basis, as
/// [`Instrument`](crate::spectral::instrument::Instrument)'s.
///
/// D² = L₀ ⊕ L₁↓: the graph Laplacian L₀ = δ*δ on C⁰ and the
/// down-Laplacian L₁↓ = δδ* on C¹. A graph is a 1-complex, with no 2-cells,
/// so the up part of the Hodge Laplacian on C¹ is absent and the lower block
/// is exactly L₁↓.
pub fn dirac(graph: &Graph) -> Map<Sum<Vertices, Edges>, Sum<Vertices, Edges>, Z> {
    let delta = coboundary(graph);
    let room = Room::sum(delta.from(), delta.to()).expect("δ's C¹ sits over its C⁰");
    // Every room below is built from `room`: the compositions can't mismatch.
    let down = Map::project_left(&room)
        .then(&delta)
        .and_then(|m| m.then(&Map::inject_right(&room)));
    let up = Map::project_right(&room)
        .then(&delta.adjoint())
        .and_then(|m| m.then(&Map::inject_left(&room)));
    down.and_then(|d| d.plus(&up?)).expect("one complex")
}
