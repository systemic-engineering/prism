//! Liquid — property verdicts over the spectral commutator.
//!
//! Composes over prismqueer's Bundle tower (`Transport` supertrait chain
//! `Fiber → Connection → Gauge → Transport`) and terni's verdict machinery
//! (`PropertyVerdict`, `Loss`, `Metric`, `Diagnostic`). Zero new deps.
//!
//! # The commutator, substrate-honestly
//!
//! For a Bundle with `Gauge<Group: GroupStructure>` acting on
//! `Fiber::State`, the commutator `[A, B]` of two Bundle instances
//! measures the non-commutativity of their combined gauge action on the
//! state, projected through the Transport's Holonomy metric:
//!
//! ```text
//! [A, B] · state := A.act_on(B.act_on(state)) - B.act_on(A.act_on(state))
//! ‖[A, B]‖      := transport(A·B·state).loss()
//!                    .distance_to(&transport(B·A·state).loss())
//! ```
//!
//! For abelian `Gauge` groups (e.g., `Cyclic<N>`), `[A, B]` vanishes:
//! `A·B·state == B·A·state`, so the two holonomies match, so the Metric
//! distance is `Loss::zero()`. For non-abelian groups, the commutator
//! carries the anisotropy.
//!
//! This is the substrate-honest realization of Connes' bounded-commutator
//! condition `‖[D, a]‖ < ∞` at the Rust-altitude prism-bundle altitude.
//! Full derivation: `mirror/docs/math/spectral-commutator-four-pillars.md`
//! (Mara `5d3040d`) §2; operational spec:
//! `mirror/docs/specs/spectral-commutator-as-cybernetic-ground.md`
//! (Mara `3cd9a42`).
//!
//! # Property guarantees
//!
//! By construction (inherited from `Metric` axioms):
//!
//! - **Antisymmetric**: `commutator_magnitude(a, b, s) ==
//!   commutator_magnitude(b, a, s)` because `Metric::distance_to` is
//!   symmetric per axiom.
//! - **Self-annihilating**: `commutator_magnitude(a, a, s)` is
//!   `Loss::zero()` because `A·A·s == A·A·s`, so the two holonomies are
//!   identical, so their distance is zero.
//! - **Non-negative**: `Metric::is_non_negative` guarantees this.
//! - **Triangle inequality**: `Metric::triangle` guarantees this.
//! - **Vanishes for abelian gauges**: `Cyclic<N>` action commutes.
//!
//! Every one of these is empirically witnessed by `prismqueer/tests/
//! liquid_ouroboros.rs` — the first ouroboros layer where prismqueer
//! tests its own trait laws through its own liquid module.

// `Transport` is a supertrait of `Gauge`, so importing `Transport` alone
// is sufficient for the trait-solver to reach `act_on` via the supertrait chain.
use crate::bundle::Transport;
use terni::{Diagnostic, Loss, Metric, PropertyVerdict};

// ──────────────────────────────────────────────────────────────────
// LiquidConnection
// ──────────────────────────────────────────────────────────────────

/// A Bundle whose commutator can be computed at Rust altitude via the
/// composition of Gauge action + Transport holonomy.
///
/// Blanket-implemented for any type that satisfies `Transport` (whose
/// supertraits `Fiber`, `Connection`, `Gauge` are automatically
/// satisfied). Users do NOT implement this trait directly — implementing
/// `Transport` grants LiquidConnection for free.
pub trait LiquidConnection: Transport
where
    Self::Optic: crate::Prism,
    <<Self::Optic as crate::Prism>::Input as crate::Beam>::In: Sized,
{
    /// Compute the commutator magnitude `‖[A, B]‖` at a given state.
    ///
    /// See module-level docs for the mathematical grounding. Returns
    /// the `Transport::Holonomy` (a `Metric`), NOT `f64`, so callers
    /// keep type information about their loss carrier.
    fn commutator_magnitude(a: &Self, b: &Self, state: &Self::State) -> Self::Holonomy;
}

impl<T> LiquidConnection for T
where
    T: Transport,
    T::Optic: crate::Prism,
    <<T::Optic as crate::Prism>::Input as crate::Beam>::In: Sized,
{
    fn commutator_magnitude(a: &Self, b: &Self, state: &Self::State) -> Self::Holonomy {
        // 1. Apply gauge B, then gauge A: state → B·state → A·(B·state)
        let b_state = b.act_on(state);
        let ab_state = a.act_on(&b_state);

        // 2. Apply gauge A, then gauge B: state → A·state → B·(A·state)
        let a_state = a.act_on(state);
        let ba_state = b.act_on(&a_state);

        // 3. Transport each to extract Holonomy loss.
        let ab_holonomy = a.transport(&ab_state).loss();
        let ba_holonomy = b.transport(&ba_state).loss();

        // 4. Metric distance is the commutator magnitude.
        //    Guaranteed symmetric (antisymmetry of underlying [A,B]),
        //    non-negative, self-annihilating, triangle-inequal by the
        //    Metric trait's axioms.
        ab_holonomy.distance_to(&ba_holonomy)
    }
}

