//! Which steps commute.

use std::collections::BTreeSet;

use crate::oid::Oid;

/// A symmetric, irreflexive relation on steps.
///
/// Soundness contract: `independent(a, b)` implies that running `a` then `b`
/// and `b` then `a` reach the same state from every reachable state. The
/// relation is part of every trace address, so it carries its own.
pub trait Independence<E>: Default {
    /// Whether `a` and `b` commute.
    fn independent(&self, a: &E, b: &E) -> bool;
    /// Content address of the relation itself.
    fn oid(&self) -> Oid;
}

/// The sites a step reads and writes.
pub trait Touches {
    /// Sites read.
    fn reads(&self) -> &BTreeSet<Oid>;
    /// Sites written.
    fn writes(&self) -> &BTreeSet<Oid>;
}

/// Independent iff distinct and neither writes a site the other touches.
///
/// Declared, not measured: sound exactly as far as the footprints are honest.
/// Hidden shared state (global counters, clocks, RNGs) makes it unsound.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Polyphony;

impl<E: Touches + PartialEq> Independence<E> for Polyphony {
    fn independent(&self, a: &E, b: &E) -> bool {
        let conflicts = |x: &E, y: &E| {
            x.writes()
                .iter()
                .any(|s| y.reads().contains(s) || y.writes().contains(s))
        };
        a != b && !conflicts(a, b) && !conflicts(b, a)
    }

    fn oid(&self) -> Oid {
        Oid::hash(b"prismqueer::time::Polyphony")
    }
}

/// Nothing commutes. Linear time: every trace is a word.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Monophony;

impl<E> Independence<E> for Monophony {
    fn independent(&self, _: &E, _: &E) -> bool {
        false
    }

    fn oid(&self) -> Oid {
        Oid::hash(b"prismqueer::time::Monophony")
    }
}
