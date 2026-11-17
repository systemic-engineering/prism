//! `prismqueer::recursion` — Recursion state carrier per Alex Move 8+9.
//!
//! Reed TICK B step 12 per Alex 2026-09-04 PM Move 8 verbatim ("`Recursion` is what
//! passes through the bundle tower and `Chaos` is what emerges") + Move 9 ("Recursion
//! settles through repeated observations through the @void. That's the tick.").
//!
//! # Alex Move 8 elegant closure
//!
//! ```text
//! Recursion.tick() -> Imperfect<Observation, EmptyFracture, Unmeasured>
//! ```
//!
//! One pass through @void 5-axis gauge basis = ONE TICK per Move 9. Settlement is
//! asymptotic (Fiedler λ_2 monotone climb per Rec #92 M2.2 L3).
//!
//! # No silent collapse (prism-019 #1)
//!
//! - **Every shard is kept.** A `Fractured` of n shards ticks into an observation of n
//!   crystals, in order. prism never picks first. If one reading must be chosen, that
//!   choice is a ruler's: supplied by the caller, with no default, and recorded there.
//! - **No fake zero.** tick measures no chaos yet, so it reports none: the observation's
//!   `chaos` is `None` and the outcome's loss is [`Unmeasured`].
//! - **Empty is typed.** An empty fracture has nothing to observe:
//!   `Failure(EmptyFracture, Unmeasured)`, never a panic.
//!
//! # Composition
//!
//! - `prismqueer::reality::Reality<T>` (Discharge #11)
//! - `prismqueer::observation::Observation<T>` (Discharge #9)
//! - `prismqueer::crystal_shard::Crystal<T>` (Discharge #8)
//! - `prismqueer::chaos::Unmeasured` (prism-019 #1)
//! - `prismqueer::shard::Shard<T>` (Discharge #5)

use crate::chaos::Unmeasured;
use crate::observation::Observation;
use crate::reality::Reality;
use terni::Imperfect;

/// An empty fracture: a `Fractured` reality with no shards, so nothing to observe.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct EmptyFracture;

/// What one tick gives: every crystal observed, or a typed empty fracture.
/// The loss is [`Unmeasured`]: tick measures no chaos yet.
pub type Tick<T> = Imperfect<Observation<T>, EmptyFracture, Unmeasured>;

/// Recursion state carrier per Alex Move 8+9.
///
/// The thing that passes through prismqueer::bundle Baez-Schreiber tower + ψ (read across from Anna Jakobs's 2012 thesis)
/// apparatus + Mandelbrot iteration. Iteration bounds → Crystal condenses; leftover
/// harmonic content → Chaos emerges.
#[derive(Clone, Debug)]
pub struct Recursion<T> {
    /// The Reality this Recursion is settling from.
    pub reality: Reality<T>,
}

impl<T: AsRef<[u8]> + Clone> Recursion<T> {
    /// Construct a Recursion from a Reality per Alex Move 8 signature.
    pub fn from_reality(reality: Reality<T>) -> Self {
        Self { reality }
    }

    /// Full 6-arrow subject-driven loop tick per Alex 2026-09-04 PM Move 9 verbatim:
    ///
    /// ```text
    /// Recursion → tick → Observation → assert(Model) → Assertion →
    ///   question_shape → KTQuestionShape → compose(Chaos) → Question →
    ///   [Choice from @subject] → next Recursion
    /// ```
    ///
    /// Returns Question that awaits Choice from @subject. The Choice-response →
    /// next Recursion crossing IS the subject-substrate boundary (external to this
    /// function; non-Vereinnahmung by construction — compiler cannot generate Choice).
    ///
    /// `chaos` is the caller's: tick measures none. An empty fracture gives
    /// `Failure(EmptyFracture, Unmeasured)` and no Question.
    pub fn subject_driven_loop_tick(
        self,
        model: crate::model::Model<T>,
        chaos: crate::chaos::ScalarChaos,
    ) -> Imperfect<crate::question::Question, EmptyFracture, Unmeasured> {
        self.tick()
            .map(|observation| observation.assert(model).question_shape().compose(chaos))
    }