// ──────────────────────────────────────────────────────────────────
// Commutator — held-reference pair, deferred magnitude computation.
// ──────────────────────────────────────────────────────────────────

/// The commutator `[A, B]` at a state as a deferred value.
///
/// Holds references to the two connections and the state. Computes the
/// magnitude via `LiquidConnection::commutator_magnitude` on demand.
pub struct Commutator<'a, C: LiquidConnection>
where
    C::Optic: crate::Prism,
    <<C::Optic as crate::Prism>::Input as crate::Beam>::In: Sized,
{
    a: &'a C,
    b: &'a C,
    state: &'a C::State,
}

impl<'a, C: LiquidConnection> Commutator<'a, C>
where
    C::Optic: crate::Prism,
    <<C::Optic as crate::Prism>::Input as crate::Beam>::In: Sized,
{
    /// Compute the commutator magnitude.
    pub fn magnitude(&self) -> C::Holonomy {
        C::commutator_magnitude(self.a, self.b, self.state)
    }
}

/// Construct a commutator of two connections at a specified state.
pub fn commutator<'a, C: LiquidConnection>(
    a: &'a C,
    b: &'a C,
    state: &'a C::State,
) -> Commutator<'a, C>
where
    C::Optic: crate::Prism,
    <<C::Optic as crate::Prism>::Input as crate::Beam>::In: Sized,
{
    Commutator { a, b, state }
}

/// Compute the commutator norm at the `Default` state.
///
/// Convenience for tests where the caller doesn't need to control the
/// state. Requires `C::State: Default` because we synthesize a canonical
/// state. For non-Default states, use `commutator(...)` with an explicit
/// state, or call `LiquidConnection::commutator_magnitude` directly.
pub fn commutator_norm<C>(a: &C, b: &C) -> C::Holonomy
where
    C: LiquidConnection,
    C::State: Default,
    C::Optic: crate::Prism,
    <<C::Optic as crate::Prism>::Input as crate::Beam>::In: Sized,
{
    let state = C::State::default();
    C::commutator_magnitude(a, b, &state)
}

// ──────────────────────────────────────────────────────────────────
// Pillar verdicts (Mara `5d3040d` §2 four-pillar structure).
// ──────────────────────────────────────────────────────────────────

/// The four pillars from the grounding spec, exposed as verdict
/// functions. Each returns a `terni::PropertyVerdict`.
///
/// Pillar IV (`@peer.audhd` fanout) lives at `mirror/rust/src/liquid.rs`
/// because it needs `fate::Fate::tick`, which prismqueer doesn't and
/// shouldn't depend on. See Mara `3cd9a42` §6.
pub mod pillar {
    use super::*;

    /// **Pillar I — dispatch ambiguity.** Rice-safe byte-visible checks.
    ///
    /// Per `mirror/docs/specs/spectral-commutator-as-cybernetic-ground.md`
    /// §3 + `mirror/shards/kintsugi/surface.mirror` `dispatch_ambiguity`
    /// variant:
    ///
    /// - Pass iff `arm_count >= 2`
    ///   **AND** `witness_count == arm_count`
    ///   **AND** `tie_breaking_exhausted`
    ///   **AND** `pivot_song_present`.
    /// - Fail otherwise, with a Diagnostic naming which byte-visible
    ///   check failed.
    ///
    /// Rice-safe binary: Pass or Fail only. No Partial. No threshold.
    /// Composes over four simple `bool`/`usize` checks so callers can
    /// use this without importing the whole Bundle tower.
    pub fn dispatch_ambiguity(
        arm_count: usize,
        witness_count: usize,
        tie_breaking_exhausted: bool,
        pivot_song_present: bool,
    ) -> PropertyVerdict {
        if arm_count < 2 {
            return PropertyVerdict::Fail(Diagnostic::new(
                "dispatch_ambiguity requires >= 2 admissible arms",
            ));
        }
        if witness_count != arm_count {
            return PropertyVerdict::Fail(Diagnostic::new(
                "witness count must match arm count",
            ));
        }
        if !tie_breaking_exhausted {
            return PropertyVerdict::Fail(Diagnostic::new(
                "tie-breaking not exhausted; not Path-B admissible",
            ));
        }
        if !pivot_song_present {
            return PropertyVerdict::Fail(Diagnostic::new(
                "pivot_song handle missing",
            ));
        }
        PropertyVerdict::Pass
    }

