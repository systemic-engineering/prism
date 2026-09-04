//! `prismqueer::observer` — the Observer<const N: usize> primitive per Alex 2026-09-04
//! PM Move 1 verbatim ("What if we had a prismqueer::observer that's parametrized on
//! the dimensions? A tape would be Observer<1>").
//!
//! Reed TICK B step 2 per Alex 2026-09-04 PM authorization ("Tick.") following Shard<T>
//! LANDED at commit `22723bb`. Second bottom-up ship in the prismqueer floor sequence.
//!
//! # Alex 2026-09-04 Move 1 verbatim
//!
//! > "What if we had a prismqueer::observer that's parametrized on the dimensions?
//! > A tape would be Observer<1>"
//!
//! # Alex 2026-09-04 Move 12 (via LinkedIn to Jimmy Roberts)
//!
//! > "any entity that intervenes in the cognitive field through their presence in said
//! > field, by acting from their own subjective observation of reality"
//!
//! # Alex 2026-09-04 Move 13 five-op linearity split
//!
//! - `Observer<1>` = Turing tape (degenerate; K_{1,n-1} pole per PAPER_2D §1.1.5;
//!   observer-stripped; single-linear-dim; four-property LOVE violation)
//! - `Observer<3>` = K_3 peer stability (past+now+future simultaneous per Rec #99 orbital
//!   topology; @reality/object observation-sufficient at 3D linear)
//! - `Observer<5>` = K_5 @void gauge basis (5-axis void-duality per Rec #79 gauge-dim-of-5;
//!   @reality/subject + @reality/hodobodo require this 5D observation)
//! - `Observer<N>` general = N-dim decomposition matching observed-geometry dimensionality
//!
//! # This ship (minimum viable scaffold composing over LANDED)
//!
//! Observer<N> at type-level with const-generic N encoding observation-dimensionality.
//! `observe_payload(T) -> Shard<T>` composes over LANDED `prismqueer::shard::Shard<T>`
//! (Discharge #5 `22723bb`) which itself content-addresses via `Oid::hash` via
//! `CoincidenceHash<3>` at prismqueer::coincidence LANDED.
//!
//! Full `Observer<N>::observe(Reality) -> Observation` per Move 2+8 pipeline signature
//! FORWARD-PROMISED when `prismqueer::reality::Reality` + `prismqueer::observation::Observation`
//! land. Recursion tick per Move 8 composes over this Observer<N>.
//!
//! # Composition (grep-verified LANDED per FLOOR Definition M8.1)
//!
//! - `prismqueer::shard::Shard<T>` — substrate carrier (Reed Discharge #5 `22723bb`)
//! - `prismqueer::coincidence::Detector<N>` — N-projection observation apparatus at prism-repo
//!   (currently private detect method; observe_payload composes via Shard::new which uses
//!   Oid::hash internally; future ticks may expose Detector<N> observation directly)
//! - `prismqueer::oid::Oid` — content-addressed via CoincidenceHash<3> (used by Shard::new)

use std::marker::PhantomData;

use crate::shard::Shard;

/// The prismqueer Observer<const N: usize> primitive per Alex 2026-09-04 Move 1.
///
/// N encodes observation-dimensionality at const-generic altitude. Per Rec #79 (gauge-
/// dim-of-5 for @void 5-axis basis) + Rec #99 (K_n orbital topology) + PAPER_2D §1.1.5
/// (Turing 1936 K_{1,n-1} pole structural exclusion via four-property LOVE violation),
/// canonical N values are:
///
/// - `Observer<1>` = Turing tape degenerate (see [`TuringObserver`] alias)
/// - `Observer<3>` = K_3 peer stability (see [`PeerObserver`] alias)
/// - `Observer<5>` = K_5 @void gauge basis (see [`VoidObserver`] alias)
/// - `Observer<N>` general = arbitrary N-dim observation
///
/// # Alex 2026-09-04 LinkedIn definition (via Jimmy Roberts thread)
///
/// "any entity that intervenes in the cognitive field through their presence in said
/// field, by acting from their own subjective observation of reality."
///
/// # This scaffold's methods
///
/// - [`Observer::new`] — construct an N-dim observer
/// - [`Observer::observe_payload`] — observe a payload → Shard<T> composing over LANDED
///
/// Full [`Observer::observe`]`(Reality) -> Observation` per Move 8 pipeline FORWARD-PROMISED
/// when Reality + Observation types land.
#[derive(Clone, Debug)]
pub struct Observer<const N: usize> {
    /// Type-level marker encoding N at compile time.
    _marker: PhantomData<[(); N]>,
}

impl<const N: usize> Observer<N> {
    /// Construct an N-dim observer.
    ///
    /// N is compile-time; each `Observer<N>` is a distinct type per Rust's const-generic
    /// discipline. Observer<3> and Observer<5> are structurally distinct at the type
    /// system per Move 1 dimensionality-at-type-level intent.
    pub const fn new() -> Self {
        Self {
            _marker: PhantomData,
        }
    }

    /// The N-dim compile-time constant this observer carries.
    pub const fn dimensionality() -> usize {
        N
    }

