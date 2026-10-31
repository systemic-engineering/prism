//! `prismqueer::room` — the room in which a thing resonates.
//!
//! A [`Room`] is a space: the space a value lives in, and the space an
//! operator's resonances live in. On one graph (a [`Graph`] multigraph, the
//! one graph type `field` is built on too) there are two rooms, C⁰ (the
//! vertex room, [`Vertices`]) and C¹ (the edge room, [`Edges`]), and their
//! sum C⁰ ⊕ C¹ ([`Sum`]) where the Dirac operator D lives. Filled with
//! faces ([`Face`]), a third: C² (the face room, [`Faces`]). Operators move
//! between rooms: δ ([`coboundary`]) is a door from C⁰ into C¹, δ₁
//! ([`coboundary1`]) one from C¹ into C², and each adjoint
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
//! let delta = coboundary(graph_from_edges(&[(0, 1), (1, 2), (0, 2)]).multigraph());
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
//! let delta = coboundary(graph_from_edges(&[(0, 1), (1, 2), (0, 2)]).multigraph());
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
//! let delta = coboundary(graph_from_edges(&[(0, 1), (1, 2), (0, 2)]).multigraph());
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
//! let delta = coboundary(graph_from_edges(&[(0, 1), (1, 2), (0, 2)]).multigraph());
//! let laplacian = delta.then(&delta.adjoint()).unwrap(); // C⁰ → C¹ → C⁰
//! let _ = delta.measure().then(&delta.adjoint().measure());
//! let _ = delta.symmetric();
//! ```
//!
//! # Faces
//!
//! δ₁ leads out of C¹, so it follows δ and never itself. The control
//! compiles:
//!
//! ```
//! use prismqueer::field::{Face, Graph};
//! use prismqueer::room::{coboundary, coboundary1};
//! let g = Graph::new([], [(0, 1), (1, 2), (0, 2)]);
//! let d0 = coboundary(&g);
//! let d1 = coboundary1(&g, &[Face::triangle(&g, [0, 1, 2]).unwrap()]).unwrap();
//! let _ = d0.then(&d1); // C⁰ → C¹ → C²
//! let _ = d1.adjoint().then(&d1); // C² → C¹ → C²
//! ```
//!
//! δ₁ then δ₁: the second door leads out of C¹, but the first one ended
//! in C².
//!
//! ```compile_fail,E0308
//! use prismqueer::field::{Face, Graph};
//! use prismqueer::room::{coboundary, coboundary1};
//! let g = Graph::new([], [(0, 1), (1, 2), (0, 2)]);
//! let d0 = coboundary(&g);
//! let d1 = coboundary1(&g, &[Face::triangle(&g, [0, 1, 2]).unwrap()]).unwrap();
//! let _ = d1.then(&d1); // C¹ → C², then out of C¹
//! let _ = d1.adjoint().then(&d1); // C² → C¹ → C²
//! ```
//!
//! δ₁ then δ: δ leads out of C⁰, into the wrong room.
//!
//! ```compile_fail,E0308
//! use prismqueer::field::{Face, Graph};
//! use prismqueer::room::{coboundary, coboundary1};
//! let g = Graph::new([], [(0, 1), (1, 2), (0, 2)]);
//! let d0 = coboundary(&g);
//! let d1 = coboundary1(&g, &[Face::triangle(&g, [0, 1, 2]).unwrap()]).unwrap();
//! let _ = d1.then(&d0); // C¹ → C², then out of C⁰
//! let _ = d1.adjoint().then(&d1); // C² → C¹ → C²
//! ```
//!
//! # Rings
//!
//! A room's coefficients live in one number system, and every computation
//! about them runs there (prism-011). The **kind** of ring is a type: [`Z`],
//! [`ZMod`] (ℤ/n) and [`QModOne`] (ℚ/ℤ, exact U(1)). The **modulus** of
//! ℤ/n is a runtime value ([`Modulus`]), carried by every residue and every
//! map and checked like dimension and address: two moduli in one operation
//! is a [`RoomError::Modulus`]. Between rings only named maps cross:
//! [`Z::reduce`], [`Map::reduce`], [`clear_denominators`],
//! [`ZMod::fraction`].
//!
//! The control compiles:
//!
//! ```
//! use prismqueer::field::Graph;
//! use prismqueer::room::{coboundary, coboundary_in, solve, Modulus, ZMod, Z};
//! let g = Graph::new([], [(0, 1), (0, 1)]);
//! let twelve = Modulus::new(12).unwrap();
//! let delta = coboundary(&g);
//! let delta12 = coboundary_in::<ZMod>(&twelve, &g);
//! let _ = delta.reduce(&twelve).then(&delta12.adjoint());
//! let c: Vec<ZMod> = [Z::from(6), Z::from(-6)].iter().map(|x| x.reduce(&twelve)).collect();
//! let _ = solve(&twelve, &g, &c);
//! ```
//!
//! Each of the following changes one line of it, and doesn't compile. A map
//! over ℤ then one over ℤ/12, with no reduction between:
//!
//! ```compile_fail,E0308
//! use prismqueer::field::Graph;
//! use prismqueer::room::{coboundary, coboundary_in, solve, Modulus, ZMod, Z};
//! let g = Graph::new([], [(0, 1), (0, 1)]);
//! let twelve = Modulus::new(12).unwrap();
//! let delta = coboundary(&g);
//! let delta12 = coboundary_in::<ZMod>(&twelve, &g);
//! let _ = delta.then(&delta12.adjoint());
//! let c: Vec<ZMod> = [Z::from(6), Z::from(-6)].iter().map(|x| x.reduce(&twelve)).collect();
//! let _ = solve(&twelve, &g, &c);
//! ```
//!
//! Integers solved in the ℤ/12 room: an integer isn't a residue until it is
//! reduced (ℤ's ring part is `()`, not a [`Modulus`]).
//!
//! ```compile_fail,E0271
//! use prismqueer::field::Graph;
//! use prismqueer::room::{coboundary, coboundary_in, solve, Modulus, ZMod, Z};
//! let g = Graph::new([], [(0, 1), (0, 1)]);
//! let twelve = Modulus::new(12).unwrap();
//! let delta = coboundary(&g);
//! let delta12 = coboundary_in::<ZMod>(&twelve, &g);
//! let _ = delta.reduce(&twelve).then(&delta12.adjoint());
//! let c: Vec<ZMod> = [Z::from(6), Z::from(-6)].iter().map(|x| x.reduce(&twelve)).collect();
//! let _ = solve(&twelve, &g, &[Z::from(6), Z::from(-6)]);
//! ```
//!
//! δ with entries in ℚ mod 1: ℚ/ℤ is a group, not a ring (1/3 ≡ 4/3, but
//! (1/3)² = 1/9 and (4/3)² = 16/9 ≢ 1/9), so it has no matrices. δ over ℤ
//! acts on its cochains instead ([`Map::act`]).
//!
//! ```compile_fail,E0277
//! use prismqueer::field::Graph;
//! use prismqueer::room::{coboundary, coboundary_in, solve, Modulus, QModOne, ZMod, Z};
//! let g = Graph::new([], [(0, 1), (0, 1)]);
//! let twelve = Modulus::new(12).unwrap();
//! let delta = coboundary(&g);
//! let delta12 = coboundary_in::<QModOne>(&(), &g);
//! let _ = delta.reduce(&twelve).then(&delta12.adjoint());
//! let c: Vec<ZMod> = [Z::from(6), Z::from(-6)].iter().map(|x| x.reduce(&twelve)).collect();
//! let _ = solve(&twelve, &g, &c);
//! ```
//!
//! An exact solve over floats: `f64` isn't [`Exactly`], since a nonzero
//! float pairing may be rounding. Floats are read through a band.
//!
//! ```compile_fail,E0277
//! use prismqueer::field::Graph;
//! use prismqueer::room::{coboundary, coboundary_in, solve, Modulus, ZMod, Z};
//! let g = Graph::new([], [(0, 1), (0, 1)]);
//! let twelve = Modulus::new(12).unwrap();
//! let delta = coboundary(&g);
//! let delta12 = coboundary_in::<ZMod>(&twelve, &g);
//! let _ = delta.reduce(&twelve).then(&delta12.adjoint());
//! let c: Vec<ZMod> = [Z::from(6), Z::from(-6)].iter().map(|x| x.reduce(&twelve)).collect();
//! let _ = solve(&(), &g, &[6.0, -6.0]);
//! ```

use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Debug;
use std::marker::PhantomData;

use num_bigint::{BigInt, Sign};
use num_traits::{One, ToPrimitive, Zero};

use crate::field::{complex_oid, Edge, Exact, Face, Graph, Group, Spanning, Walk};
use crate::oid::{Oid, Scope};
use crate::spectral::kleinos::VertexId;

// --- Kinds of room -----------------------------------------------------------

mod sealed {
    pub trait Sealed {}
}

