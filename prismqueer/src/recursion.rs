//! `prismqueer::recursion` — Recursion state carrier per Alex Move 8+9.
//!
//! Reed TICK B step 12 per Alex 2026-09-04 PM Move 8 verbatim ("`Recursion` is what
//! passes through the bundle tower and `Chaos` is what emerges") + Move 9 ("Recursion
//! settles through repeated observations through the @void. That's the tick.").
//!
//! # Alex Move 8 elegant closure
//!
//! ```text
//! Recursion.tick::<N>() -> Observation
//! ```
//!
//! One pass through @void 5-axis gauge basis = ONE TICK per Move 9. Settlement is
//! asymptotic (Fiedler λ_2 monotone climb per Rec #92 M2.2 L3).
//!
//! # Composition
//!
//! - `prismqueer::reality::Reality<T>` (Discharge #11)
//! - `prismqueer::observation::Observation<T>` (Discharge #9)
//! - `prismqueer::crystal_shard::Crystal<T>` (Discharge #8)
//! - `prismqueer::chaos::ScalarChaos` (Discharge #7)
//! - `prismqueer::shard::Shard<T>` (Discharge #5)

use crate::chaos::ScalarChaos;
use crate::crystal_shard::Crystal;
use crate::observation::Observation;
use crate::reality::Reality;
use crate::shard::Shard;
use terni::Loss;

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
    ///   hypothesize → Hypothesis → compose(Chaos) → Question →
    ///   [Choice from @subject] → next Recursion
    /// ```
    ///
    /// Returns Question that awaits Choice from @subject. The Choice-response →
    /// next Recursion crossing IS the subject-substrate boundary (external to this
    /// function; non-Vereinnahmung by construction — compiler cannot generate Choice).
    pub fn subject_driven_loop_tick(
        self,
        model: crate::model::Model<T>,
        chaos: crate::chaos::ScalarChaos,
    ) -> crate::question::Question {
        self.tick().assert(model).hypothesize().compose(chaos)
    }

    /// One pass through @void = ONE TICK per Alex Move 9.
    ///
    /// Minimum-viable: extracts a Crystal from the Reality (Settled variant) OR
    /// crystallizes the first Fractured shard. Chaos defaults to zero (asymptotic
    /// settled limit).
    ///
    /// Full implementation composing prismqueer::bundle Baez-Schreiber tower +
    /// ψ (read across from Anna Jakobs's 2012 thesis) + Mandelbrot iteration + Fiedler λ_2 monotone-climb check FORWARD-PROMISED.
    pub fn tick(self) -> Observation<T> {
        match self.reality {
            Reality::Settled(crystal) => Observation::new(crystal, ScalarChaos::zero()),
            Reality::Fractured(mut shards) => {
                let crystal: Crystal<T> = shards.drain(..).next().unwrap_or_else(|| {
                    Shard::from_parts(
                        // Placeholder Shard for empty Fractured; iteration_index=0
                        // signals "no observation possible" per substrate-honest.
                        panic_helper_empty::<T>(),
                        crate::oid::Oid::dark(),
                        terni::Transparency::Clear,
                        0,
                    )
                });
                Observation::new(crystal, ScalarChaos::zero())
            }
        }
    }
}

fn panic_helper_empty<T>() -> T {
    unimplemented!(
        "empty Fractured cannot produce a Crystal without a payload; caller must ensure non-empty"
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::chaos::Unmeasured;

    #[test]
    fn recursion_tick_from_settled_reality_returns_observation() {
        let reality: Reality<&[u8]> = Reality::Settled(Crystal::new(b"c"));
        let recursion = Recursion::from_reality(reality);
        let _obs = recursion.tick();
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
        use crate::hypothesis::KTQuestionShape;
        use crate::model::Model;
        use terni::Loss;

        let reality: Reality<&[u8]> = Reality::Settled(Crystal::new(b"stable"));
        let recursion = Recursion::from_reality(reality);
        let model: Model<&[u8]> = Model::empty();
        let question = recursion.subject_driven_loop_tick(model, ScalarChaos::zero());
        // Full Move 9 pipeline discharged: Reality → Observation → Assertion →
        // Hypothesis → Question emitted awaiting Choice from @subject.
        assert_eq!(question.hypothesis.shape, KTQuestionShape::Reflexive);
    }
}