    /// One pass through @void = ONE TICK per Alex Move 9.
    ///
    /// - `Settled(crystal)` → `Success`, an observation of that one crystal.
    /// - `Fractured(shards)`, non-empty → `Success`, an observation of **every** shard,
    ///   in order. None is dropped and none is preferred.
    /// - `Fractured([])` → `Failure(EmptyFracture, Unmeasured)`.
    ///
    /// Chaos is not measured here, so every observation carries `chaos: None`.
    ///
    /// Full implementation composing prismqueer::bundle Baez-Schreiber tower +
    /// ψ (read across from Anna Jakobs's 2012 thesis) + Mandelbrot iteration + Fiedler λ_2 monotone-climb check FORWARD-PROMISED.
    pub fn tick(self) -> Tick<T> {
        match self.reality {
            Reality::Settled(crystal) => Imperfect::Success(Observation::new(vec![crystal], None)),
            Reality::Fractured(shards) if shards.is_empty() => {
                Imperfect::Failure(EmptyFracture, Unmeasured)
            }
            Reality::Fractured(shards) => Imperfect::Success(Observation::new(shards, None)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::chaos::ScalarChaos;
    use crate::crystal_shard::Crystal;
    use crate::shard::Shard;

    #[test]
    fn recursion_tick_from_settled_reality_returns_observation() {
        let reality: Reality<&[u8]> = Reality::Settled(Crystal::new(b"c"));
        let obs = Recursion::from_reality(reality)
            .tick()
            .ok()
            .expect("settled");
        assert_eq!(obs.crystals.len(), 1);
        assert_eq!(obs.crystals[0].payload, b"c");
    }

    // prism-019 #1: a fracture keeps every shard through tick. No first wins.
    #[test]
    fn tick_keeps_every_shard_of_a_three_shard_fracture() {
        let shards: Vec<Shard<&[u8]>> = vec![Shard::new(b"a"), Shard::new(b"b"), Shard::new(b"c")];
        let obs = Recursion::from_reality(Reality::Fractured(shards))
            .tick()
            .ok()
            .expect("a non-empty fracture is observed");
        let payloads: Vec<&[u8]> = obs.crystals.iter().map(|c| c.payload).collect();
        assert_eq!(payloads, vec![b"a" as &[u8], b"b", b"c"]);
    }

    // An empty fracture is a typed outcome, never a panic.
    #[test]
    fn tick_on_an_empty_fracture_is_a_typed_failure() {
        let reality: Reality<&[u8]> = Reality::Fractured(vec![]);
        let outcome = Recursion::from_reality(reality).tick();
        assert_eq!(
            outcome.err_with_loss(),
            Some((EmptyFracture, Unmeasured)),
            "nothing to observe, and nothing measured"
        );
    }

    // tick measures no chaos, so it reports none: never a silent zero.
    #[test]
    fn tick_reports_no_chaos_it_did_not_measure() {
        let fractured: Reality<&[u8]> =
            Reality::Fractured(vec![Shard::new(b"a"), Shard::new(b"b")]);
        let settled: Reality<&[u8]> = Reality::Settled(Crystal::new(b"c"));
        for reality in [fractured, settled] {
            let outcome = Recursion::from_reality(reality).tick();
            assert_eq!(outcome.loss(), Unmeasured);
            let obs = outcome.ok().expect("observed");
            assert_eq!(obs.chaos, None);
        }
    }

    #[test]
    fn subject_driven_loop_tick_composes_full_6_arrow_move_9_pipeline() {
        use crate::model::Model;
        use crate::question::KTQuestionShape;

        let reality: Reality<&[u8]> = Reality::Settled(Crystal::new(b"stable"));
        let recursion = Recursion::from_reality(reality);
        let model: Model<&[u8]> = Model::empty();
        // The chaos is the caller's measurement, passed through untouched.
        let question = recursion
            .subject_driven_loop_tick(model, ScalarChaos::new(0.5))
            .ok()
            .expect("settled reality yields a question");
        // Full Move 9 pipeline discharged: Reality → Observation → Assertion →
        // KTQuestionShape → Question emitted awaiting Choice from @subject.
        assert_eq!(question.shape, KTQuestionShape::Reflexive);
        assert_eq!(question.chaos, ScalarChaos::new(0.5));
    }

    #[test]
    fn subject_driven_loop_tick_on_an_empty_fracture_asks_no_question() {
        use crate::model::Model;

        let reality: Reality<&[u8]> = Reality::Fractured(vec![]);
        let outcome = Recursion::from_reality(reality)
            .subject_driven_loop_tick(Model::empty(), ScalarChaos::new(0.5));
        assert_eq!(outcome.err(), Some(EmptyFracture));
    }
}
