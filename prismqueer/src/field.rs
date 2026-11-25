//! `field`: a law of prism's floor, re-exported at its old path, with the
//! lab's pieces on top. Groups, lattices, graphs, faces and flatness live in
//! `prism_laws::field` (docs/spec/laws.md).
//!
//! The lab's: [`FieldPrism`] and [`ScalarPrism`] (prisms that settle into
//! fields, over the lab's `Prism`), and the measured half (`lapack`): a
//! float group read through a declared band, and `Field::transports` on
//! `spectral::join`'s simple graph ([`Transports`]).

pub use prism_laws::field::*;

// The measured half (`lapack`): a float group read through a declared band.
#[cfg(feature = "lapack")]
mod reading;
#[cfg(feature = "lapack")]
pub use reading::{check_flat_within, Log2, Measured, Transports};

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
