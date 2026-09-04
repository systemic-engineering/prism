//! `prismqueer::observation` — Observation product type per Alex Move 8 elegant closure.
//!
//! Reed TICK B step 5 per Alex 2026-09-04 PM Move 8 verbatim ("The `Recursion` settles
//! into a `Crystal` and leftover `Chaos`. Which IS an `Observation`.").
//!
//! # Alex Move 8 shape
//!
//! ```text
//! Observation = { crystal: Crystal, chaos: Chaos }
//! ```
//!
//! Straight product type. No verdict-enum. No branching. No duplication. Move 6b+7+8
//! reflex-cascade fires all dissolved via this shape per Alex Move 8 elegant closure.
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
    /// The settled Crystal deposited by this observation-tick.
    pub crystal: Crystal<T>,
    /// The leftover Chaos residual per Move 3+8 (encoded as ScalarChaos monoid).
    pub chaos: ScalarChaos,
}

impl<T> Observation<T> {
    /// Construct an Observation from Crystal + Chaos parts.
    pub fn new(crystal: Crystal<T>, chaos: ScalarChaos) -> Self {
        Self { crystal, chaos }
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
    use terni::Loss;

    #[test]
    fn observation_product_type_composes_crystal_and_chaos() {
        let crystal: Crystal<&[u8]> = Crystal::new(b"settled");
        let chaos = ScalarChaos::zero();
        let obs = Observation::new(crystal, chaos);
        assert_eq!(obs.crystal.iteration_index, 0);
    }

    #[test]
    fn observation_assert_model_composes_to_assertion_per_alex_move_2() {
        let obs: Observation<&[u8]> = Observation::new(Crystal::new(b"c"), ScalarChaos::zero());
        let model: Model<&[u8]> = Model::empty();
        let assertion = obs.assert(model);
        assert_eq!(assertion.model.shards.len(), 0);
    }
}
