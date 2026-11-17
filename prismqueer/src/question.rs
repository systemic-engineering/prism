//! `prismqueer::question` — Question = KTQuestionShape.compose(Chaos) per Alex Move 3+9.
//!
//! Reed TICK B step 10 per Alex 2026-09-04 PM Move 3 + Move 9 (Question is what the
//! substrate emits; subject inputs Choice in response). The shape was carried by a
//! constant `Hypothesis`; that name now belongs to [`crate::hypothesis`] (prism-021).
//!
//! # Composition
//!
//! - [`KTQuestionShape`]
//! - `prismqueer::chaos::ScalarChaos` (Discharge #7 `1350d60`)
//!
//! # Karl Tomm question typology (Tomm 1988)
//!
//! - `Linear` — investigative; observer-outside
//! - `Circular` — inter-relational; observer-inside
//! - `Strategic` — influence-oriented; observer-directive
//! - `Reflexive` — self-observing; observer-recursive (K_3 stable orbit)

use crate::chaos::ScalarChaos;

/// Karl Tomm 1988 question typology per Mara Def §3.4.1.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum KTQuestionShape {
    Linear,
    Circular,
    Strategic,
    Reflexive,
}

impl KTQuestionShape {
    /// Compose the shape + Chaos → Question per Alex Move 3 pipeline.
    pub fn compose(self, chaos: ScalarChaos) -> Question {
        Question::new(self, chaos)
    }
}

/// Question per Alex Move 3+9 pipeline.
///
/// Emitted by the substrate; @subject inputs Choice in response.
/// Prioritized by Chaos-residual per Move 3 verbatim.
#[derive(Clone, Debug)]
pub struct Question {
    /// The K-T question shape.
    pub shape: KTQuestionShape,
    /// The Chaos-residual that prioritizes this concrete question.
    pub chaos: ScalarChaos,
}

impl Question {
    /// Construct a Question from a shape + Chaos composition.
    pub fn new(shape: KTQuestionShape, chaos: ScalarChaos) -> Self {
        Self { shape, chaos }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use terni::Loss;

    #[test]
    fn question_composes_shape_and_chaos() {
        let q = KTQuestionShape::Reflexive.compose(ScalarChaos::zero());
        assert_eq!(q.shape, KTQuestionShape::Reflexive);
    }
}
