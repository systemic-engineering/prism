//! The measured half of [`field`](super) (`lapack`): a float group, read
//! only through a declared [`Noise`] band, and a field on `spectral::join`'s
//! simple graph. No silent tolerance.

use prism_laws::field::{check_flat_by, EdgeKey, Field, FieldError, Flatness, Graph, Group};

use crate::spectral::join::Graph as Shape;
use crate::spectral::settle::Noise;

/// A group read by measurement: how far a value is from the identity.
pub trait Measured: Group {
    /// Distance from the identity, in the group's own units.
    fn magnitude(&self) -> f64;
}

/// A float group: logarithms (base 2) under addition. A measurement, not
/// [`Exact`](super::Exact): read it through [`check_flat_within`] with a declared band.
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

/// Flat or curved, read through a declared band: a loop closes when its
/// holonomy is within `noise` of the identity.
pub fn check_flat_within<V: Measured>(field: &Field<Graph, V>, noise: Noise) -> Flatness<V> {
    check_flat_by(field, |h| h.magnitude() <= noise.0)
}

/// A connection on [`spectral::join`](crate::spectral::join)'s simple graph:
/// [`Field::simple`] on its multigraph.
pub trait Transports<V: Group>: Sized {
    /// One transport per edge, given in either orientation (a transport
    /// given on `(v, u)` is stored inverted on `(u, v)`). Every edge is
    /// `nth` 0; a pair given twice keeps the last. For parallel edges, use
    /// [`Field::multigraph`].
    fn transports(
        graph: &Shape,
        given: impl IntoIterator<Item = (EdgeKey, V)>,
    ) -> Result<Self, FieldError>;
}

impl<V: Group> Transports<V> for Field<Graph, V> {
    fn transports(
        graph: &Shape,
        given: impl IntoIterator<Item = (EdgeKey, V)>,
    ) -> Result<Self, FieldError> {
        Field::simple(graph.multigraph(), given)
    }
}
