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
use sha2::{Digest, Sha256};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};
use terni::{Diagnostic, Loss, Metric, PropertyVerdict};

// ──────────────────────────────────────────────────────────────────
// FluxThread
// ──────────────────────────────────────────────────────────────────

/// A Bundle whose commutator can be computed at Rust altitude via the
/// composition of Gauge action + Transport holonomy.
///
/// Blanket-implemented for any type that satisfies `Transport` (whose
/// supertraits `Fiber`, `Connection`, `Gauge` are automatically
/// satisfied). Users do NOT implement this trait directly — implementing
/// `Transport` grants FluxThread for free.
pub trait FluxThread: Transport
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

impl<T> FluxThread for T
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
/// magnitude via `FluxThread::commutator_magnitude` on demand.
pub struct Commutator<'a, C: FluxThread>
where
    C::Optic: crate::Prism,
    <<C::Optic as crate::Prism>::Input as crate::Beam>::In: Sized,
{
    a: &'a C,
    b: &'a C,
    state: &'a C::State,
}

impl<'a, C: FluxThread> Commutator<'a, C>
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
pub fn commutator<'a, C: FluxThread>(
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
/// state, or call `FluxThread::commutator_magnitude` directly.
pub fn commutator_norm<C>(a: &C, b: &C) -> C::Holonomy
where
    C: FluxThread,
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

    /// **Pillar V (fate composition) — `HolonomyHealth` verdict marshaling.**
    ///
    /// Convert fate's `HolonomyHealth` classification into a
    /// `PropertyVerdict`. The mapping:
    ///
    /// - `Healthy` → `Pass`
    /// - `TooShallow` → `Partial { confidence: 0.5, .. }` — step barely
    ///   moved the manifold; signal present but not decisive
    /// - `OverCutting` → `Fail` — geometric distortion; loss > 10×
    ///   `BERRY_PHASE` (`= 0.847`, the fiber-bundle constant per
    ///   `crate::fate::feature`)
    ///
    /// **Substrate-honest divergence from spec §7.2:** Mara's spec
    /// proposed a `theta_pass` parameter, but `HolonomyHealth` (per
    /// `crate::fate::feature::holonomy_health`) is already a three-way
    /// classification against `BERRY_PHASE`. The threshold is baked
    /// into the fate carrier; a pillar-side theta would be redundant.
    /// Reed adjudication: match on the enum directly; report the
    /// divergence in the spec's next REED-INLINE cascade.
    ///
    /// Behind `fate` feature.
    #[cfg(feature = "fate")]
    pub fn of_health(
        health: &crate::fate::feature::HolonomyHealth,
    ) -> PropertyVerdict {
        use crate::fate::feature::HolonomyHealth;
        match health {
            HolonomyHealth::Healthy => PropertyVerdict::Pass,
            HolonomyHealth::TooShallow => PropertyVerdict::Partial {
                confidence: 0.5,
                diagnostics: vec![Diagnostic::new(
                    "holonomy too shallow: step barely moved the manifold",
                )],
            },
            HolonomyHealth::OverCutting => PropertyVerdict::Fail(Diagnostic::new(
                "holonomy over-cutting: geometric distortion (loss > 10× BERRY_PHASE)",
            )),
        }
    }

    /// **Fold a sequence of verdicts into a single unified verdict.**
    ///
    /// Uses `PropertyVerdict::merge_with` starting from `Pass`.
    /// Semantics: `Fail` dominates (any `Fail` in the sequence →
    /// unified `Fail`); `Pass` is the neutral element; two
    /// `Partial`s take min confidence + union diagnostics.
    ///
    /// Empty input → `Pass` (the neutral element).
    ///
    /// Substrate-honest formalization of the fold pattern that
    /// mirror-side property tests (e.g. `rust/src/collapse.rs::
    /// prop_tests::merged_algedonic_verdicts_pass_when_all_ticks_
    /// positive`) previously did manually. Parallel to
    /// [`viability_of_magnitudes`] + [`algedonic_of_magnitude`] —
    /// completes the pillar composition surface.
    pub fn fold(verdicts: &[PropertyVerdict]) -> PropertyVerdict {
        let mut unified = PropertyVerdict::Pass;
        for v in verdicts {
            unified.merge_with(v);
        }
        unified
    }

    /// **Pillar II — algedonic threshold, generalized.**
    ///
    /// Same Pass/Partial/Fail semantics as [`algedonic`] but on a
    /// raw `Loss` magnitude instead of a `Commutator` wrapper. Use
    /// this when the magnitude comes from substrate-specific
    /// measurements — e.g. a single collapse tick's byte-shrinkage
    /// from `mirror/rust/src/collapse.rs`, or any measured `Loss`
    /// that does not originate from a commutator computation.
    ///
    /// Parallel to [`viability_of_magnitudes`] (iter 4) — completes
    /// the symmetric generalization of Pillar II + Pillar III to
    /// domain-specific `Loss` values.
    ///
    /// - Pass when `magnitude > theta`.
    /// - Fail when `magnitude.is_zero()` (no signal to detect).
    /// - Partial when `0 < magnitude <= theta`
    ///   (`confidence = 0.5` Rice-safe midpoint).
    pub fn algedonic_of_magnitude<L>(magnitude: &L, theta: &L) -> PropertyVerdict
    where
        L: Loss + PartialOrd,
    {
        if magnitude > theta {
            PropertyVerdict::Pass
        } else if magnitude.is_zero() {
            PropertyVerdict::Fail(Diagnostic::new(
                "magnitude vanished; no algedonic signal",
            ))
        } else {
            PropertyVerdict::Partial {
                confidence: 0.5,
                diagnostics: vec![Diagnostic::new(
                    "algedonic signal present but below threshold",
                )],
            }
        }
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
        C: FluxThread,
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
        C: FluxThread,
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

    // ─────────────────────────────────────────────────────────────────
    // Arc 2A — Sample + Arbitrary + forall (property-based-testing surface).
    //
    // Per Mara witnessed-property-inference spec (mirror repo)
    // §7.1 + §7.3 + §9.2, Hypothesis-style choice-sequence buffer
    // with deterministic replay + on-demand SplitMix64 extension.
    //
    // The buffer IS the trace of decisions; two samples with the
    // same buffer produce byte-identical draws — this is what makes
    // shrinking + content-addressed verdict cache work.
    //
    // Alex 2026-07-18 direction: "the full statespace covered liquid
    // floor boards." This is the first board — the surface Void's
    // default @peer stands on when running rust/ altitude property
    // tests. All state paths are admitted by pillar::forall verdicts.
    // ─────────────────────────────────────────────────────────────────

    static SAMPLE_COUNTER: AtomicU64 = AtomicU64::new(0);

    /// SplitMix64 — deterministic 64-bit PRNG for on-demand buffer
    /// extension. Zero-deps; standard splitmix64 constants from
    /// Steele-Lea-Flood 2014 "Fast Splittable Pseudorandom Number
    /// Generators" (OOPSLA).
    fn splitmix64(mut z: u64) -> u64 {
        z = z.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let z1 = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        let z2 = (z1 ^ (z1 >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z2 ^ (z2 >> 31)
    }

    /// Hypothesis-style choice-sequence buffer with deterministic
    /// replay + on-demand extension.
    ///
    /// The buffer IS the trace of decisions the property test made.
    /// `Sample::from_bytes(bytes)` replays those decisions byte-for-
    /// byte; `Sample::new()` seeds fresh from time+counter for random
    /// exploration; both extend the buffer via SplitMix64 when a draw
    /// exceeds the current buffer.
    ///
    /// Two Samples with byte-equal buffers produce byte-equal draws
    /// — this is what makes shrinking work (mutate buffer bytes,
    /// replay to observe verdict-delta) AND what makes content-
    /// addressed verdict caches work (`sha256(spec_oid || target_oid
    /// || buffer_oid)` uniquely keys a verdict).
    pub struct Sample {
        buffer: Vec<u8>,
        position: usize,
        /// Seed for SplitMix64 extension when position exceeds buffer.
        /// Derived from initial buffer hash (`from_bytes`) or from
        /// time+counter (`new`). Persists across draws so extension
        /// is deterministic-per-Sample.
        seed: u64,
        /// Optional Fate bias distribution (composition seam for
        /// mirror-side @kintsugi/butterfly + roomba walkers).
        /// Reserved for Arc 5; not read yet at Arc 2A.
        #[allow(dead_code)]
        bias: Option<[f64; 5]>,
    }

    impl Sample {
        /// Fresh Sample seeded from time+counter. Non-deterministic
        /// across process runs (by design — random exploration). Use
        /// [`Sample::from_bytes`] for deterministic replay.
        pub fn new() -> Self {
            let counter = SAMPLE_COUNTER.fetch_add(1, Ordering::Relaxed);
            let time = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .map(|d| d.as_nanos() as u64)
                .unwrap_or(0);
            let seed = splitmix64(
                time.wrapping_add(counter.wrapping_mul(0x9E37_79B9_7F4A_7C15)),
            );
            Self {
                buffer: Vec::new(),
                position: 0,
                seed,
                bias: None,
            }
        }

        /// Sample seeded from an explicit byte-buffer. Two Samples
        /// with the same buffer produce byte-identical draws
        /// (deterministic replay). Extension when the buffer is
        /// exhausted uses SplitMix64 seeded from the buffer's SHA-256.
        pub fn from_bytes(buffer: Vec<u8>) -> Self {
            let seed = if buffer.is_empty() {
                0
            } else {
                let mut hasher = Sha256::new();
                hasher.update(&buffer);
                let hash = hasher.finalize();
                u64::from_le_bytes(hash[0..8].try_into().unwrap())
            };
            Self {
                buffer,
                position: 0,
                seed,
                bias: None,
            }
        }

        /// Current read position in the buffer. Advances by one per
        /// [`draw_bool`]; by 8 per [`draw_integer`] or 8-byte draws;
        /// by 4 per `i32`/`u32` draws.
        pub fn depth(&self) -> usize {
            self.position
        }

        /// Content-address the buffer via SHA-256. Two Samples with
        /// the same buffer bytes have the same `buffer_oid`; this is
        /// the key material for the `@mirror/store/liquid` verdict
        /// cache per witnessed-property-inference spec §5.2.
        pub fn buffer_oid(&self) -> [u8; 32] {
            let mut hasher = Sha256::new();
            hasher.update(&self.buffer);
            hasher.finalize().into()
        }

        /// Set Fate bias distribution. Reserved seam for Arc 5
        /// (Roomba stigmergy) + @kintsugi/butterfly walker composition
        /// per mirror spec §7.5.
        #[allow(dead_code)]
        pub fn set_bias(&mut self, bias: &[f64; 5]) {
            self.bias = Some(*bias);
        }

        /// Read next byte; extend buffer via SplitMix64 if exhausted.
        fn read_u8(&mut self) -> u8 {
            if self.position >= self.buffer.len() {
                self.seed = splitmix64(self.seed);
                self.buffer.extend_from_slice(&self.seed.to_le_bytes());
            }
            let byte = self.buffer[self.position];
            self.position += 1;
            byte
        }

        /// Read next N bytes; extends buffer as needed.
        fn read_bytes(&mut self, n: usize) -> Vec<u8> {
            (0..n).map(|_| self.read_u8()).collect()
        }

        /// Draw an integer uniformly in `[min, max]` (inclusive both
        /// sides). Consumes 8 bytes from the buffer.
        ///
        /// If `max < min`, returns `min` (defensive; caller should
        /// pass a valid range).
        pub fn draw_integer(&mut self, min: i64, max: i64) -> i64 {
            if max <= min {
                return min;
            }
            let bytes: [u8; 8] = self.read_bytes(8).try_into().unwrap();
            let raw = u64::from_le_bytes(bytes);
            let range = (max - min + 1) as u64;
            min + (raw % range) as i64
        }

        /// Draw a bool with p=0.5. Consumes 1 byte from the buffer.
        pub fn draw_bool(&mut self) -> bool {
            (self.read_u8() & 1) == 1
        }

        /// Draw an element uniformly from a slice of choices.
        /// Consumes 8 bytes (via [`draw_integer`]).
        ///
        /// Panics if `choices` is empty.
        pub fn draw_from<T: Copy>(&mut self, choices: &[T]) -> T {
            assert!(!choices.is_empty(), "draw_from requires non-empty choices");
            let idx = self.draw_integer(0, (choices.len() - 1) as i64) as usize;
            choices[idx]
        }
    }

    impl Default for Sample {
        fn default() -> Self {
            Self::new()
        }
    }

    /// Trait for types that can be sampled from a [`Sample`].
    ///
    /// Implement this for domain types to enable [`forall`] over them.
    /// Impls provided for `i32`, `i64`, `u32`, `u64`, and `bool`.
    pub trait Arbitrary {
        fn arbitrary(sample: &mut Sample) -> Self;
    }

    impl Arbitrary for bool {
        fn arbitrary(sample: &mut Sample) -> Self {
            sample.draw_bool()
        }
    }

    impl Arbitrary for i32 {
        fn arbitrary(sample: &mut Sample) -> Self {
            let bytes: [u8; 4] = sample.read_bytes(4).try_into().unwrap();
            i32::from_le_bytes(bytes)
        }
    }

    impl Arbitrary for i64 {
        fn arbitrary(sample: &mut Sample) -> Self {
            let bytes: [u8; 8] = sample.read_bytes(8).try_into().unwrap();
            i64::from_le_bytes(bytes)
        }
    }

    impl Arbitrary for u32 {
        fn arbitrary(sample: &mut Sample) -> Self {
            let bytes: [u8; 4] = sample.read_bytes(4).try_into().unwrap();
            u32::from_le_bytes(bytes)
        }
    }

    impl Arbitrary for u64 {
        fn arbitrary(sample: &mut Sample) -> Self {
            let bytes: [u8; 8] = sample.read_bytes(8).try_into().unwrap();
            u64::from_le_bytes(bytes)
        }
    }

    /// The property-based-testing runner.
    ///
    /// Draws `n` independent samples of `T`, applies `f` to each, and
    /// folds the resulting verdicts via [`PropertyVerdict::merge_with`]
    /// starting from `Pass`.
    ///
    /// Semantics (inherited from `merge_with`):
    /// - `Fail` dominates: any counterexample → unified `Fail`
    /// - `Pass` is the neutral element
    /// - Two `Partial`s take min confidence + union diagnostics
    ///
    /// Composes cleanly with all other pillar primitives: the
    /// per-iteration verdict `f(value)` can invoke
    /// [`algedonic`](super::pillar::algedonic),
    /// [`viability`](super::pillar::viability),
    /// [`algedonic_of_magnitude`](super::pillar::algedonic_of_magnitude),
    /// [`viability_of_magnitudes`](super::pillar::viability_of_magnitudes),
    /// [`dispatch_ambiguity`](super::pillar::dispatch_ambiguity), or
    /// return a `PropertyVerdict` directly — the fold contract holds.
    ///
    /// Per Mara witnessed-property-inference spec (mirror repo)
    /// §7.3 + §9.2 Arc 2. First liquid floor board for Void's default
    /// @peer to stand on at rust/ altitude.
    pub fn forall<T, F>(n: usize, mut f: F) -> PropertyVerdict
    where
        T: Arbitrary,
        F: FnMut(T) -> PropertyVerdict,
    {
        let mut unified = PropertyVerdict::Pass;
        for _ in 0..n {
            let mut sample = Sample::new();
            let value = T::arbitrary(&mut sample);
            let verdict = f(value);
            unified.merge_with(&verdict);
        }
        unified
    }
}

// ──────────────────────────────────────────────────────────────────
// Prelude — the delightful use-line.
// ──────────────────────────────────────────────────────────────────

/// `use prismqueer::flux::prelude::*;` — imports the surface consumers
/// need most often: commutator constructors, the `pillar` module, and
/// terni's verdict types.
pub mod prelude {
    pub use super::pillar;
    pub use super::{commutator, commutator_norm, Commutator, FluxThread};
    pub use terni::{Diagnostic, PropertyVerdict};
}