/// The kind of a room: which space it is, as a type. Sealed: the kinds are
/// [`Vertices`], [`Edges`], [`Faces`] and their [`Sum`]s.
pub trait Kind: sealed::Sealed + Clone + PartialEq + Debug + 'static {
    /// What a room of this kind carries besides its dimension and address:
    /// its basis for C⁰, nothing for C¹, its two summands for a sum.
    type Parts: Clone + PartialEq + Debug;
}

/// C⁰: one dimension per vertex, in ascending vertex id.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Vertices;

/// C¹: one dimension per edge, in canonical order and orientation (lower
/// vertex id first). Parallel edges are distinct dimensions, by
/// [`Edge::nth`](crate::field::Edge::nth).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Edges;

/// C²: one dimension per face, in the order the faces are given, each
/// oriented canonically ([`Face`]).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Faces;

/// A ⊕ B: `A`'s dimensions first, then `B`'s.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Sum<A, B>(PhantomData<(A, B)>);

impl sealed::Sealed for Vertices {}
impl sealed::Sealed for Edges {}
impl sealed::Sealed for Faces {}
impl<A, B> sealed::Sealed for Sum<A, B> {}

impl Kind for Vertices {
    /// The basis: the vertex ids, ascending.
    type Parts = Vec<VertexId>;
}
impl Kind for Edges {
    type Parts = ();
}
impl Kind for Faces {
    /// The address of the C¹ it sits over.
    type Parts = Oid;
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
/// changes C¹ over a fixed C⁰, not the vertex space. C² sits over a C¹
/// the same way ([`Room::one_skeleton`]): refilling the faces changes C²
/// over a fixed C¹, and its `skeleton` is still the C⁰ under both.
#[derive(Clone, Debug, PartialEq)]
pub struct Room<K: Kind> {
    /// C⁰: the 0-skeleton (V, ∅)'s address. C¹: the complex's. C²: the
    /// 2-complex's. A sum: its summands', in order.
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
    /// A face runs along an edge the graph doesn't have.
    Outside(Edge),
    /// Two moduli in one operation: ℤ/n and ℤ/m are different rings, and
    /// a residue of one isn't a residue of the other.
    Modulus {
        /// The ring the operation was in.
        expected: Modulus,
        /// The ring it was handed.
        found: Modulus,
    },
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
            complex: complex_oid(
                skeleton.parts.iter().copied(),
                graph.edges().map(|e| e.ends()),
            ),
            skeleton: skeleton.complex.clone(),
            dim: graph.edges().count(),
            parts: (),
            kind: PhantomData,
        }
    }
}

impl Room<Faces> {
    /// C² of `graph` filled by `faces`, over C¹ of `graph`. Refused
    /// ([`RoomError::Outside`]) when a face runs along an edge the graph
    /// doesn't have.
    pub fn faces(graph: &Graph, faces: &[Face]) -> Result<Self, RoomError> {
        boundaries(graph, faces)?;
        Ok(Room::faces_over(&Room::edges(graph), faces))
    }

    /// C² over `edges`. Addressed by the C¹'s address and every face, as
    /// its walk: each step's start and edge (ends and `nth`).
    fn faces_over(edges: &Room<Edges>, faces: &[Face]) -> Self {
        let mut bytes = edges.complex.as_str().as_bytes().to_vec();
        for face in faces {
            bytes.push(0xFF); // separator
            for (v, e) in face.vertices().iter().zip(face.edges()) {
                let (a, b) = e.ends();
                for x in [*v, a, b, e.nth() as u64] {
                    bytes.extend_from_slice(&x.to_le_bytes());
                }
            }
        }
        Room {
            complex: Oid::hash(&Scope::public(), &bytes),
            skeleton: edges.skeleton.clone(),
            dim: faces.len(),
            parts: edges.complex.clone(),
            kind: PhantomData,
        }
    }

    /// The address of the C¹ this room sits over: its 1-skeleton.
    pub fn one_skeleton(&self) -> &Oid {
        &self.parts
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

// --- Rings -------------------------------------------------------------------

/// The runtime part of a ring: what its kind, as a type, doesn't say. `()`
/// for [`Z`], [`QModOne`] and `f64`; the [`Modulus`] for [`ZMod`].
pub trait RingPart: Clone + PartialEq + Debug {
    /// `Ok` when `found` is this ring, a [`RoomError::Modulus`] otherwise.
    fn admits(&self, found: &Self) -> Result<(), RoomError>;
}

impl RingPart for () {
    fn admits(&self, _: &()) -> Result<(), RoomError> {
        Ok(())
    }
}

/// The modulus n of ℤ/n: an integer n ≥ 2, of any size.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Modulus(BigInt);

impl Modulus {
    /// n, if n ≥ 2. ℤ/1 is the zero ring (0 = 1), and ℤ/0 is ℤ.
    pub fn new(n: impl Into<BigInt>) -> Option<Self> {
        let n = n.into();
        (n > BigInt::one()).then_some(Modulus(n))
    }

    /// n itself.
    pub fn value(&self) -> &BigInt {
        &self.0
    }

    /// x mod n, in 0..n.
    fn residue(&self, x: BigInt) -> BigInt {
        let r = x % &self.0;
        if r.sign() == Sign::Minus {
            r + &self.0
        } else {
            r
        }
    }
}

impl RingPart for Modulus {
    fn admits(&self, found: &Self) -> Result<(), RoomError> {
        if self == found {
            Ok(())
        } else {
            Err(RoomError::Modulus {
                expected: self.clone(),
                found: found.clone(),
            })
        }
    }
}

/// What a cochain's values are: an abelian group, which ℤ acts on by
/// repeated addition (a ℤ-module). The values of a twist.
///
/// Every ring is one ([`Entry`]); [`QModOne`] is one without being a ring.
/// The arithmetic assumes one ring: the room's operations check it
/// ([`RingPart::admits`]) before they compute, and [`ZMod`]'s arithmetic
/// panics, never answers, on two moduli.
pub trait Coefficient: Clone + PartialEq + Debug {
    /// The ring's runtime part.
    type Ring: RingPart;
    /// The ring this value lives in.
    fn ring(&self) -> &Self::Ring;
    /// The additive identity of `ring`.
    fn zero(ring: &Self::Ring) -> Self;
    /// `self + other`.
    fn add(&self, other: &Self) -> Self;
    /// `−self`.
    fn neg(&self) -> Self;
    /// `n · self`: `self` added `n` times (subtracted, for n < 0).
    fn times(&self, n: &BigInt) -> Self;
}

/// What a map's entries are: a commutative ring, with its one.
///
/// Exact entries ([`Z`], [`ZMod`]) are arbitrary-precision, so their
/// arithmetic is total; measured entries (`f64`) round.
pub trait Entry: Coefficient {
    /// The multiplicative identity of `ring`.
    fn one(ring: &Self::Ring) -> Self;
    /// `self · other`.
    fn mul(&self, other: &Self) -> Self;
}

/// Coefficients whose equality is exact, so a pairing that isn't zero
/// really isn't: [`Z`], [`ZMod`], [`QModOne`]. Only these get [`solve`].
/// `f64` isn't one.
pub trait Exactly: Coefficient {}

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

    /// The ring map ℤ → ℤ/n: this integer mod n.
    pub fn reduce(&self, n: &Modulus) -> ZMod {
        ZMod::new(self.0.clone(), n)
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
        Z(BigInt::zero())
    }
    /// Addition.
    fn compose(&self, other: &Self) -> Self {
        Coefficient::add(self, other)
    }
    /// Negation.
    fn inverse(&self) -> Self {
        Coefficient::neg(self)
    }
}

impl Exact for Z {}

impl Coefficient for Z {
    type Ring = ();
    fn ring(&self) -> &() {
        &()
    }
    fn zero(_: &()) -> Self {
        Z(BigInt::zero())
    }
    fn add(&self, other: &Self) -> Self {
        Z(&self.0 + &other.0)
    }
    fn neg(&self) -> Self {
        Z(-&self.0)
    }
    fn times(&self, n: &BigInt) -> Self {
        Z(n * &self.0)
    }
}

impl Entry for Z {
    fn one(_: &()) -> Self {
        Z(BigInt::one())
    }
    fn mul(&self, other: &Self) -> Self {
        Z(&self.0 * &other.0)
    }
}

impl Exactly for Z {}

impl Measure for Z {
    /// Exact up to 2⁵³ in magnitude; beyond, the nearest float, and ±∞ past
    /// `f64`'s range.
    fn measure(&self) -> f64 {
        self.0.to_f64().expect("every BigInt has a float reading")
    }
}

/// An integer mod n, exactly: a residue in 0..n, with its [`Modulus`].
///
/// The ring ℤ/n, n a runtime value. Pitch classes are ℤ/12; a `not` is a
/// half-turn in ℤ/2. Every residue carries its modulus, so a residue from
/// another ℤ/n is refused ([`RoomError::Modulus`]), never read as one of
/// this ring's.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct ZMod {
    residue: BigInt,
    modulus: Modulus,
}

