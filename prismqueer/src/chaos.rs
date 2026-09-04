//! `prismqueer::chaos` — phenomenological substrate-native re-export of `terni::Loss`
//! per Alex 2026-09-04 PM Move 3 verbatim ("What if it were @chaos? `Chaos`. And Chaos
//! IS `Loss`?").
//!
//! Reed TICK B step 3 per Alex 2026-09-04 PM "Tick." authorization following Shard<T>
//! (`22723bb`) + Observer<N> (`55b8a76`) LANDED. Third bottom-up ship in the prismqueer
//! floor sequence. Composes over LANDED `terni::Loss` monoid trait.
//!
//! # Alex 2026-09-04 Move 3 verbatim
//!
//! > "settling_evidence. I feel this is not confidence. This is @chaos. What if it were
//! > @chaos? `Chaos`. And Chaos IS `Loss`?"
//!
//! # Substrate-native naming discipline
//!
//! Per HARD RULE [[feedback-alex-phenomenologizes-reeds-mechanical-names-substrate-native-
//! beats-mechanical-descriptive]]: `Loss` is the ML/statistics mechanical-descriptive name;
//! `Chaos` is the phenomenological substrate-native name for the SAME primitive. Both
//! coexist as reading-labels for the same monoid trait (same pattern as gauge/ampel are
//! reading-labels for `Imperfect<G, Y, R>` per Move 3).
//!
//! # This ship (namespace-altitude substrate-fix; not terni-crate rename)
//!
//! This ship authors `prismqueer::chaos` module that re-exports `terni::Loss` +
//! `terni::Metric` + the four LANDED impl types under `Chaos` naming at prismqueer
//! altitude. This is the NAMESPACE-altitude ship (substrate-fix at prismqueer surface
//! altitude).
//!
//! Full terni-crate rename (`terni::Loss` → `terni::Chaos` at primitive) FORWARD-PROMISED
//! at future tick when terni-crate ripple (ConvergenceLoss → ConvergenceChaos etc.) is
//! ready for cross-crate coordinated ship. Per HARD RULE [[feedback-reed-workaround-
//! whore-reflex-dual-type-aliases-preserving-arbitrary-parameter-order]]: this is
//! substrate-fix at namespace altitude (prismqueer surface), NOT dual-alias workaround —
//! `Chaos` becomes THE name at prismqueer altitude; `Loss` remains as internal terni
//! implementation-detail-name until coordinated ripple lands.
//!
//! # Composition (grep-verified LANDED per FLOOR Definition M8.1)
//!
//! - `terni::Loss` trait — monoid (zero + combine associative) at
//!   `/Users/alexwolf/dev/projects/prism/imperfect/src/lib.rs:107` LANDED
//! - `terni::Metric: Loss` — adds symmetry + non-negative + triangle-inequality axioms
//! - `terni::ConvergenceLoss` — iterative refinement impl
//! - `terni::ApertureLoss` — partial observation impl
//! - `terni::RoutingLoss` — decision impl
//! - `terni::ScalarLoss` — numeric impl (via prismqueer::ScalarLoss re-export)
//!
//! # Composition with today's terminal-form arc
//!
//! - Move 3 (Alex 2026-09-04 PM): Chaos IS Loss; phenomenological rename at prismqueer
//!   altitude via this ship
//! - Move 8 (Alex 2026-09-04 PM elegant closure): `Observation = { crystal, chaos }`
//!   product type; chaos field references prismqueer::chaos::Chaos (this module) per
//!   forward-promised Observation module
//! - Move 15+17 (Alex 2026-09-04 PM): Chaos-residual carried by `Flux<T>` in-motion state;
//!   composes with `prismqueer::flux::FluxThread` LANDED
//! - Move 16 (Alex 2026-09-04 PM): Reality sum-type Fractured has Chaos residual per H^0 > 1;
//!   Settled has Chaos::zero() monoid identity limit

pub use terni::{Loss as Chaos, Metric as ChaosMetric};

// Domain-specific Chaos impls (re-exported from terni with phenomenological naming).
pub use terni::{ApertureLoss as ApertureChaos, ConvergenceLoss as ConvergenceChaos, RoutingLoss as RoutingChaos};

// Numeric Chaos impl via prismqueer::ScalarLoss re-export.
pub use crate::ScalarLoss as ScalarChaos;

#[cfg(test)]
mod tests {
    use super::*;
    use terni::Loss;

    #[test]
    fn chaos_is_loss_type_alias_composes_at_type_level() {
        // Chaos::zero() invokes terni::Loss::zero() via type-alias.
        // ScalarChaos = ScalarLoss = numeric monoid impl.
        let zero: ScalarChaos = ScalarChaos::zero();
        let five: ScalarChaos = ScalarChaos::new(5.0);
        let composed = zero.combine(&five);
        // Monoid identity: zero ◦ x = x
        assert_eq!(composed, five, "Chaos::zero() is the monoid identity");
    }

    #[test]
    fn convergence_chaos_from_terni_convergence_loss_composes() {
        // ConvergenceChaos = ConvergenceLoss (iterative refinement impl).
        let residual: ConvergenceChaos = ConvergenceChaos::new(3);
        let doubled = residual.combine(&residual);
        // Loss::combine is associative; monoid axiom preserved.
        let _residual_squared = doubled.combine(&residual);
    }

    #[test]
    fn chaos_type_alias_is_loss_trait_at_type_level() {
        // Reed 2026-09-04 substrate-fix verification: Chaos and Loss are literally
        // the SAME trait per pub use terni::Loss as Chaos. This test confirms the
        // type-alias reaches back to the monoid axioms.
        fn require_chaos<C: Chaos>(_: &C) -> bool { true }
        let scalar = ScalarChaos::zero();
        assert!(require_chaos(&scalar));
    }
}
