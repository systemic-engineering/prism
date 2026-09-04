//! `prismqueer::assertion` — Assertion { observation, model } per Alex Move 2 pipeline.
//!
//! Reed TICK B step 7 per Alex 2026-09-04 PM Move 2 verbatim ("from an Observation you
//! can form an Assertion (combining the observer's `Model` of reality, or any `Model`,
//! with the Observation)").
//!
//! # Composition
//!
//! - `prismqueer::observation::Observation<T>` (Discharge #9)
//! - `prismqueer::model::Model<T>` (Discharge #10)

use crate::model::Model;
use crate::observation::Observation;

/// Assertion = Observation composed with Model per Alex Move 2 pipeline.
///
/// `Observation.assert(Model) -> Assertion` per Move 8 pipeline. Hawking model-dependent-
/// reality EXPLICIT at type-level: Assertion carries BOTH the observation AND the model
/// through which the observation was made.
#[derive(Clone, Debug)]
pub struct Assertion<T> {
    /// The Observation this assertion is grounded in.
    pub observation: Observation<T>,
    /// The Model through which the observation was made (Hawking model-dependent-reality).
    pub model: Model<T>,
}

impl<T> Assertion<T> {
    /// Construct an Assertion from Observation + Model parts.
    pub fn new(observation: Observation<T>, model: Model<T>) -> Self {
        Self { observation, model }
    }

    /// Compose Assertion → Hypothesis per Alex Move 2 pipeline.
    ///
    /// > "from an Assertion you can form a Hypothesis which IS a Karl-Tomm question"
    ///
    /// Per Mara Def §3.4.1 karl_tomm(y) := rank_by_spectral_commutator(m) at altitude(y)+1.
    /// Minimum-viable: defaults to Reflexive K-T shape (observer-recursive; K_3 stable
    /// orbit); full spectral-commutator-ranking FORWARD-PROMISED.
    pub fn hypothesize(self) -> crate::hypothesis::Hypothesis {
        crate::hypothesis::Hypothesis::new(crate::hypothesis::KTQuestionShape::Reflexive)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::chaos::ScalarChaos;
    use crate::crystal_shard::Crystal;
    use terni::Loss;

    #[test]
    fn assertion_composes_observation_and_model() {
        let obs: Observation<&[u8]> = Observation::new(Crystal::new(b"c"), ScalarChaos::zero());
        let model: Model<&[u8]> = Model::empty();
        let assertion = Assertion::new(obs, model);
        assert_eq!(assertion.model.shards.len(), 0);
    }

    #[test]
    fn assertion_hypothesize_composes_to_hypothesis_per_alex_move_2() {
        use crate::hypothesis::KTQuestionShape;
        let obs: Observation<&[u8]> = Observation::new(Crystal::new(b"c"), ScalarChaos::zero());
        let model: Model<&[u8]> = Model::empty();
        let hypothesis = Assertion::new(obs, model).hypothesize();
        assert_eq!(hypothesis.shape, KTQuestionShape::Reflexive);
    }
}