impl ZMod {
    /// x mod n.
    pub fn new(x: impl Into<BigInt>, n: &Modulus) -> Self {
        ZMod {
            residue: n.residue(x.into()),
            modulus: n.clone(),
        }
    }

    /// The residue, in 0..n.
    pub fn residue(&self) -> &BigInt {
        &self.residue
    }

    /// n.
    pub fn modulus(&self) -> &Modulus {
        &self.modulus
    }

    /// The group map ℤ/n → ℚ/ℤ, x ↦ x/n: injective, the way back from
    /// [`clear_denominators`]. Not a ring map; ℚ/ℤ isn't a ring.
    pub fn fraction(&self) -> QModOne {
        QModOne::reduced(self.residue.clone(), self.modulus.0.clone())
    }

    fn with(&self, x: BigInt) -> Self {
        ZMod {
            residue: self.modulus.residue(x),
            modulus: self.modulus.clone(),
        }
    }

    fn same(&self, other: &Self) {
        assert!(
            self.modulus == other.modulus,
            "ℤ/{} and ℤ/{} don't mix: check RingPart::admits first",
            self.modulus.0,
            other.modulus.0
        );
    }
}

impl Coefficient for ZMod {
    type Ring = Modulus;
    fn ring(&self) -> &Modulus {
        &self.modulus
    }
    fn zero(n: &Modulus) -> Self {
        ZMod::new(0, n)
    }
    fn add(&self, other: &Self) -> Self {
        self.same(other);
        self.with(&self.residue + &other.residue)
    }
    fn neg(&self) -> Self {
        self.with(-&self.residue)
    }
    fn times(&self, n: &BigInt) -> Self {
        self.with(n * &self.residue)
    }
}

impl Entry for ZMod {
    fn one(n: &Modulus) -> Self {
        ZMod::new(1, n)
    }
    fn mul(&self, other: &Self) -> Self {
        self.same(other);
        self.with(&self.residue * &other.residue)
    }
}

impl Exactly for ZMod {}

/// A rational mod 1, exactly: num/den reduced, with 0 ≤ num < den. The
/// circle U(1) restricted to its exact (torsion) points, ℚ/ℤ.
///
/// An abelian group, a [`Coefficient`], but not a ring: 1/3 ≡ 4/3, while
/// (1/3)² = 1/9 and (4/3)² = 16/9 ≢ 1/9, so a product isn't defined mod 1.
/// δ over ℤ acts on its cochains ([`Map::act`]); there are no maps with
/// entries in it.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct QModOne {
    num: BigInt,
    den: BigInt,
}

impl QModOne {
    /// num/den mod 1, reduced. `None` when den is 0.
    pub fn new(num: impl Into<BigInt>, den: impl Into<BigInt>) -> Option<Self> {
        let (num, den) = (num.into(), den.into());
        match den.sign() {
            Sign::NoSign => None,
            Sign::Minus => Some(QModOne::reduced(-num, -den)),
            Sign::Plus => Some(QModOne::reduced(num, den)),
        }
    }

    /// The numerator, in 0..den.
    pub fn numerator(&self) -> &BigInt {
        &self.num
    }

    /// The denominator, ≥ 1, coprime to the numerator.
    pub fn denominator(&self) -> &BigInt {
        &self.den
    }

    /// num/den mod 1 for den > 0.
    fn reduced(num: BigInt, den: BigInt) -> Self {
        let mut num = num % &den;
        if num.sign() == Sign::Minus {
            num += &den;
        }
        let g = gcd(num.clone(), den.clone());
        QModOne {
            num: num / &g,
            den: den / g,
        }
    }
}

/// gcd(a, b) for a ≥ 0, b > 0.
fn gcd(mut a: BigInt, mut b: BigInt) -> BigInt {
    while !b.is_zero() {
        let r = a % &b;
        a = b;
        b = r;
    }
    a
}

impl Coefficient for QModOne {
    type Ring = ();
    fn ring(&self) -> &() {
        &()
    }
    fn zero(_: &()) -> Self {
        QModOne {
            num: BigInt::zero(),
            den: BigInt::one(),
        }
    }
    fn add(&self, other: &Self) -> Self {
        QModOne::reduced(
            &self.num * &other.den + &other.num * &self.den,
            &self.den * &other.den,
        )
    }
    fn neg(&self) -> Self {
        QModOne::reduced(-&self.num, self.den.clone())
    }
    fn times(&self, n: &BigInt) -> Self {
        QModOne::reduced(n * &self.num, self.den.clone())
    }
}

impl Exactly for QModOne {}

/// ℚ mod 1 as ℤ/m: m the lcm of the denominators, each value c = c′/m
/// with c′ = c·m mod m. A twist δg ≡ c (mod 1) is then δg′ ≡ c′ (mod m),
/// and every witness maps back by [`ZMod::fraction`]. With every value 0
/// (m = 1, refused as ℤ/1) m is 2, which holds 0 too.
pub fn clear_denominators(c: &[QModOne]) -> (Modulus, Vec<ZMod>) {
    let lcm = c.iter().fold(BigInt::one(), |m, x| {
        let g = gcd(x.den.clone(), m.clone());
        m / g * &x.den
    });
    let m = Modulus::new(lcm).unwrap_or_else(|| Modulus(BigInt::from(2)));
    let reduced = c
        .iter()
        .map(|x| ZMod::new(&x.num * (&m.0 / &x.den), &m))
        .collect();
    (m, reduced)
}

impl Coefficient for f64 {
    type Ring = ();
    fn ring(&self) -> &() {
        &()
    }
    fn zero(_: &()) -> Self {
        0.0
    }
    fn add(&self, other: &Self) -> Self {
        self + other
    }
    fn neg(&self) -> Self {
        -self
    }
    fn times(&self, n: &BigInt) -> Self {
        n.to_f64().expect("every BigInt has a float reading") * self
    }
}

impl Entry for f64 {
    fn one(_: &()) -> Self {
        1.0
    }
    fn mul(&self, other: &Self) -> Self {
        self * other
    }
}

// --- Maps --------------------------------------------------------------------

/// A linear map from room `A` to room `B` with entries `R`: a door.
///
/// As a matrix it has `to().dim()` rows and `from().dim()` columns. The
/// storage is column-major and private; read it through [`Map::entry`],
/// [`Map::apply`] or [`Modes`].
///
/// It carries its ring ([`Map::ring`]): the kind is `R`, checked at compile
/// time; the modulus of ℤ/n is a value, checked like the rooms.
#[derive(Clone, Debug, PartialEq)]
pub struct Map<A: Kind, B: Kind, R: Entry> {
    from: Room<A>,
    to: Room<B>,
    ring: R::Ring,
    /// Column-major: entry `(row, col)` is at `col * to.dim + row`.
    entries: Vec<R>,
}

/// `Ok` when the lengths agree.
fn length(expected: usize, found: usize) -> Result<(), RoomError> {
    if expected == found {
        Ok(())
    } else {
        Err(RoomError::Length { expected, found })
    }
}

/// `Ok` when every value lives in `ring`.
fn within<'a, V: Coefficient + 'a>(
    ring: &V::Ring,
    values: impl IntoIterator<Item = &'a V>,
) -> Result<(), RoomError> {
    values.into_iter().try_for_each(|x| ring.admits(x.ring()))
}

impl<A: Kind, B: Kind, R: Entry<Ring = ()>> Map<A, B, R> {
    /// The map that sends everything to zero.
    pub fn zero(from: &Room<A>, to: &Room<B>) -> Self {
        Map::zero_in(&(), from, to)
    }

    /// A map from its rows: `to.dim()` rows of `from.dim()` entries each.
    pub fn from_rows(from: &Room<A>, to: &Room<B>, rows: &[Vec<R>]) -> Result<Self, RoomError> {
        Map::from_rows_in(&(), from, to, rows)
    }
}

impl<A: Kind, B: Kind, R: Entry> Map<A, B, R> {
    /// The map over `ring` that sends everything to zero.
    pub fn zero_in(ring: &R::Ring, from: &Room<A>, to: &Room<B>) -> Self {
        Map {
            from: from.clone(),
            to: to.clone(),
            ring: ring.clone(),
            entries: vec![R::zero(ring); from.dim * to.dim],
        }
    }

    /// A map over `ring` from its rows: `to.dim()` rows of `from.dim()`
    /// entries each, every one in `ring`.
    pub fn from_rows_in(
        ring: &R::Ring,
        from: &Room<A>,
        to: &Room<B>,
        rows: &[Vec<R>],
    ) -> Result<Self, RoomError> {
        length(to.dim, rows.len())?;
        for row in rows {
            length(from.dim, row.len())?;
            within(ring, row)?;
        }
        let entries = (0..from.dim)
            .flat_map(|col| rows.iter().map(move |row| row[col].clone()))
            .collect();
        Ok(Map {
            from: from.clone(),
            to: to.clone(),
            ring: ring.clone(),
            entries,
        })
    }

