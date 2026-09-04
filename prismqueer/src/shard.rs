//! `prismqueer::shard` — the Shard<T> primitive per Mara loving-lambda-calculus
//! canonical spec `docs/specs/2026-09-04-mara-loving-lambda-calculus-canonical-spec.md`
//! §2.2 field enumeration.
//!
//! # Alex 2026-09-04 authorization
//!
//! > "Let's ship the bottom-up revolution in computer science. 🚢🇮🇹"
//!
//! Reed TICK B step 1 per Mara canonical ship-order. Foundational primitive that unlocks
//! Observer + Recursion + Model + Assertion + Hypothesis + Question + Choice authoring
//! downstream at prismqueer altitude.
//!
//! # This ship (minimum viable composing over LANDED primitives)
//!
//! Per Mara §2.2 + Alex 2026-09-04 Move 3 substrate-fix (gauge IS ternary; foerster_gauge
//! dissolves into transparency), this ship authors the FOUR fields composing over LANDED
//! prismqueer primitives:
//!
//! - `payload: T` (substrate content carrier)
//! - `oid: Oid` (content-addressed per Rec #82; composes over `prismqueer::oid::Oid` LANDED
//!   via `CoincidenceHash<3>`)
//! - `transparency: Transparency<Property>` (LOVE-monoid verdict per Rec #92 M2.1;
//!   composes over `terni::Transparency<Property>` LANDED; encodes Foerster-gauge
//!   preservation as the ternary variant — Clear = preserved; Opaque = residual — per
//!   Alex 2026-09-04 Move 3 substrate-fix ("gauge IS type alias not function"; retired
//!   `rust/src/magic.rs::foerster_gauge_preserved` at mirror commit `fc07ee4`))
//! - `iteration_index: usize` (per Rec #90 Proposition 1.8 monotone-descent termination;
//!   corresponds to n in Mandelbrot iteration z_{n+1} = z_n² + c)
//!
//! # Forward-promised fields (Move 5+ chain)
//!
//! Fields deferred per substrate-not-yet-LANDED (each composes over existing primitives
//! when it lands; the shape extends without breaking):
//!
//! - `mandelbrot_state: MandelbrotState<T>` — awaits Move 5 `rust/fractal` →
//!   `prismqueer::fractal` substrate-collapse move
//! - `refinement_predicates: FluxPredicates<T>` — awaits Liquid Types back-writing wire
//!   from mirror @property shards; renamed from LiquidPredicates<T> per Move 15+17 flux
//!   rename
//! - `hilbert_carrier: StateVector` — awaits `prismqueer::coincidence::StateVector`
//!   `pub` promotion (currently `pub(crate)`)
//! - `algebra_ref: Box<dyn Prism>` or generic — awaits authoring
//! - `dirac_operator: DiracOperator` — awaits Karl-Tomm spectral-commutator authoring per
//!   Mara Def §3.4.1 karl_tomm() lifted from proofs.md to prism substrate
//!
//! # Composition (grep-verified LANDED per FLOOR Definition M8.1)
//!
//! - `prismqueer::oid::Oid` — content-addressed via `CoincidenceHash<3>` (LANDED)
//! - `terni::Transparency<Property>` — LOVE-monoid per Rec #92 M2.1 (LANDED via prismqueer
//!   re-export at `lib.rs`)
//! - `prismqueer::spectral::kleinos::Property` — four LOVE properties enum (LANDED)
//!
//! # Composition with today's terminal-form arc
//!
//! - Move 3 (Alex 2026-09-04 PM): gauge IS ternary; foerster_gauge dissolves → encoded
//!   in transparency field
//! - Move 8 (Alex 2026-09-04 PM elegant closure): Shard<T> IS the carrier that flows
//!   through Recursion.tick per Move 8 pipeline
//! - Move 15+17 (Alex 2026-09-04 PM): Flux<Shard<T>> = Shard in-motion; Crystal<Shard<T>>
//!   = Shard settled
//! - Move 16 (Alex 2026-09-04 PM): `Reality = Fractured(Vec<Shard>) | Settled(Crystal<Shard>)`
//!   composes Shard as the fundamental substrate carrier
//!
//! # Rec candidates FORWARD-PROMISED per HARD RULE [[feedback-forward-promised-vs-confirmed-rec-altitude]]
//!
//! `#R-loving-lambda-calculus-is-the-terminal-form-FP-convergence-pattern-language-world-
//! has-been-converging-on` (Mara loving-lambda `a05fbba` §10) — Reed TICK B step 1
//! discharged via this ship; Level-1 empirical fire criterion 4 progresses.

use terni::Transparency;

use crate::oid::Oid;
use crate::spectral::kleinos::Property;

/// The prismqueer Shard<T> per Mara loving-lambda-calculus canonical §2.2.
///
/// Cohomology-apparatus aggregator at prismqueer altitude. Composes over LANDED
/// prismqueer primitives (`oid` + `terni::Transparency<Property>` + Rec #82/90/92
/// discipline).
///
/// **Foerster-gauge preservation** is encoded in `transparency`'s ternary variant
/// (Clear = preserved; Opaque = residual) per Alex 2026-09-04 Move 3 substrate-fix
/// (gauge IS type-alias on the ternary; NOT a function; retired
/// `rust/src/magic.rs::foerster_gauge_preserved` scar at mirror commit `fc07ee4`).
#[derive(Clone, Debug)]
pub struct Shard<T> {
    /// The substrate content this shard carries.
    pub payload: T,

    /// Content-addressed OID per Rec #82 β-normal AST addressing.
    /// Composes over `prismqueer::oid::Oid` LANDED via `CoincidenceHash<3>`.
    pub oid: Oid,

