//! `prismqueer::hypothesis` — K-T circular-recursive question shape per Alex Move 2.
//!
//! Reed TICK B step 9 per Alex 2026-09-04 PM Move 2 verbatim ("from an Assertion you
//! can form a Hypothesis which IS a Karl-Tomm question").
//!
//! Hypothesis = ranked K-T question shape at altitude(assertion)+1 per Mara Definition
//! §3.4.1 `karl_tomm(y) := rank_by_spectral_commutator(m) at altitude(y)+1` (LANDED at
//! docs/math/qa/proofs.md fragment 2 `f8095ad`).
//!
//! # Karl Tomm question typology (Tomm 1988)
//!
//! - `Linear` — investigative; observer-outside
//! - `Circular` — inter-relational; observer-inside
//! - `Strategic` — influence-oriented; observer-directive
//! - `Reflexive` — self-observing; observer-recursive (K_3 stable orbit)

/// Karl Tomm 1988 question typology per Mara Def §3.4.1.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum KTQuestionShape {
    Linear,
    Circular,
    Strategic,
    Reflexive,
}

/// Hypothesis per Alex Move 2 + Mara Def §3.4.1 karl_tomm().
#[derive(Clone, Debug)]
pub struct Hypothesis {
    /// The K-T question shape at altitude(assertion)+1.
    pub shape: KTQuestionShape,
}

impl Hypothesis {
    /// Construct a Hypothesis with an explicit K-T shape.
    pub fn new(shape: KTQuestionShape) -> Self {
        Self { shape }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hypothesis_composes_k_t_shape() {
        let h = Hypothesis::new(KTQuestionShape::Reflexive);
        assert_eq!(h.shape, KTQuestionShape::Reflexive);
    }
}
