//! `prismqueer::observation` — Observation product type per Alex Move 8 elegant closure.
//!
//! Reed TICK B step 5 per Alex 2026-09-04 PM Move 8 verbatim ("The `Recursion` settles
//! into a `Crystal` and leftover `Chaos`. Which IS an `Observation`.").
//!
//! # Alex Move 8 shape
//!
//! ```text
//! Observation = { crystals: [Crystal], chaos: Chaos? }
//! ```
//!
//! Straight product type. No verdict-enum. No branching. No duplication. Move 6b+7+8
//! reflex-cascade fires all dissolved via this shape per Alex Move 8 elegant closure.
//!
//! # Every shard, and no unmeasured chaos (prism-019 #1)
//!
//! - `crystals` holds **every** crystal the tick saw, in the reality's order. A
//!   fracture of three shards is an observation of three. prism never picks one;
//!   choosing a single reading is a ruler's act, supplied and signed by the caller.
//! - `chaos` is `None` when no chaos was measured. A `ScalarChaos` here is a
//!   measurement, never a default.
//!
//! # Composition (grep-verified LANDED)
//!
//! - `prismqueer::crystal_shard::Crystal<T>` — settled Shard (Discharge #8)
//! - `prismqueer::chaos::ScalarChaos` — residual per Move 3 (Discharge #7 `1350d60`)

use crate::chaos::ScalarChaos;
use crate::crystal_shard::Crystal;

/// Observation product type per Alex Move 8 elegant closure.
///
/// `Recursion.tick(reality) -> Observation` per Move 8 pipeline.
/// Crystal condenses; Chaos emerges as leftover harmonic residual.
#[derive(Clone, Debug)]
pub struct Observation<T> {
    /// Every Crystal deposited by this observation-tick, in the reality's order.
    /// None is dropped.
    pub crystals: Vec<Crystal<T>>,
    /// The leftover Chaos residual per Move 3+8, when it was measured.
    /// `None`: not measured (never a silent zero).
    pub chaos: Option<ScalarChaos>,
}

impl<T> Observation<T> {
    /// Construct an Observation from its crystals and, if measured, its chaos.
    pub fn new(crystals: Vec<Crystal<T>>, chaos: Option<ScalarChaos>) -> Self {
        Self { crystals, chaos }
    }

    /// Compose Observation with Model → Assertion per Alex Move 2 pipeline.
    ///
    /// > "from an Observation you can form an Assertion (combining the observer's
    /// > `Model` of reality, or any `Model`, with the Observation)"
    ///
    /// Hawking model-dependent-reality EXPLICIT: Assertion carries BOTH observation
    /// AND model through which the observation was made.
    pub fn assert(self, model: crate::model::Model<T>) -> crate::assertion::Assertion<T> {
        crate::assertion::Assertion::new(self, model)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::Model;

    #[test]
    fn observation_product_type_composes_crystals_and_measured_chaos() {
        let crystal: Crystal<&[u8]> = Crystal::new(b"settled");
        let obs = Observation::new(vec![crystal], Some(ScalarChaos::new(1.5)));
        assert_eq!(obs.crystals[0].iteration_index, 0);
        assert_eq!(obs.chaos, Some(ScalarChaos::new(1.5)));
    }

    #[test]
    fn observation_assert_model_composes_to_assertion_per_alex_move_2() {
        let obs: Observation<&[u8]> = Observation::new(vec![Crystal::new(b"c")], None);
        let model: Model<&[u8]> = Model::empty();
        let assertion = obs.assert(model);
        assert_eq!(assertion.model.shards.len(), 0);
    }
}