    /// The ring the entries live in: its runtime part.
    pub fn ring(&self) -> &R::Ring {
        &self.ring
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

    /// The map applied to a vector of `from`, in the map's ring.
    pub fn apply(&self, x: &[R]) -> Result<Vec<R>, RoomError> {
        length(self.from.dim, x.len())?;
        within(&self.ring, x)?;
        let mut y = vec![R::zero(&self.ring); self.to.dim];
        for (column, xj) in self.columns().zip(x) {
            for (yi, aij) in y.iter_mut().zip(column) {
                *yi = yi.add(&aij.mul(xj));
            }
        }
        Ok(y)
    }

    /// First this map, then `next`: path order, as [`Group::compose`].
    /// `next` must leave the room this map enters: by kind at compile time,
    /// by dimension and complex here, and in one ring.
    pub fn then<C: Kind>(&self, next: &Map<B, C, R>) -> Result<Map<A, C, R>, RoomError> {
        self.to.admits(&next.from)?;
        self.ring.admits(&next.ring)?;
        let mut entries = Vec::with_capacity(self.from.dim * next.to.dim);
        for column in self.columns() {
            entries.extend(next.apply(column)?);
        }
        Ok(Map {
            from: self.from.clone(),
            to: next.to.clone(),
            ring: self.ring.clone(),
            entries,
        })
    }

    /// `self + other`, between the same two rooms, in one ring.
    pub fn plus(&self, other: &Self) -> Result<Self, RoomError> {
        self.from.admits(&other.from)?;
        self.to.admits(&other.to)?;
        self.ring.admits(&other.ring)?;
        let entries = self
            .entries
            .iter()
            .zip(&other.entries)
            .map(|(a, b)| a.add(b))
            .collect();
        Ok(self.with(entries, self.ring.clone()))
    }

    /// `−self`.
    pub fn negate(&self) -> Self {
        self.with(
            self.entries.iter().map(Coefficient::neg).collect(),
            self.ring.clone(),
        )
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
            ring: self.ring.clone(),
            entries,
        }
    }

