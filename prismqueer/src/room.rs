//! `prismqueer::room` — the spaces operators move between (red skeleton).
//!
//! Composition is checked by kind: a map from room A to room B only
//! composes with a map out of B. The control compiles:
//!
//! ```
//! use prismqueer::room::coboundary;
//! use prismqueer::spectral::graph_from_edges;
//! let delta = coboundary(&graph_from_edges(&[(0, 1), (1, 2), (0, 2)]));
//! let _ = delta.then(&delta.adjoint()); // C⁰ → C¹ → C⁰
//! ```
//!
//! and the only change, δ then δ, doesn't (rustdoc on stable doesn't check
//! the error code, which is why the control is here):
//!
//! ```compile_fail
//! use prismqueer::room::coboundary;
//! use prismqueer::spectral::graph_from_edges;
//! let delta = coboundary(&graph_from_edges(&[(0, 1), (1, 2), (0, 2)]));
//! let _ = delta.then(&delta); // C⁰ → C¹, then out of C⁰: C¹ isn't C⁰
//! ```
#![allow(missing_docs, unused_variables)]

use std::fmt::Debug;
use std::marker::PhantomData;

use crate::field::Exact;
use crate::oid::Oid;
use crate::spectral::kleinos::Graph;

pub trait Kind: Clone + PartialEq + Debug + 'static {
    type Parts: Clone + PartialEq + Debug;
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Vertices;
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Edges;
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Sum<A, B>(PhantomData<(A, B)>);

impl Kind for Vertices {
    type Parts = ();
}
impl Kind for Edges {
    type Parts = ();
}
impl<A: Kind, B: Kind> Kind for Sum<A, B> {
    type Parts = Box<(Room<A>, Room<B>)>;
}