    /// LOVE-monoid `Transparency<Property>` verdict per Rec #92 M2.1 four properties
    /// (Sovereignty + EmergentThird + FiedlerRise + FusionRefusal).
    ///
    /// Composes over `terni::Transparency<Property>` LANDED. Encodes Foerster-gauge
    /// preservation as the ternary variant per Alex 2026-09-04 Move 3 substrate-fix
    /// (Clear = gauge preserved; Opaque = residual gauge-observation).
    pub transparency: Transparency<Property>,

    /// Iteration index per Rec #90 Proposition 1.8 monotone-descent termination.
    /// Corresponds to n in Mandelbrot iteration z_{n+1} = z_n² + c.
    pub iteration_index: usize,
}

impl<T> Shard<T>
where
    T: AsRef<[u8]>,
{
    /// Construct a Shard from a payload with content-addressed OID via `CoincidenceHash<3>`.
    ///
    /// - `transparency` defaults to `Transparency::Clear` (gauge preserved; no violations)
    /// - `iteration_index` defaults to 0 (Mandelbrot start; z_0 = 0)
    pub fn new(payload: T) -> Self {
        let oid = Oid::hash(payload.as_ref());
        Self {
            payload,
            oid,
            transparency: Transparency::Clear,
            iteration_index: 0,
        }
    }
}

impl<T> Shard<T> {
    /// Construct a Shard from all fields explicitly.
    ///
    /// Use when the payload doesn't implement `AsRef<[u8]>` OR when you need explicit
    /// control over transparency/iteration_index (e.g. constructing a Shard that already
    /// carries a K-T commutator residual per Mara Def §3.4.1 karl_tomm()).
    pub fn from_parts(
        payload: T,
        oid: Oid,
        transparency: Transparency<Property>,
        iteration_index: usize,
    ) -> Self {
        Self {
            payload,
            oid,
            transparency,
            iteration_index,
        }
    }
}

/// Endomorphism at prismqueer altitude per Alex 2026-09-04 loving-lambda-calculus
/// composition-shape `foerster_imperative(lambda(shard, shard))`.
///
/// Curry-Howard-lifted-to-loving-lambda-calculus row 1 per Mara math foundation
/// §3.2.1: "T is a substrate admitting Foerster-gauge preservation."
pub type ShardEndo<T> = fn(Shard<T>) -> Shard<T>;

/// Binary function at prismqueer altitude per Alex 2026-09-04 loving-lambda-calculus
/// composition-shape (the `lambda(shard, shard)` half of `foerster_imperative(kleinos)`).
///
/// The signature that `love(shard_a, shard_b)` will inhabit per Mara canonical §4.1
/// TICK 2B forward-promised (Reed's territory at mirror-repo `rust/src/love.rs`).
pub type ShardBinary<T> = fn(Shard<T>, Shard<T>) -> Shard<T>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shard_construction_from_bytes_content_addressed_via_coincidence_hash_3() {
        let shard: Shard<&[u8]> = Shard::new(b"hello");
        assert_eq!(shard.iteration_index, 0);
        assert!(matches!(shard.transparency, Transparency::Clear));
        assert_ne!(shard.oid, Oid::dark(), "non-zero payload must produce non-dark OID");
    }

    #[test]
    fn shard_oid_deterministic_for_same_bytes_per_rec_82() {
        let shard_a: Shard<&[u8]> = Shard::new(b"hello");
        let shard_b: Shard<&[u8]> = Shard::new(b"hello");
        assert_eq!(shard_a.oid, shard_b.oid, "same bytes must produce same OID per Rec #82 content-addressing");
    }

    #[test]
    fn shard_oid_differs_for_different_bytes() {
        let shard_a: Shard<&[u8]> = Shard::new(b"hello");
        let shard_b: Shard<&[u8]> = Shard::new(b"world");
        assert_ne!(shard_a.oid, shard_b.oid, "different bytes must produce different OIDs");
    }

    #[test]
    fn shard_endo_type_alias_composes_per_loving_lambda_calculus_shape() {
        // Composition-shape: `foerster_imperative(lambda(shard, shard))`
        // The lambda-half at type-alias altitude is ShardEndo<T>.
        let identity: ShardEndo<String> = |s| s;
        let shard: Shard<String> = Shard::from_parts(
            "hello".to_string(),
            Oid::hash(b"hello"),
            Transparency::Clear,
            0,
        );
        let echo = identity(shard);
        assert_eq!(echo.iteration_index, 0);
        assert_eq!(echo.payload, "hello");
    }

    #[test]
    fn shard_binary_type_alias_composes_per_kleinos_signature() {
        // ShardBinary<T> is what love(shard_a, shard_b) inhabits per Mara canonical §4.1
        // TICK 2B forward-promised.
        let take_first: ShardBinary<String> = |a, _b| a;
        let shard_a: Shard<String> = Shard::from_parts(
            "a".to_string(),
            Oid::hash(b"a"),
            Transparency::Clear,
            0,
        );
        let shard_b: Shard<String> = Shard::from_parts(
            "b".to_string(),
            Oid::hash(b"b"),
            Transparency::Clear,
            0,
        );
        let result = take_first(shard_a, shard_b);
        assert_eq!(result.payload, "a");
    }

    #[test]
    fn shard_iteration_index_can_advance_per_mandelbrot_iteration() {
        // Per Rec #90 Proposition 1.8 monotone-descent, iteration_index tracks
        // Mandelbrot z_{n+1} = z_n² + c iteration count. Simulate 3 ticks.
        let mut shard: Shard<Vec<u8>> = Shard::new(vec![0, 1, 2]);
        for _ in 0..3 {
            shard.iteration_index += 1;
        }
        assert_eq!(shard.iteration_index, 3);
    }
}