    /// self*·self, from `A` to itself, without a room check: both doors are
    /// this map's. Entry (i, j) is column i · column j, so it and (j, i) are
    /// one product, exactly: symmetric by construction.
    fn gram(&self) -> Map<A, A, R> {
        let columns: Vec<&[R]> = self.columns().collect();
        let dot = |x: &[R], y: &[R]| {
            x.iter()
                .zip(y)
                .fold(R::zero(&self.ring), |sum, (a, b)| sum.add(&a.mul(b)))
        };
        let entries = columns
            .iter()
            .flat_map(|cj| columns.iter().map(|ci| dot(ci, cj)))
            .collect();
        Map {
            from: self.from.clone(),
            to: self.from.clone(),
            ring: self.ring.clone(),
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

    fn with<S: Entry>(&self, entries: Vec<S>, ring: S::Ring) -> Map<A, B, S> {
        Map {
            from: self.from.clone(),
            to: self.to.clone(),
            ring,
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
        self.with(self.entries.iter().map(Measure::measure).collect(), ())
    }
}

impl<A: Kind, B: Kind> Map<A, B, Z> {
    /// The ring map ℤ → ℤ/n on every entry: this map over ℤ/n.
    pub fn reduce(&self, n: &Modulus) -> Map<A, B, ZMod> {
        self.with(
            self.entries.iter().map(|x| x.reduce(n)).collect(),
            n.clone(),
        )
    }

    /// This integer map acting on a vector of `from` with values in any
    /// [`Coefficient`] group, by n · x: how δ over ℤ moves a cochain of
    /// ℚ mod 1, which has no maps of its own. On a vector of ℤ it is
    /// [`apply`](Map::apply).
    pub fn act<V: Coefficient>(&self, ring: &V::Ring, x: &[V]) -> Result<Vec<V>, RoomError> {
        length(self.from.dim, x.len())?;
        within(ring, x)?;
        let mut y = vec![V::zero(ring); self.to.dim];
        for (column, xj) in self.columns().zip(x) {
            for (yi, aij) in y.iter_mut().zip(column) {
                if !aij.0.is_zero() {
                    *yi = yi.add(&xj.times(&aij.0));
                }
            }
        }
        Ok(y)
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

impl<A: Kind, R: Entry<Ring = ()>> Map<A, A, R> {
    /// The map that changes nothing.
    pub fn identity(room: &Room<A>) -> Self {
        Map::identity_in(&(), room)
    }
}

impl<A: Kind, R: Entry> Map<A, A, R> {
    /// The map over `ring` that changes nothing.
    pub fn identity_in(ring: &R::Ring, room: &Room<A>) -> Self {
        let mut map = Map::zero_in(ring, room, room);
        for i in 0..room.dim {
            map.set(i, i, R::one(ring));
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

// The sum's maps are over the rings whose kind says it all (ℤ, f64): D over
// ℤ/n isn't needed yet.

impl<A: Kind, B: Kind, R: Entry<Ring = ()>> Map<A, Sum<A, B>, R> {
    /// A into A ⊕ B.
    pub fn inject_left(room: &Room<Sum<A, B>>) -> Self {
        let mut map = Map::zero(room.left(), room);
        for i in 0..room.left().dim {
            map.set(i, i, R::one(&()));
        }
        map
    }
}

impl<A: Kind, B: Kind, R: Entry<Ring = ()>> Map<B, Sum<A, B>, R> {
    /// B into A ⊕ B.
    pub fn inject_right(room: &Room<Sum<A, B>>) -> Self {
        let offset = room.left().dim;
        let mut map = Map::zero(room.right(), room);
        for i in 0..room.right().dim {
            map.set(offset + i, i, R::one(&()));
        }
        map
    }
}

impl<A: Kind, B: Kind, R: Entry<Ring = ()>> Map<Sum<A, B>, A, R> {
    /// A ⊕ B onto A.
    pub fn project_left(room: &Room<Sum<A, B>>) -> Self {
        Map::inject_left(room).adjoint()
    }
}

impl<A: Kind, B: Kind, R: Entry<Ring = ()>> Map<Sum<A, B>, B, R> {
    /// A ⊕ B onto B.
    pub fn project_right(room: &Room<Sum<A, B>>) -> Self {
        Map::inject_right(room).adjoint()
    }
}

impl<A: Kind, B: Kind, C: Kind, D: Kind, R: Entry<Ring = ()>> Map<Sum<A, B>, Sum<C, D>, R> {
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

impl<A: Kind, B: Kind, R: Entry<Ring = ()>> Map<Sum<A, B>, Sum<A, B>, R> {
    /// γ, the grading of A ⊕ B: +1 on `A`, −1 on `B`. A self-adjoint
    /// involution; D anticommutes with it on C⁰ ⊕ C¹, because D only moves
    /// between the summands.
    pub fn grading(room: &Room<Sum<A, B>>) -> Self {
        let mut map = Map::identity(room);
        for i in room.left().dim..room.dim {
            map.set(i, i, R::one(&()).neg());
        }
        map
    }
}

// --- Resonance ---------------------------------------------------------------

/// A map from a room to itself that equals its own adjoint, checked: a
/// room's resonator. Only [`Map::symmetric`] makes one.
#[derive(Clone, Debug, PartialEq)]
pub struct Symmetric<A: Kind, R: Entry>(Map<A, A, R>);

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

    /// The same polynomial in ℤ[x], for exact arithmetic on it: its
    /// square-free part, its real roots ([`crate::poly`]).
    pub fn poly(&self) -> crate::poly::ZPoly {
        crate::poly::ZPoly::from_descending(self.coefficients.iter().map(|c| c.0.clone()))
    }
}

// --- Smith normal form -------------------------------------------------------

/// The Smith normal form of an integer map M: A → B, exactly: unimodular U
/// on B and V on A with U·M·V = diag(d₁, d₂, …), d₁ | d₂ | …, each
/// dᵢ ≥ 0 and the zeros last.
///
/// The factors are what M is up to a change of basis on either side, over
/// ℤ: the cokernel B / im M is ⊕ ℤ/dᵢ ⊕ ℤ^(dim B − rank). A factor
/// dᵢ > 1 is torsion: an element of B that isn't in the image while dᵢ
/// times it is. Over ℤ/n the factor dᵢ acts as gcd(dᵢ, n).
#[derive(Clone, Debug, PartialEq)]
pub struct Smith<A: Kind, B: Kind> {
    factors: Vec<Z>,
    left: Map<B, B, Z>,
    right: Map<A, A, Z>,
    from: Room<A>,
    to: Room<B>,
}

impl<A: Kind, B: Kind> Smith<A, B> {
    /// d₁ | d₂ | …: min(dim A, dim B) of them, nonzero first.
    pub fn factors(&self) -> &[Z] {
        &self.factors
    }

    /// The rank of M: how many factors are nonzero.
    pub fn rank(&self) -> usize {
        self.factors.iter().filter(|d| !d.0.is_zero()).count()
    }

    /// U, on M's target: the row operations.
    pub fn left(&self) -> &Map<B, B, Z> {
        &self.left
    }

    /// V, on M's source: the column operations.
    pub fn right(&self) -> &Map<A, A, Z> {
        &self.right
    }

    /// diag(d): the map U·M·V, from A to B.
    pub fn diagonal(&self) -> Map<A, B, Z> {
        let mut d = Map::zero(&self.from, &self.to);
        for (i, di) in self.factors.iter().enumerate() {
            d.set(i, i, di.clone());
        }
        d
    }
}

impl<A: Kind, B: Kind> Map<A, B, Z> {
    /// The Smith normal form ([`Smith`]), by elimination over ℤ: pivot on
    /// the smallest nonzero entry left, clear its row and column by division
    /// with remainder (a remainder becomes the next, smaller pivot), and
    /// when the pivot doesn't divide some entry left, add that entry's row to
    /// the pivot's and go again. Each pass shrinks the pivot, so it ends.
    /// Every step is an exact integer row or column operation, recorded in
    /// U or V.
    pub fn smith(&self) -> Smith<A, B> {
        let (m, n) = (self.to.dim, self.from.dim);
        let mut a: Vec<Vec<BigInt>> = (0..m)
            .map(|i| {
                (0..n)
                    .map(|j| self.entries[self.at(i, j)].0.clone())
                    .collect()
            })
            .collect();
        let unit = |k: usize| -> Vec<Vec<BigInt>> {
            (0..k)
                .map(|i| (0..k).map(|j| BigInt::from(u8::from(i == j))).collect())
                .collect()
        };
        let (mut u, mut v) = (unit(m), unit(n));
        // row_i -= q·row_t on A and U; col_j -= q·col_t on A and V.
        let sub_row = |a: &mut Vec<Vec<BigInt>>, i: usize, t: usize, q: &BigInt| {
            let pivot = a[t].clone();
            for (x, p) in a[i].iter_mut().zip(&pivot) {
                *x -= q * p;
            }
        };
        let sub_col = |a: &mut Vec<Vec<BigInt>>, j: usize, t: usize, q: &BigInt| {
            for row in a.iter_mut() {
                let p = row[t].clone();
                row[j] -= q * p;
            }
        };
        let mut factors = Vec::with_capacity(m.min(n));
        for t in 0..m.min(n) {
            loop {
                let smallest = (t..m)
                    .flat_map(|i| (t..n).map(move |j| (i, j)))
                    .filter(|&(i, j)| !a[i][j].is_zero())
                    .min_by_key(|&(i, j)| a[i][j].magnitude().clone());
                let Some((p, q)) = smallest else { break };
                a.swap(t, p);
                u.swap(t, p);
                for row in a.iter_mut().chain(v.iter_mut()) {
                    row.swap(t, q);
                }
                let mut clear = true;
                for i in t + 1..m {
                    let q = &a[i][t] / &a[t][t];
                    sub_row(&mut a, i, t, &q);
                    sub_row(&mut u, i, t, &q);
                    clear &= a[i][t].is_zero();
                }
                for j in t + 1..n {
                    let q = &a[t][j] / &a[t][t];
                    sub_col(&mut a, j, t, &q);
                    sub_col(&mut v, j, t, &q);
                    clear &= a[t][j].is_zero();
                }
                if !clear {
                    continue;
                }
                let pivot = a[t][t].clone();
                let stray =
                    (t + 1..m).find(|&i| (t + 1..n).any(|j| !(&a[i][j] % &pivot).is_zero()));
                match stray {
                    Some(i) => {
                        let minus_one = BigInt::from(-1);
                        sub_row(&mut a, t, i, &minus_one);
                        sub_row(&mut u, t, i, &minus_one);
                    }
                    None => break,
                }
            }
            if a[t][t].sign() == Sign::Minus {
                for x in a[t].iter_mut().chain(u[t].iter_mut()) {
                    *x = -&*x;
                }
            }
            factors.push(Z(a[t][t].clone()));
        }
        fn square<K: Kind>(room: &Room<K>, rows: Vec<Vec<BigInt>>) -> Map<K, K, Z> {
            let rows: Vec<Vec<Z>> = rows
                .into_iter()
                .map(|r| r.into_iter().map(Z).collect())
                .collect();
            Map::from_rows(room, room, &rows).expect("square, by construction")
        }
        Smith {
            factors,
            left: square(&self.to, u),
            right: square(&self.from, v),
            from: self.from.clone(),
            to: self.to.clone(),
        }
    }

    /// Row `i`, as a vector of `from`.
    fn row(&self, i: usize) -> Vec<Z> {
        (0..self.from.dim)
            .map(|j| self.entries[self.at(i, j)].clone())
            .collect()
    }
}

impl<A: Kind> Map<A, A, Z> {
    /// det M, exactly, by Bareiss's fraction-free elimination: every
    /// division is exact, so every step stays in ℤ. The empty map's is 1.
    pub fn determinant(&self) -> Z {
        let n = self.from.dim;
        let mut a: Vec<Vec<BigInt>> = (0..n)
            .map(|i| {
                (0..n)
                    .map(|j| self.entries[self.at(i, j)].0.clone())
                    .collect()
            })
            .collect();
        let mut sign = BigInt::one();
        let mut previous = BigInt::one();
        for k in 0..n {
            let Some(p) = (k..n).find(|&i| !a[i][k].is_zero()) else {
                return Z(BigInt::zero());
            };
            if p != k {
                a.swap(p, k);
                sign = -sign;
            }
            for i in k + 1..n {
                for j in k + 1..n {
                    a[i][j] = (&a[i][j] * &a[k][k] - &a[i][k] * &a[k][j]) / &previous;
                }
            }
            previous = a[k][k].clone();
        }
        Z(sign * previous)
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
/// One row per edge: m parallel edges on u–v are m rows, so L₀ = δ*δ adds m
/// to each end's diagonal and −m off it. A self-loop (v, v) has the zero
/// row: s(v) − s(v) = 0. The two ends are summed, not written over each
/// other.
pub fn coboundary(graph: &Graph) -> Map<Vertices, Edges, Z> {
    delta(&(), &Room::vertices(graph), graph)
}

/// δ over any ring: the same ±1 and 0, which every ring has. Over ℤ/n it
/// is [`coboundary`] reduced ([`Map::reduce`]); over ℤ it is
/// [`coboundary`].
pub fn coboundary_in<R: Entry>(ring: &R::Ring, graph: &Graph) -> Map<Vertices, Edges, R> {
    delta(ring, &Room::vertices(graph), graph)
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
    Ok(delta(&(), skeleton, graph))
}

/// δ over `ring` out of `skeleton`, which holds the graph's vertices.
fn delta<R: Entry>(
    ring: &R::Ring,
    skeleton: &Room<Vertices>,
    graph: &Graph,
) -> Map<Vertices, Edges, R> {
    let index: BTreeMap<_, _> = skeleton
        .parts
        .iter()
        .enumerate()
        .map(|(i, &v)| (v, i))
        .collect();
    let mut delta: Map<Vertices, Edges, R> =
        Map::zero_in(ring, skeleton, &Room::edges_over(skeleton, graph));
    let one = R::one(ring);
    for (row, (u, v)) in graph.edges().map(|e| e.ends()).enumerate() {
        for (w, sign) in [(u, one.neg()), (v, one.clone())] {
            let at = delta.at(row, index[&w]);
            delta.entries[at] = delta.entries[at].add(&sign);
        }
    }
    delta
}

/// δ₁: C¹ → C², exactly. `(δ₁c)(f) = Σ ±c(e)` around the face, +1 on
/// an edge walked from its smaller end, −1 against it: the twist around f
/// in its canonical orientation ([`Face`]), the holonomy field reports on
/// it. A face's row is its boundary ∂f as a chain, so ∂₂ = δ₁*.
///
/// An edge walked twice adds twice: the face of a self-loop walked twice
/// (a bigon on one edge) is 2 there, the 1-cell ℝℙ². A self-loop walked
/// once counts +1, as in [`Cycle::chain`].
///
/// δ₁ ∘ δ = 0: every face is a closed walk, so the two ends of each step
/// cancel around it. Refused ([`RoomError::Outside`]) when a face runs
/// along an edge the graph doesn't have.
pub fn coboundary1(graph: &Graph, faces: &[Face]) -> Result<Map<Edges, Faces, Z>, RoomError> {
    coboundary1_in(&(), graph, faces)
}

/// δ₁ over any ring: the same integers, which every ring has.
pub fn coboundary1_in<R: Entry>(
    ring: &R::Ring,
    graph: &Graph,
    faces: &[Face],
) -> Result<Map<Edges, Faces, R>, RoomError> {
    let rows = boundaries(graph, faces)?;
    let edges = Room::edges(graph);
    let mut delta: Map<Edges, Faces, R> =
        Map::zero_in(ring, &edges, &Room::faces_over(&edges, faces));
    for (row, chain) in rows.iter().enumerate() {
        for (col, n) in chain.iter().enumerate() {
            if !n.0.is_zero() {
                delta.set(row, col, R::one(ring).times(&n.0));
            }
        }
    }
    Ok(delta)
}

/// Each face's boundary as an integer chain over `graph`'s edges.
fn boundaries(graph: &Graph, faces: &[Face]) -> Result<Vec<Vec<Z>>, RoomError> {
    let m = graph.edges().count();
    faces
        .iter()
        .map(|face| {
            let mut chain = vec![Z::zero(&()); m];
            for (&from, &e) in face.vertices().iter().zip(face.edges()) {
                let i = graph.index(e).ok_or(RoomError::Outside(e))?;
                let sign = if e.ends().0 == from { 1 } else { -1 };
                chain[i] = chain[i].add(&Z::from(sign));
            }
            Ok(chain)
        })
        .collect()
}

/// L₀ = δ*δ on C⁰ of `graph`, exactly: symmetric by construction.
pub fn laplacian(graph: &Graph) -> Symmetric<Vertices, Z> {
    Symmetric(coboundary(graph).gram())
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

// --- Exact or genuine --------------------------------------------------------

/// Whether a twist c ∈ C¹ is exact (c = δg) or a genuine class, decided in
/// its own ring, with the witness.
#[derive(Clone, Debug, PartialEq)]
pub enum Exactness<V> {
    /// c = δg: the potential g, a vector of C⁰ (vertices ascending), zero
    /// at each component's smallest vertex.
    Exact(Vec<V>),
    /// No g has δg = c: every basis cycle whose pairing with c isn't zero.
    /// Never empty.
    Genuine(Vec<Cycle<V>>),
}

/// A certificate that no potential exists: a cycle z with ⟨z, c⟩ ≠ 0.
///
/// z is an integer chain with δ*z = 0, so for every g,
/// ⟨z, δg⟩ = ⟨δ*z, g⟩ = 0; a c with ⟨z, c⟩ ≠ 0 isn't any δg.
#[derive(Clone, Debug, PartialEq)]
pub struct Cycle<V> {
    vertices: Vec<VertexId>,
    edges: Vec<Edge>,
    chain: Vec<Z>,
    pairing: V,
}

impl<V> Cycle<V> {
    /// The loop, from its smallest vertex back to it, as
    /// [`field::Loop::vertices`](crate::field::Loop::vertices).
    pub fn vertices(&self) -> &[VertexId] {
        &self.vertices
    }
    /// The edges around it in path order: `edges()[k]` joins `vertices()[k]`
    /// and `vertices()[k + 1]`.
    pub fn edges(&self) -> &[Edge] {
        &self.edges
    }
    /// z as a vector of C¹: +1 on an edge walked from its smaller end, −1
    /// against it, 0 off the loop. A self-loop is +1.
    pub fn chain(&self) -> &[Z] {
        &self.chain
    }
    /// ⟨z, c⟩, the twist around the loop: never zero.
    pub fn pairing(&self) -> &V {
        &self.pairing
    }
}

/// ⟨z, c⟩ = Σ zᵢ · cᵢ: an integer chain paired with a cochain over `ring`.
pub fn pair<V: Coefficient>(ring: &V::Ring, z: &[Z], c: &[V]) -> Result<V, RoomError> {
    length(z.len(), c.len())?;
    within(ring, c)?;
    Ok(z.iter()
        .zip(c)
        .fold(V::zero(ring), |sum, (zi, ci)| sum.add(&ci.times(&zi.0))))
}

/// Solve δg = c on the edges of `graph`, exactly, in `ring`: a potential g,
/// or the cycles that certify there is none.
///
/// The spanning-tree method, on the forest field's
/// [`check_flat`](crate::field::check_flat) walks (one forest, so the same
/// potential and the same loops). g is 0 at each component's root and
/// g(w) = g(u) + c(u → w) down each tree edge; then every chord closes a
/// fundamental cycle z, and c = δg exactly when every ⟨z, c⟩ is 0. Those
/// cycles are a basis of the cycle space (b₁ of them), so this decides.
///
/// - **No Smith normal form on edges.** A graph's incidence matrix is
///   totally unimodular: every square minor is 0 or ±1. So every invariant
///   factor of δ is 1, coker δ over ℤ is free (ℤ^b₁, no torsion), and over
///   any coefficient group c is exact iff every cycle pairs to 0 with it.
///   Torsion first appears on faces (δ₁: C¹ → C², ℝℙ²'s ℤ/2), which is
///   where Smith form is needed ([`bounds`]).
/// - **ℚ mod 1 is ℤ/m.** The method needs only an abelian group, so
///   [`QModOne`] is solved directly; with m the lcm of the denominators it
///   is the same solve as ℤ/m on [`clear_denominators`]'s c′, mapped back by
///   [`ZMod::fraction`]. One algorithm.
/// - **Only the filtration.** What this decides is c ∈ im δ₀, the first
///   step of exact ⊆ closed ⊆ all. On a graph (no 2-cells) every 1-cochain
///   is closed, so closed = all here. There is no orthogonal Hodge split
///   over ℤ/n or ℚ mod 1, so none is offered: a [`Cycle`] names the class
///   by its pairing, not by a harmonic representative. The orthogonal
///   split exists only measured, over `f64` (L₁↓'s modes).
///
/// Refused when c isn't one value per edge, or a value lives in another
/// ring than `ring` ([`RoomError::Modulus`]).
pub fn solve<V: Exactly>(
    ring: &V::Ring,
    graph: &Graph,
    c: &[V],
) -> Result<Exactness<V>, RoomError> {
    let edges: Vec<Edge> = graph.edges().collect();
    length(edges.len(), c.len())?;
    within(ring, c)?;
    // c along edge i, leaving from `from`: −c against its orientation.
    let forward = |i: usize, from: VertexId| edges[i].ends().0 == from;
    let step = |i: usize, from: VertexId| {
        if forward(i, from) {
            c[i].clone()
        } else {
            c[i].neg()
        }
    };
    let zero = V::zero(ring);
    let Spanning { order, cycles } = graph.walked();

    let mut potential: BTreeMap<VertexId, V> = BTreeMap::new();
    for (w, parent) in order {
        let g = match parent {
            None => zero.clone(),
            Some((u, i)) => potential[&u].add(&step(i, u)),
        };
        potential.insert(w, g);
    }

    let mut genuine = Vec::new();
    for walk in cycles {
        let pairing = walk
            .vertices
            .iter()
            .zip(&walk.edges)
            .fold(zero.clone(), |sum, (&from, &i)| sum.add(&step(i, from)));
        if pairing != zero {
            genuine.push(Cycle {
                chain: chain(&walk, &edges),
                edges: walk.edges.iter().map(|&i| edges[i]).collect(),
                vertices: walk.vertices,
                pairing,
            });
        }
    }
    Ok(if genuine.is_empty() {
        Exactness::Exact(potential.into_values().collect())
    } else {
        Exactness::Genuine(genuine)
    })
}

/// A closed walk as an integer chain z ∈ C₁: +1 for each step along an edge
/// from its smaller end, −1 against it. A self-loop walked once is +1.
fn chain(walk: &Walk, edges: &[Edge]) -> Vec<Z> {
    let mut z = vec![Z::zero(&()); edges.len()];
    for (&from, &i) in walk.vertices.iter().zip(&walk.edges) {
        let sign = if edges[i].ends().0 == from { 1 } else { -1 };
        z[i] = z[i].add(&Z::from(sign));
    }
    z
}

// --- Faces: curvature, three kinds of twist, and boundaries -----------------

/// Each face's curvature: (δ₁c)(f), the twist around f in its canonical
/// orientation, exactly in c's ring. The holonomy field's
/// [`check_flat`](crate::field::check_flat) reports on a face that doesn't
/// close.
///
/// Refused when c isn't one value per edge, or a value lives in another
/// ring than `ring`; or a face is off the graph.
pub fn curvature<V: Exactly>(
    ring: &V::Ring,
    graph: &Graph,
    faces: &[Face],
    c: &[V],
) -> Result<Vec<V>, RoomError> {
    coboundary1(graph, faces)?.act(ring, c)
}

/// What kind of twist a 1-cochain c is, on a graph filled by faces, decided
/// in its own ring, with the witness. The filtration exact ⊆ closed ⊆ all
/// (Seam's K2), read from the inside out.
#[derive(Clone, Debug, PartialEq)]
pub enum Twist<V> {
    /// c = δg, c ∈ im δ₀: a rotation removes it. The potential g, as
    /// [`Exactness::Exact`]'s.
    Gradient(Vec<V>),
    /// δ₁c = 0 but c ∉ im δ₀: a genuine comma, around a loop that bounds
    /// nothing. Every basis cycle z with ⟨z, c⟩ ≠ 0, as
    /// [`Exactness::Genuine`]'s. Never empty.
    ///
    /// Each z bounds no faces: c is closed, so for any 2-chain w,
    /// ⟨∂₂w, c⟩ = ⟨w, δ₁c⟩ = 0, and ⟨z, c⟩ ≠ 0 rules z = ∂₂w out.
    Harmonic(Vec<Cycle<V>>),
    /// δ₁c ≠ 0: a local twist on these faces, each with its (δ₁c)(f).
    /// Never empty.
    Curvature(Vec<Curl<V>>),
}

/// A face that doesn't close: curvature, located.
#[derive(Clone, Debug, PartialEq)]
pub struct Curl<V> {
    index: usize,
    face: Face,
    value: V,
}

impl<V> Curl<V> {
    /// Which face, by its place in the faces given: its dimension of C².
    pub fn index(&self) -> usize {
        self.index
    }
    /// The face.
    pub fn face(&self) -> &Face {
        &self.face
    }
    /// (δ₁c)(f), the twist around it: never zero.
    pub fn value(&self) -> &V {
        &self.value
    }
}

/// Classify c ∈ C¹ on `graph` filled by `faces`, exactly, in `ring`:
/// curvature if δ₁c ≠ 0; else gradient if c = δg ([`solve`]); else
/// harmonic.
///
/// Only the filtration exact ⊆ closed ⊆ all is promised, in every ring.
/// There is no orthogonal split c = gradient + harmonic + curl: over ℤ/n
/// and ℚ mod 1 there is no inner product to split by, so none is offered.
/// A curved c may hide a harmonic part too; it is reported by its faces,
/// never folded in. The complex's holes are [`holes`], c-independent, and a
/// reading of c along one names its cycle ([`Hole`]): on a curved c the
/// number depends on the path, so the path is the caller's to choose.
/// The orthogonal Hodge split exists only measured, over `f64`.
///
/// Refused as [`curvature`] and [`solve`] refuse.
pub fn classify<V: Exactly>(
    ring: &V::Ring,
    graph: &Graph,
    faces: &[Face],
    c: &[V],
) -> Result<Twist<V>, RoomError> {
    let zero = V::zero(ring);
    let curls: Vec<Curl<V>> = curvature(ring, graph, faces, c)?
        .into_iter()
        .zip(faces)
        .enumerate()
        .filter(|(_, (value, _))| *value != zero)
        .map(|(index, (value, face))| Curl {
            index,
            face: face.clone(),
            value,
        })
        .collect();
    if !curls.is_empty() {
        return Ok(Twist::Curvature(curls));
    }
    Ok(match solve(ring, graph, c)? {
        Exactness::Exact(g) => Twist::Gradient(g),
        Exactness::Genuine(cycles) => Twist::Harmonic(cycles),
    })
}

/// Whether a 1-chain z bounds faces, z = ∂₂w, in R, with the witness.
#[derive(Clone, Debug, PartialEq)]
pub enum Boundary<R: Bounded> {
    /// z = ∂₂w: the 2-chain w, one value per face.
    Bounds(Vec<R>),
    /// z is no ∂₂w: a cocycle c with values in R's [dual](Bounded::Dual),
    /// δ₁c = 0 and ⟨z, c⟩ ≠ 0. For every w, ⟨∂₂w, c⟩ = ⟨w, δ₁c⟩ = 0,
    /// so no w has ∂₂w = z.
    Unbounded(Vec<R::Dual>),
}

/// Rings whose linear equations Smith form decides: [`Z`] and [`ZMod`].
/// A diagonal equation d·x = y is solved one entry at a time, and when one
/// has no solution its row of U is turned into a separating cocycle.
pub trait Bounded: Entry + Exactly {
    /// Where the separating cocycles live: a group in which every z that
    /// isn't a boundary pairs nonzero with some cocycle. For ℤ that is
    /// ℚ/ℤ ([`QModOne`]): torsion is seen only mod 1 (ℝℙ²'s loop z pairs
    /// to 1/2, while every integer cocycle pairs to 0 with it). For ℤ/n it
    /// is ℤ/n itself.
    type Dual: Exactly;
    /// The dual's runtime part.
    fn dual(ring: &Self::Ring) -> <Self::Dual as Coefficient>::Ring;
    /// An x with d·x = self, if any. d is an invariant factor, ≥ 0.
    fn divide(&self, d: &BigInt) -> Option<Self>;
    /// When d·x = self has no solution: the cocycle s·u from U's row u, s
    /// chosen so that ⟨z, s·u⟩ = s·self ≠ 0 and s·d = 0.
    fn linking(&self, d: &BigInt, row: &[Z]) -> Vec<Self::Dual>;
    /// The order of the summand R/dR that the invariant factor d leaves in
    /// a cokernel, or `None` when d acts as a unit and leaves nothing.
    fn order(ring: &Self::Ring, d: &BigInt) -> Option<Order>;
}

impl Bounded for Z {
    type Dual = QModOne;
    fn dual(_: &()) {}
    fn divide(&self, d: &BigInt) -> Option<Self> {
        if d.is_zero() {
            return self.0.is_zero().then(|| self.clone());
        }
        (&self.0 % d).is_zero().then(|| Z(&self.0 / d))
    }
    /// d > 0, d ∤ y: u/d, so ⟨z, c⟩ = y/d ≢ 0 mod 1. d = 0, y ≠ 0:
    /// u/(2y), so ⟨z, c⟩ = 1/2.
    fn linking(&self, d: &BigInt, row: &[Z]) -> Vec<QModOne> {
        let den = if d.is_zero() {
            BigInt::from(2) * &self.0
        } else {
            d.clone()
        };
        row.iter()
            .map(|u| QModOne::new(u.0.clone(), den.clone()).expect("a nonzero denominator"))
            .collect()
    }
    /// 0 leaves ℤ, free; 1 leaves nothing; d > 1 leaves ℤ/d.
    fn order(_: &(), d: &BigInt) -> Option<Order> {
        if d.is_zero() {
            Some(Order::Infinite)
        } else {
            (!d.is_one()).then(|| Order::Finite(Z(d.clone())))
        }
    }
}

impl Bounded for ZMod {
    type Dual = ZMod;
    fn dual(n: &Modulus) -> Modulus {
        n.clone()
    }
    /// d acts on ℤ/n as g = gcd(d, n): d·x = y is solvable iff g | y, and
    /// then x = (y/g)·(d/g)⁻¹ mod n/g.
    fn divide(&self, d: &BigInt) -> Option<Self> {
        let n = &self.modulus;
        let g = acting(d, n);
        if !(&self.residue % &g).is_zero() {
            return None;
        }
        let part = &n.0 / &g;
        let x = (&self.residue / &g) * inverse(&(d / &g), &part);
        Some(ZMod::new(x, n))
    }
    /// g ∤ y: (n/g)·u, so δ₁c = (n/g)·d·(…) ≡ 0 and ⟨z, c⟩ = (n/g)·y ≢ 0.
    fn linking(&self, d: &BigInt, row: &[Z]) -> Vec<ZMod> {
        let n = &self.modulus;
        let s = &n.0 / acting(d, n);
        row.iter().map(|u| ZMod::new(&s * &u.0, n)).collect()
    }
    /// d leaves ℤ/n / dℤ/n = ℤ/gcd(d, n): nothing when the gcd is 1, and
    /// all of ℤ/n for d = 0.
    fn order(n: &Modulus, d: &BigInt) -> Option<Order> {
        let g = acting(d, n);
        (!g.is_one()).then_some(Order::Finite(Z(g)))
    }
}

/// How the invariant factor d acts on ℤ/n: as gcd(d, n), so 0 acts as n.
fn acting(d: &BigInt, n: &Modulus) -> BigInt {
    gcd(d.clone(), n.0.clone())
}

/// a⁻¹ mod m, for a coprime to m ≥ 1, by the extended Euclidean algorithm.
fn inverse(a: &BigInt, m: &BigInt) -> BigInt {
    let (mut r0, mut r1) = (m.clone(), a % m);
    let (mut s0, mut s1) = (BigInt::zero(), BigInt::one());
    while !r1.is_zero() {
        let q = &r0 / &r1;
        (r0, r1) = (r1.clone(), r0 - &q * &r1);
        (s0, s1) = (s1.clone(), s0 - &q * &s1);
    }
    s0
}

/// Decide whether z ∈ C₁ bounds faces, z = ∂₂w, exactly, in `ring`: the
/// 2-chain w, or a cocycle that certifies there is none.
///
/// By the Smith form of ∂₂ = δ₁* over ℤ, U·∂₂·V = diag(d): ∂₂w = z
/// exactly when diag(d)·x = U·z with w = V·x. U and V are unimodular, so
/// they stay invertible over ℤ/n, and the equation splits into dᵢ·xᵢ = yᵢ,
/// one per factor (dᵢ = 0 past the rank). Over ℤ/n, dᵢ acts as gcd(dᵢ, n):
/// ℝℙ²'s factor 2 is 0 over ℤ/2 (the loop through the cross-cap doesn't
/// bound) and a unit over ℤ/3 (it does). When some yᵢ isn't reachable,
/// row i of U, scaled ([`Bounded::linking`]), is a cocycle that pairs
/// nonzero with z: row i of U·∂₂ is dᵢ times row i of V⁻¹, so the scale
/// that kills dᵢ makes it closed.
///
/// Refused when z isn't one value per edge, or a value lives in another
/// ring than `ring`; or a face is off the graph.
pub fn bounds<R: Bounded>(
    ring: &R::Ring,
    graph: &Graph,
    faces: &[Face],
    z: &[R],
) -> Result<Boundary<R>, RoomError> {
    let boundary = coboundary1(graph, faces)?.adjoint();
    length(boundary.to.dim, z.len())?;
    within(ring, z)?;
    let smith = boundary.smith();
    let y = smith.left.act(ring, z)?;
    let none = BigInt::zero();
    let mut x = Vec::with_capacity(faces.len());
    for (i, yi) in y.iter().enumerate() {
        let d = smith.factors.get(i).map_or(&none, |d| &d.0);
        match yi.divide(d) {
            Some(xi) if i < faces.len() => x.push(xi),
            Some(_) => {}
            None => return Ok(Boundary::Unbounded(yi.linking(d, &smith.left.row(i)))),
        }
    }
    x.resize(faces.len(), R::zero(ring));
    Ok(Boundary::Bounds(smith.right.act(ring, &x)?))
}

// --- Holes: H₁ of the complex ------------------------------------------------

/// The order of a hole in H₁: how many times around it bounds.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Order {
    /// No multiple bounds: a free generator, a ℤ of H₁(ℤ).
    Infinite,
    /// d·z bounds and no smaller positive multiple does: a ℤ/d. Over ℤ that
    /// is torsion (ℝℙ²'s 2); over ℤ/n every generator is finite, a free one
    /// of order n and a torsion one of order gcd(d, n).
    Finite(Z),
}

/// One generator of H₁ of the complex: a cycle that bounds no faces, with
/// its order and the certificate that it bounds nothing. A property of the
/// complex, never of a cochain c.
///
/// A reading of c along the hole is `pair(ring, hole.cycle(), c)`
/// ([`pair`]), and it belongs to this cycle, not to the hole: when c is
/// curved, another cycle z′ around the same hole, z′ = z + ∂₂w, reads
/// ⟨z′, c⟩ = ⟨z, c⟩ + ⟨w, δ₁c⟩. So a reading names its path, (z, ⟨z, c⟩),
/// and which path is the caller's choice. Curvature ([`curvature`],
/// [`Twist::Curvature`]) stays per face and path-free; nothing here folds
/// the two into one verdict.
#[derive(Clone, Debug, PartialEq)]
pub struct Hole<R: Bounded> {
    cycle: Vec<Z>,
    order: Order,
    certificate: Vec<R::Dual>,
}

impl<R: Bounded> Hole<R> {
    /// z, an integer chain of C¹'s edges with ∂₁z = 0 (δ*z = 0), read in R
    /// by reduction. As [`Cycle::chain`], but a sum of basis loops, not
    /// always one walk.
    pub fn cycle(&self) -> &[Z] {
        &self.cycle
    }
    /// Its order in H₁(R).
    pub fn order(&self) -> &Order {
        &self.order
    }
    /// A cocycle c with values in R's [dual](Bounded::Dual): δ₁c = 0 and
    /// ⟨z, c⟩ ≠ 0, so z bounds nothing ([`Boundary::Unbounded`]'s kind).
    /// The certificates of one [`holes`] call are dual to its cycles:
    /// ⟨zᵢ, cⱼ⟩ = 0 for i ≠ j.
    pub fn certificate(&self) -> &[R::Dual] {
        &self.certificate
    }
}

/// The genuine holes of `graph` filled by `faces`, exactly, in `ring`: a
/// basis of H₁ = ker ∂₁ / im ∂₂, each generator a cycle with its order and
/// its certificate. Independent of any cochain.
///
/// - **Cycles.** [`Graph::walked`]'s fundamental cycles z₁ … z_b₁ are a
///   ℤ-basis of ker ∂₁ (chord k lies on z_k alone, with sign s_k = ±1, so a
///   cycle's coordinates are its chord values times s_k). ∂₁ is totally
///   unimodular, so they stay a basis of ker ∂₁ over every ℤ/n.
/// - **Boundaries.** ∂₂ in those coordinates is the b₁ × F integer matrix A,
///   A[k][f] = s_k·(∂f)(chord k). Its Smith form U·A·V = diag(d) splits
///   H₁(ℤ) = ⊕ ℤ/dᵢ ⊕ ℤ^(b₁ − rank): a factor 1 is no hole, dᵢ > 1 is
///   torsion of order dᵢ, and 0 (and every i past F) is free.
/// - **Over ℤ/n**, dᵢ acts as gcd(dᵢ, n), so H₁(ℤ/n) = ⊕ ℤ/gcd(dᵢ, n):
///   a free generator gives ℤ/n, torsion dᵢ gives ℤ/gcd(dᵢ, n), nothing
///   when that's 1. That is the universal coefficient theorem,
///   H₁(ℤ/n) = H₁(ℤ) ⊗ ℤ/n, with no Tor term since H₀ is free. ℝℙ²: one
///   generator of order 2 over ℤ and over ℤ/2, none over ℤ/3.
/// - **Generators.** Generator i is column i of U⁻¹ in the cycle basis:
///   Σ_k (U⁻¹)ₖᵢ z_k. U⁻¹ is exact: with U′·U·V′ = I (U's own Smith form,
///   all units), U⁻¹ = V′·U′.
/// - **Certificates.** Row i of U, scaled by [`Bounded::linking`] so that
///   s·dᵢ = 0 (1/dᵢ over ℤ, 1/2 for a free one; n/gcd over ℤ/n), and put on
///   the chords (times s_k): δ₁c = s·dᵢ·(row i of V⁻¹) = 0, and
///   ⟨zⱼ, cᵢ⟩ = s·(U·U⁻¹)ᵢⱼ, so the certificates are dual to the cycles.
///
/// **The basis is a choice**, made deterministically by the forest and the
/// Smith transforms: generators come in Smith order (torsion by ascending
/// factor, then free). The counts and the orders are the invariants.
///
/// Refused ([`RoomError::Outside`]) when a face is off the graph.
pub fn holes<R: Bounded>(
    ring: &R::Ring,
    graph: &Graph,
    faces: &[Face],
) -> Result<Vec<Hole<R>>, RoomError> {
    let boundary = boundaries(graph, faces)?;
    let edges: Vec<Edge> = graph.edges().collect();
    let Spanning { cycles, .. } = graph.walked();
    let chords = graph.chords();
    let basis: Vec<Vec<Z>> = cycles.iter().map(|walk| chain(walk, &edges)).collect();
    let signs: Vec<Z> = chords
        .iter()
        .zip(&basis)
        .map(|(&e, z)| z[e].clone())
        .collect();
    let room = |k: usize| Room::over(&(0..k as u64).collect());
    let rows: Vec<Vec<Z>> = chords
        .iter()
        .zip(&signs)
        .map(|(&e, s)| boundary.iter().map(|df| df[e].mul(s)).collect())
        .collect();
    let a = Map::from_rows(&room(faces.len()), &room(basis.len()), &rows)
        .expect("b₁ rows of F, by construction");
    let smith = a.smith();
    let own = smith.left.smith();
    let inverse = own.left.then(&own.right).expect("one room");

    let none = BigInt::zero();
    let one = R::one(ring);
    let zero = <R::Dual as Coefficient>::zero(&R::dual(ring));
    let mut found = Vec::new();
    for i in 0..basis.len() {
        let d = smith.factors.get(i).map_or(&none, |d| &d.0);
        let Some(order) = R::order(ring, d) else {
            continue;
        };
        let mut cycle = vec![Z::zero(&()); edges.len()];
        for (k, zk) in basis.iter().enumerate() {
            let coefficient = &inverse.entries[inverse.at(k, i)].0;
            for (x, y) in cycle.iter_mut().zip(zk) {
                *x = x.add(&y.times(coefficient));
            }
        }
        let mut certificate = vec![zero.clone(); edges.len()];
        for (k, value) in one.linking(d, &smith.left.row(i)).into_iter().enumerate() {
            certificate[chords[k]] = value.times(&signs[k].0);
        }
        found.push(Hole {
            cycle,
            order,
            certificate,
        });
    }
    Ok(found)
}