#[derive(Clone, Debug, PartialEq)]
pub struct Room<K: Kind> {
    complex: Oid,
    dim: usize,
    parts: K::Parts,
    kind: PhantomData<K>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Place {
    pub dim: usize,
    pub complex: Oid,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RoomError {
    Mismatch { expected: Place, found: Place },
    Length { expected: usize, found: usize },
    Overflow,
    Asymmetric { row: usize, col: usize },
    Unmeasured(i32),
}

impl<K: Kind> Room<K> {
    pub fn dim(&self) -> usize {
        todo!()
    }
    pub fn complex(&self) -> &Oid {
        todo!()
    }
}

impl<A: Kind, B: Kind> Room<Sum<A, B>> {
    pub fn sum(left: &Room<A>, right: &Room<B>) -> Result<Self, RoomError> {
        todo!()
    }
    pub fn left(&self) -> &Room<A> {
        todo!()
    }
    pub fn right(&self) -> &Room<B> {
        todo!()
    }
}

pub trait Entry: Clone + PartialEq + Debug {
    fn zero() -> Self;
    fn one() -> Self;
    fn checked_add(&self, other: &Self) -> Option<Self>;
    fn checked_mul(&self, other: &Self) -> Option<Self>;
    fn checked_neg(&self) -> Option<Self>;
}

pub trait Measure: Entry + Exact {
    fn measure(&self) -> f64;
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Z(pub i64);

impl crate::field::Group for Z {
    fn identity() -> Self {
        todo!()
    }
    fn compose(&self, other: &Self) -> Self {
        todo!()
    }
    fn inverse(&self) -> Self {
        todo!()
    }
}
impl Exact for Z {}

impl Entry for Z {
    fn zero() -> Self {
        todo!()
    }
    fn one() -> Self {
        todo!()
    }
    fn checked_add(&self, other: &Self) -> Option<Self> {
        todo!()
    }
    fn checked_mul(&self, other: &Self) -> Option<Self> {
        todo!()
    }
    fn checked_neg(&self) -> Option<Self> {
        todo!()
    }
}

impl Measure for Z {
    fn measure(&self) -> f64 {
        todo!()
    }
}

impl Entry for f64 {
    fn zero() -> Self {
        todo!()
    }
    fn one() -> Self {
        todo!()
    }
    fn checked_add(&self, other: &Self) -> Option<Self> {
        todo!()
    }
    fn checked_mul(&self, other: &Self) -> Option<Self> {
        todo!()
    }
    fn checked_neg(&self) -> Option<Self> {
        todo!()
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Map<A: Kind, B: Kind, R> {
    from: Room<A>,
    to: Room<B>,
    entries: Vec<R>,
}

impl<A: Kind, B: Kind, R: Entry> Map<A, B, R> {
    pub fn zero(from: &Room<A>, to: &Room<B>) -> Self {
        todo!()
    }
    pub fn from_rows(from: &Room<A>, to: &Room<B>, rows: &[Vec<R>]) -> Result<Self, RoomError> {
        todo!()
    }
    pub fn from(&self) -> &Room<A> {
        todo!()
    }
    pub fn to(&self) -> &Room<B> {
        todo!()
    }
    pub fn entry(&self, row: usize, col: usize) -> Option<&R> {
        todo!()
    }
    pub fn apply(&self, x: &[R]) -> Result<Vec<R>, RoomError> {
        todo!()
    }
    /// Red: untyped in the middle room, as a flat matrix is.
    pub fn then<B2: Kind, C: Kind>(&self, next: &Map<B2, C, R>) -> Result<Map<A, C, R>, RoomError> {
        todo!()
    }
    pub fn plus(&self, other: &Self) -> Result<Self, RoomError> {
        todo!()
    }
    pub fn negate(&self) -> Result<Self, RoomError> {
        todo!()
    }
    pub fn adjoint(&self) -> Map<B, A, R> {
        todo!()
    }
}

impl<A: Kind, B: Kind, R: Measure> Map<A, B, R> {
    pub fn measure(&self) -> Map<A, B, f64> {
        todo!()
    }
}

impl<A: Kind, R: Entry> Map<A, A, R> {
    pub fn identity(room: &Room<A>) -> Self {
        todo!()
    }
    pub fn symmetric(self) -> Result<Symmetric<A, R>, RoomError> {
        todo!()
    }
}

impl<A: Kind, B: Kind, R: Entry> Map<A, Sum<A, B>, R> {
    pub fn inject_left(room: &Room<Sum<A, B>>) -> Self {
        todo!()
    }
}
impl<A: Kind, B: Kind, R: Entry> Map<B, Sum<A, B>, R> {
    pub fn inject_right(room: &Room<Sum<A, B>>) -> Self {
        todo!()
    }
}
impl<A: Kind, B: Kind, R: Entry> Map<Sum<A, B>, A, R> {
    pub fn project_left(room: &Room<Sum<A, B>>) -> Self {
        todo!()
    }
}
impl<A: Kind, B: Kind, R: Entry> Map<Sum<A, B>, B, R> {
    pub fn project_right(room: &Room<Sum<A, B>>) -> Self {
        todo!()
    }
}
impl<A: Kind, B: Kind, C: Kind, D: Kind, R: Entry> Map<Sum<A, B>, Sum<C, D>, R> {
    pub fn diagonal(f: &Map<A, C, R>, g: &Map<B, D, R>) -> Result<Self, RoomError> {
        todo!()
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Symmetric<A: Kind, R>(Map<A, A, R>);

impl<A: Kind> Symmetric<A, f64> {
    pub fn decompose(&self) -> Result<Modes<A>, RoomError> {
        todo!()
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Modes<A: Kind> {
    room: Room<A>,
    spectrum: Vec<f64>,
    vectors: Vec<f64>,
}

impl<A: Kind> Modes<A> {
    pub fn room(&self) -> &Room<A> {
        todo!()
    }
    pub fn spectrum(&self) -> &[f64] {
        todo!()
    }
    pub fn vector(&self, k: usize) -> Option<&[f64]> {
        todo!()
    }
    pub fn vectors(&self) -> impl Iterator<Item = &[f64]> {
        todo!();
        #[allow(unreachable_code)]
        std::iter::empty()
    }
}

pub fn coboundary(graph: &Graph) -> Map<Vertices, Edges, Z> {
    todo!()
}

pub fn dirac(graph: &Graph) -> Map<Sum<Vertices, Edges>, Sum<Vertices, Edges>, Z> {
    todo!()
}
