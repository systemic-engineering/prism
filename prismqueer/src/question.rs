//! `prismqueer::question` — Question = Hypothesis.compose(Chaos) per Alex Move 3+9.
//!
//! Reed TICK B step 10 per Alex 2026-09-04 PM Move 3 verbatim ("the `Chaos` + `Hypothesis`
//! is what we can turn into a `Question`") + Move 9 (Question is what the substrate emits;
//! subject inputs Choice in response).
//!
//! # Composition
//!
//! - `prismqueer::hypothesis::Hypothesis` (Discharge #12)
//! - `prismqueer::chaos::ScalarChaos` (Discharge #7 `1350d60`)

use crate::chaos::ScalarChaos;
use crate::hypothesis::Hypothesis;

/// Question per Alex Move 3+9 pipeline.
///
/// Emitted by the substrate; @subject inputs Choice in response.
/// Prioritized by Chaos-residual per Move 3 verbatim.
#[derive(Clone, Debug)]
pub struct Question {
    /// The K-T question shape carrier from the Hypothesis.
    pub hypothesis: Hypothesis,
    /// The Chaos-residual that prioritizes this concrete question.
    pub chaos: ScalarChaos,
}

impl Question {
    /// Construct a Question from Hypothesis + Chaos composition.
    pub fn new(hypothesis: Hypothesis, chaos: ScalarChaos) -> Self {
        Self { hypothesis, chaos }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::hypothesis::KTQuestionShape;
    use terni::Loss;

    #[test]
    fn question_composes_hypothesis_and_chaos() {
        let h = Hypothesis::new(KTQuestionShape::Reflexive);
        let q = Question::new(h, ScalarChaos::zero());
        assert_eq!(q.hypothesis.shape, KTQuestionShape::Reflexive);
    }
}
