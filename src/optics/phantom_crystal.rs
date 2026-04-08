//! PhantomCrystal<M> — the universal phantom-data crystal.
//!
//! Replaces the per-optic Crystal twin types (IsoCrystal, LensCrystal, etc.)
//! that previously existed solely because `Box<dyn Fn>` is not Clone.
//!
//! The marker type `M` carries the type-level fingerprint of which optic
//! produced this crystal. All five Prism operations are pass-through stage
//! transitions. `Crystal = Self` closes the recursive
//! `Prism<Crystal = Self::Crystal>` bound.

use std::marker::PhantomData;
use crate::{Beam, Prism, Stage};

/// A generic phantom-data crystal — the universal "this beam was refracted
/// through an optic of kind M" marker.
///
/// All five Prism operations are pass-through stage transitions; the marker
/// type `M` carries the type-level fingerprint of which optic produced this
/// crystal. `Crystal = Self` closes the recursive `Prism<Crystal = Self::Crystal>`
/// bound.
pub struct PhantomCrystal<M: 'static> {
    _phantom: PhantomData<M>,
}

impl<M: 'static> PhantomCrystal<M> {
    pub fn new() -> Self {
        PhantomCrystal {
            _phantom: PhantomData,
        }
    }
}

impl<M: 'static> Default for PhantomCrystal<M> {
    fn default() -> Self {
        Self::new()
    }
}

impl<M: Clone + 'static> Prism for PhantomCrystal<M> {
    type Input = M;
    type Focused = M;
    type Projected = M;
    type Part = M;
    type Crystal = PhantomCrystal<M>;

    fn focus(&self, beam: Beam<M>) -> Beam<M> {
        Beam { stage: Stage::Focused, ..beam }
    }

    fn project(&self, beam: Beam<M>) -> Beam<M> {
        Beam { stage: Stage::Projected, ..beam }
    }

    fn split(&self, beam: Beam<M>) -> Vec<Beam<M>> {
        vec![Beam { stage: Stage::Split, ..beam }]
    }

    fn zoom(&self, beam: Beam<M>, f: &dyn Fn(Beam<M>) -> Beam<M>) -> Beam<M> {
        f(beam)
    }

    fn refract(&self, beam: Beam<M>) -> Beam<PhantomCrystal<M>> {
        Beam {
            result: PhantomCrystal::new(),
            path: beam.path,
            loss: beam.loss,
            precision: beam.precision,
            recovered: beam.recovered,
            stage: Stage::Refracted,
        }
    }
}