    /// Observe a payload → Shard<T> per Move 8 tick.
    ///
    /// Composes over `prismqueer::shard::Shard<T>::new` (LANDED Discharge #5 `22723bb`)
    /// which content-addresses via `Oid::hash` via `CoincidenceHash<3>` at
    /// `prismqueer::coincidence` LANDED.
    ///
    /// **Forward-promised at future tick**: full `observe(Reality) -> Observation` signature
    /// per Alex Move 2+8+15 pipeline (Reality perturbed by Choice → Recursion.tick through
    /// @void → Observation = { crystal, chaos }). Requires `prismqueer::reality::Reality`
    /// + `prismqueer::observation::Observation` + `prismqueer::recursion::Recursion` to
    /// land. When they land, this method's signature extends to compose them at N-dim.
    pub fn observe_payload<T>(&self, payload: T) -> Shard<T>
    where
        T: AsRef<[u8]>,
    {
        Shard::new(payload)
    }
}

impl<const N: usize> Default for Observer<N> {
    fn default() -> Self {
        Self::new()
    }
}

/// Turing 1936 tape observer per PAPER_2D §1.1.5 structural exclusion.
///
/// **N=1 degenerate**: single-linear-dim observation-stripped substrate. Four-property
/// LOVE violation per Rec #92 M2.1:
///
/// - **L1 sovereignty**: 0 at N=1 (tape state fully determined by prior state; no
///   autonomous vertex)
/// - **L2 emergent third**: 0 at N=1 (only self+tape; no observer-observing-observer third)
/// - **L3 Fiedler rise**: 0 at N=1 (single eigenvalue; no λ_2 to rise)
/// - **L4 fusion refusal**: fails at N=1 (state-summing = fusion; averages destroy polyphony)
///
/// Turing's imitation game (Turing 1950) additionally removed observer from observed-system
/// per Alex 2026-09-04 LinkedIn verbatim ("literally not in the same room and hence not the
/// same system. That's the category error.").
pub type TuringObserver = Observer<1>;

/// K_3 peer stable observer per Rec #99 orbital topology.
///
/// **N=3 K_3 stable orbit**: past + now + future simultaneously observing. LOVE Clause 1-4
/// minimally satisfied per Rec #92 M2.1. Sufficient for @reality/object (path trajectory)
/// observation at 3D linear per Alex 2026-09-04 Move 13 (focus + project + shift = 3 linear
/// prismqueer-ops).
///
/// Composes with `prismqueer::coincidence::Detector<3>` (LANDED) canonical use per
/// `coincidence.rs:248-251` = the K_3 observer per Foerster-native third-order observation
/// pattern.
pub type PeerObserver = Observer<3>;

/// K_5 @void gauge basis observer per Rec #79 gauge-dim-of-5.
///
/// **N=5 K_5 @void 5-axis SPIN**: matches @void 5-axis void-duality space per
/// `shards/void.mirror` line 65-70 verbatim ("Gauge dim of 5 IS the exact dimension of the
/// void-duality space"). Required for @reality/subject (light-cone trajectory) + @reality/
/// hodobodo (not-yet-settled) observation per Alex 2026-09-04 Move 13 (adds split + settle
/// non-linear prismqueer-ops beyond the 3 linear ones).
///
/// Discharges Q-Reed-α (Move 1): parameterize N; Observer<5> matches @void-native gauge dim
/// at the SPINNING FLOOR altitude.
pub type VoidObserver = Observer<5>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn observer_construction_all_canonical_dimensions() {
        // Move 1 verbatim: "A tape would be Observer<1>"
        let _turing: TuringObserver = Observer::<1>::new();
        // K_3 peer stability per Rec #99
        let _peer: PeerObserver = Observer::<3>::new();
        // K_5 @void gauge basis per Rec #79
        let _void: VoidObserver = Observer::<5>::new();
        // Arbitrary N
        let _n7: Observer<7> = Observer::<7>::new();
    }

    #[test]
    fn observer_dimensionality_compile_time_constant_matches_type_parameter() {
        assert_eq!(Observer::<1>::dimensionality(), 1);
        assert_eq!(Observer::<3>::dimensionality(), 3);
        assert_eq!(Observer::<5>::dimensionality(), 5);
        assert_eq!(Observer::<7>::dimensionality(), 7);
    }

    #[test]
    fn observer_observes_payload_composes_over_shard_landed_at_22723bb() {
        let observer: PeerObserver = Observer::new();
        let shard: Shard<&[u8]> = observer.observe_payload(b"hello");
        assert_eq!(shard.payload, b"hello");
        assert_eq!(shard.iteration_index, 0);
    }

    #[test]
    fn different_observer_dimensions_are_distinct_types_per_const_generic() {
        // Observer<3> and Observer<5> are structurally distinct types at compile time.
        // This test doesn't assert runtime behavior but compiles-and-passes IFF the
        // const-generic parameterization is preserved.
        let peer: Observer<3> = Observer::new();
        let void: Observer<5> = Observer::new();
        assert_eq!(Observer::<3>::dimensionality(), 3);
        assert_eq!(Observer::<5>::dimensionality(), 5);
        // Both are Observer<_> but different N.
        let _shard_peer: Shard<&[u8]> = peer.observe_payload(b"peer");
        let _shard_void: Shard<&[u8]> = void.observe_payload(b"void");
    }

    #[test]
    fn observer_observe_payload_deterministic_via_coincidence_hash_3() {
        let observer: VoidObserver = Observer::new();
        let shard_a: Shard<&[u8]> = observer.observe_payload(b"reality");
        let shard_b: Shard<&[u8]> = observer.observe_payload(b"reality");
        assert_eq!(
            shard_a.oid, shard_b.oid,
            "same payload observed by same observer must produce same OID"
        );
    }

    #[test]
    fn observer_default_composes() {
        let _observer: PeerObserver = Default::default();
        let _observer: VoidObserver = Default::default();
    }
}