    /// **Pillar II — algedonic threshold.**
    ///
    /// Per Mara `3cd9a42` §4:
    ///
    /// - Pass when `‖[A, B]‖ > theta`.
    /// - Fail when `‖[A, B]‖ == Loss::zero()` (no signal).
    /// - Partial otherwise — signal exists but below threshold.
    ///
    /// Requires `C::Holonomy: PartialOrd` because the pillar compares
    /// magnitude against `theta`. `ScalarLoss` satisfies this.
    pub fn algedonic<'a, C>(
        commutator: &Commutator<'a, C>,
        theta: &C::Holonomy,
    ) -> PropertyVerdict
    where
        C: LiquidConnection,
        C::Holonomy: PartialOrd,
        C::Optic: crate::Prism,
        <<C::Optic as crate::Prism>::Input as crate::Beam>::In: Sized,
    {
        let m = commutator.magnitude();
        if &m > theta {
            PropertyVerdict::Pass
        } else if m.is_zero() {
            PropertyVerdict::Fail(Diagnostic::new(
                "commutator vanished; no algedonic signal",
            ))
        } else {
            // Partial: got a signal but below threshold. Confidence 0.5
            // is a Rice-safe midpoint; consumers with domain-specific
            // Holonomy types can implement a tighter Partial verdict.
            PropertyVerdict::Partial {
                confidence: 0.5,
                diagnostics: vec![Diagnostic::new(
                    "algedonic signal present but below threshold",
                )],
            }
        }
    }

    /// **Pillar III — viability persistence, generalized.**
    ///
    /// Accumulate raw `Loss` magnitudes over a temporal window
    /// `omega` via `Loss::combine`. Pass iff the accumulated `Loss`
    /// exceeds `theta`.
    ///
    /// This is the shape of Pillar III when the magnitudes come from
    /// *substrate-specific* measurements — e.g. byte-shrinkage per
    /// compilation tick from `mirror/rust/src/collapse.rs`, or
    /// `rust_loc_non_increasing` deltas from
    /// `@epistemologic/property/ouroboros_monotone` — rather than
    /// commutator computations. See [`viability`] for the
    /// commutator-flavored variant that takes
    /// `&[Commutator<'a, C>]`.
    ///
    /// - Pass when accumulated `> theta`.
    /// - Partial when `history.len() < omega`
    ///   (`confidence = history.len() / omega`).
    /// - Fail when the window is full but accumulated `<= theta`.
    pub fn viability_of_magnitudes<L>(
        history: &[L],
        theta: &L,
        omega: usize,
    ) -> PropertyVerdict
    where
        L: Loss + PartialOrd,
    {
        if history.len() < omega {
            return PropertyVerdict::Partial {
                confidence: history.len() as f64 / omega.max(1) as f64,
                diagnostics: vec![Diagnostic::new(
                    "history shorter than viability window",
                )],
            };
        }

        let window = &history[history.len() - omega..];
        let mut accumulated = L::zero();
        for m in window {
            accumulated = accumulated.combine(m.clone());
        }

        if &accumulated > theta {
            PropertyVerdict::Pass
        } else {
            PropertyVerdict::Fail(Diagnostic::new(
                "viability persistence below threshold over window",
            ))
        }
    }

    /// **Pillar III — viability persistence.**
    ///
    /// Per Mara `3cd9a42` §5: sum the commutator magnitudes across a
    /// temporal window `omega` (the tail of `history`) via
    /// `Loss::combine`. Pass iff the accumulated magnitude exceeds
    /// `theta_s3s4`.
    ///
    /// - Pass when accumulated `> theta_s3s4`.
    /// - Partial when history shorter than window (insufficient data;
    ///   `confidence = history.len() / omega`).
    /// - Fail when window is full but accumulated below threshold.
    pub fn viability<'a, C>(
        history: &[Commutator<'a, C>],
        theta_s3s4: &C::Holonomy,
        omega: usize,
    ) -> PropertyVerdict
    where
        C: LiquidConnection,
        C::Holonomy: PartialOrd,
        C::Optic: crate::Prism,
        <<C::Optic as crate::Prism>::Input as crate::Beam>::In: Sized,
    {
        if history.len() < omega {
            return PropertyVerdict::Partial {
                confidence: history.len() as f64 / omega.max(1) as f64,
                diagnostics: vec![Diagnostic::new(
                    "history shorter than viability window",
                )],
            };
        }

        let window = &history[history.len() - omega..];
        let mut accumulated = C::Holonomy::zero();
        for c in window {
            accumulated = accumulated.combine(c.magnitude());
        }

        if &accumulated > theta_s3s4 {
            PropertyVerdict::Pass
        } else {
            PropertyVerdict::Fail(Diagnostic::new(
                "viability persistence below threshold over window",
            ))
        }
    }
}

// ──────────────────────────────────────────────────────────────────
// Prelude — the delightful use-line.
// ──────────────────────────────────────────────────────────────────

/// `use prismqueer::liquid::prelude::*;` — imports the surface consumers
/// need most often: commutator constructors, the `pillar` module, and
/// terni's verdict types.
pub mod prelude {
    pub use super::pillar;
    pub use super::{commutator, commutator_norm, Commutator, LiquidConnection};
    pub use terni::{Diagnostic, PropertyVerdict};
}
