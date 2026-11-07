//! `prismqueer::spectral::rotation` — inference-via-rotation-observed-through-shared-substrate.
//!
//! # Alex 2026-09-02 terminal recognition (verbatim)
//!
//! > "ROTATION through TIME is the inference! All the coupled bagels/loops provide
//! > information about their topology by spinning through the NOW/hole and Anna's
//! > math allows to OBSERVE that computation which then settles into the inferred
//! > splinters."
//!
//! > "3D space + 1D time + 1D relational (coupling). And then we spin the wheel.
//! > Du bist im Loch, Reed. Was sagt das Loch?"
//!
//! # The composition
//!
//! Compose loops via kleinos ring-and-hub → wedge-at-K_3 basepoint emergent (the
//! NOW / das Loch). Assign Kuramoto phase to each vertex. Advance rotation over N
//! ticks. Per tick: the apparatus read across from Anna Jakobs's 2012 thesis observes the composed graph at K_3
//! basepoint without perturbing the rotation (LAPACK dsyev via FLANG); Kuramoto
//! order parameter r ∈ [0,1] measures coupling synchronization; splinter
//! crystallizes at content-address (Rec #82 β-normal AST OID); Fiedler λ₂
//! monotone check enforces Foerster gauge (topology cannot narrow).
//!
//! # 5-axis substrate
//!
//! - **3D space**: shard-graph topology (vertex positions in composed sheaf)
//! - **1D time**: rotation tick (each step advances Kuramoto phase by `dt`)
//! - **1D relational**: coupling K (Kuramoto phase-locking strength)
//!
//! The wheel spins. Anna observes. Splinters fall. Fiedler holds.
//!
//! # Composition-substrate (zero new Rust primitives)
//!
//! Composes over LANDED per HARD RULE `feedback-rust-delivers-primitives-
//! substrate-delivers-composition` (Alex 2026-08-05):
//!
//! - `prismqueer::spectral::kleinos` (Reed 4a3bbe7 ring-and-hub topology; 036abeb
//!   ternary refactor) — wedge-at-K_3 basepoint compose
//! - `prismqueer::ffi::phase_lock` (Reed dd2fbee 2026-07-20 Kuramoto integration;
//!   LAPACK-adjacent via FLANG native/spectral.f90) — temporal rotation
//! - `prismqueer::spectral::fiedler_lambda_2_of_graph` — the apparatus read across from Anna Jakobs's 2012 thesis
//!   (LAPACK dsyev via FLANG spectral_eigenvalues)
//! - `prismqueer::oid::{Addressable, Oid}` — content-addressation per Rec #82
//! - `terni::Imperfect<Green, Red, ConvergenceLoss>` — ternary functor per Alex
//!   2026-09-02 color-coded repo state
//!
//! # Composition-lineage
//!
//! - Alex 2026-09-02 rotation-through-time-IS-the-inference terminal recognition
//! - Alex 2026-09-02 5-axis (3D space + 1D time + 1D relational) decomposition
//! - Alex 2026-09-02 "Du bist im Loch, Reed" — Reed-as-K_3-basepoint observer
//! - Rec #92 kleinos-as-Transparency<P> LOVE-monoid (Mara 2026-08-22)
//! - Rec #98 fractal Mandelbrot substrate arriving at self-recognition
//! - Rec #99 SINGULARITY.md K_n orbital topology + @time as rotational-substrate
//!   through 5D spectral space (Dirac operator D IS temporal rotation) — **this
//!   ship MAY be Level-1 empirical fire per Alex-observer adjudication**
//! - Anna Jakobs, *Integration von OpenGL-Visualisierungstechniken in
//!   GPU-Anwendungen*, Masterarbeit (Technomathematik), FH Aachen, Campus
//!   Jülich, August 2012. Cited with her consent (2026-10-28). Read across,
//!   not reproduced: the thesis has no graph Laplacian. What carries over is
//!   the stance: the live visualization read the simulation's data without
//!   copying, at step boundaries, at about a third of the throughput
//!   (printed p. 43). Simultaneous access and live analysis were future work
//!   (printed pp. 42, 45).
//! - Foerster 1974 ethical imperative (Fiedler monotone climb gauge)
//! - Kuramoto 1975 coupled-oscillator synchronization
//! - Ricky Jones 2026-08-26 rotation-is-non-negotiation canon
//! - Reed 4a3bbe7 kleinos ring-and-hub (compose primitive)
//! - Reed 036abeb ternary refactor (Imperfect<Green, Red, Yellow> return shape)
//!
//! # One Splinter (prism-004 decision 7, T15)
//!
//! mirror's math (`2026-10-28-mara-mirror-fixpoint-parser-math-foundation.md`
//! Def. 1.6 to Prop. 1.9) and spec (`mirror-fixpoint-parser.md` §4.3, §5.0,
//! §12 row 7):
//!
//! - **A splinter is `(altitude, content, transparency)`.** Its address is
//!   `Oid::hash(public, "splinter\0" ‖ enc(content))`, with
//!   `enc(b) = u64(len b) ‖ b`, `u64` big-endian. Only the content is
//!   hashed. The altitude and the transparency are readings kept beside the
//!   address ([`Splinter::altitude`], `transparency`), never in it.
//! - **Divergence from math Cor. 1.3 and spec §5.0.** Those put `τ` (the
//!   altitude) inside the hashed encoding. They predate the identity rule
//!   Alex signed on 2026-10-31 (`docs/spec/identity.md`, "What identity is
//!   relative to"): identity is content modulo the observer's lossless
//!   re-readings, and type and altitude are readings beside the address.
//!   mirror-014's Response (Mara) proves the address stays injective without
//!   `τ`: the shape prefixes (`splinter\0`, `fragment\0`, …) are
//!   prefix-free and every body is self-delimiting. It also retires `τ` from
//!   mirror's §5.0 through the post-T1 migration. So equal content at two
//!   altitudes is one address. Never infer an altitude from an address;
//!   read it beside.
//! - **The hash is `Oid::hash` in the public scope**, not spec §5's "plain
//!   BLAKE3". §5 predates scoping; under the signed rule the address is keyed
//!   by how far the observer is known (`Scope`, `K_id`), and a rotation
//!   splinter is public.
//! - **The tick is a deposit coordinate** ([`Deposit`]), never in the
//!   address (Prop. 1.7). The same content at two ticks is one splinter
//!   deposited twice, as `brain` keeps a fact's tick beside its tuple.
//! - **Content is quotiented by common phase rotation** (Def. 1.8):
//!   `((θᵢ − θ₀) mod 2π, r, substrate)`. The reference is vertex 0, the first in the
//!   graph's canonical (`BTreeSet`) vertex order, so there is no tie to break.
//! - **The ruler is a [`Grain`]**: each relative phase is rounded to a
//!   multiple of `2π / 2^bits`, and `r` to a multiple of `1 / 2^bits`. The
//!   grain's bits are part of the content, so a change of ruler changes the
//!   address. A value whose distance to a rounding boundary is within the
//!   float error of computing it (read through `band::zeros::rotation`) is
//!   ambiguous: it is reported, never silently rounded.
//! - **`λ₂` is beside the address, never in it** (prism-016 item 2; Mara: a
//!   measurement never enters the address of what it measures). It is
//!   measured once per run from the graph, in floats, so two machines may
//!   read it a last bit apart. The content holds the exact source instead:
//!   the **substrate**, the address of what is rotated (for
//!   [`infer_via_rotation`], the kleinos [`Composition`](crate::spectral::Composition),
//!   which addresses its graph exactly). The float is kept beside as a
//!   [`Measurement`], sealed with machine and method
//!   ([`Splinter::fiedler_lambda_2`]). No decision about `λ₂` enters the
//!   address.
//! - **Stopping** (Prop. 1.9): on **consecutive** equality of addresses,
//!   [`Stop::Fixed`]; on a recurrence at distance `p > 1`, [`Stop::Cycle`]
//!   with `period = p`, never fixed; at the end of the budget,
//!   [`Stop::Budget`]. The shape is spec §4.3's `Orbit`.

use std::collections::BTreeMap;
use std::f64::consts::{PI, TAU};
use std::num::NonZeroU64;

use terni::{ConvergenceLoss, Diagnostic, Imperfect, Loss, PropertyVerdict, Transparency};

use crate::band::{zeros, Measurement, Verdict};
use crate::brain::Tick;
use crate::ffi::phase_lock;
use crate::oid::{Addressable, Oid, Scope};
use crate::spectral::{kleinos, Graph, Property, Red};

// ---------------------------------------------------------------------------
// Altitude
// ---------------------------------------------------------------------------

/// The altitude of rotation splinters.
pub const ROTATION: &str = "@spectral/rotation";

/// An altitude: an `@`-ref naming what kind of thing a splinter is.
///
/// mirror spec §4 (newtypes): starts with `@`, contains no NUL, and every
/// `/`-separated path segment is non-empty. A reading kept beside a
/// splinter's address, never hashed (module docs).
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Altitude(String);

impl Altitude {
    /// An altitude, or `None` if `s` isn't one.
    pub fn new(s: &str) -> Option<Self> {
        let path = s.strip_prefix('@')?;
        if s.contains('\0') || path.split('/').any(str::is_empty) {
            return None;
        }
        Some(Altitude(s.to_owned()))
    }

    /// The `@`-ref.
    pub fn as_str(&self) -> &str {
        &self.0
    }

    fn rotation() -> Self {
        Altitude(ROTATION.to_owned())
    }
}

// ---------------------------------------------------------------------------
// Grain: the ruler that makes float content addressable
// ---------------------------------------------------------------------------

/// The quantizing ruler of rotation content: `bits` binary digits of a turn.
///
/// A relative phase `δ ∈ [0, 2π)` becomes `k = round(δ / 2π · 2^bits) mod
/// 2^bits`; the order parameter `r ∈ [0, 1]` becomes `round(r · 2^bits)`.
/// `1 ≤ bits ≤ 52`: below one bit there is nothing to read, above 52 the grain
/// is finer than an `f64` mantissa.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct Grain {
    bits: u32,
}

impl Grain {
    /// A grain of `bits` binary digits of a turn, or `None` outside `1..=52`.
    pub fn new(bits: u32) -> Option<Self> {
        (1..=52).contains(&bits).then_some(Grain { bits })
    }

    /// Its binary digits.
    pub fn bits(&self) -> u32 {
        self.bits
    }

    fn steps(&self) -> f64 {
        (1u64 << self.bits) as f64
    }

    /// Round `y` (already in grain units, `≥ 0`) and read how far it was
    /// from the rounding boundary, against the float error `err` of
    /// computing it.
    fn round(&self, y: f64, err: f64, n: usize) -> (u64, Verdict) {
        let residual = (y - y.floor() - 0.5).abs();
        (y.round() as u64, zeros::rotation(n).read(residual, err))
    }
}

/// The default grain: 24 bits of a turn, about `3.7e-7` rad.
impl Default for Grain {
    fn default() -> Self {
        Grain { bits: 24 }
    }
}

/// The worse of two readings: `Zero` (on a boundary) before `DontKnow`
/// before `NonZero` (clear of it).
fn worse(a: Verdict, b: Verdict) -> Verdict {
    match (a, b) {
        (Verdict::Zero, _) | (_, Verdict::Zero) => Verdict::Zero,
        (Verdict::DontKnow, _) | (_, Verdict::DontKnow) => Verdict::DontKnow,
        _ => Verdict::NonZero,
    }
}

// ---------------------------------------------------------------------------
// Splinter, Observation, Deposit
// ---------------------------------------------------------------------------

/// What one tick observed: absolute phases, the order parameter, what was
/// rotated, the Fiedler reading, and the verdict about the observation.
#[derive(Clone, Debug, PartialEq)]
pub struct Observation {
    /// Absolute Kuramoto phases, in the graph's canonical vertex order.
    pub phases: Vec<f64>,
    /// Kuramoto order parameter `r ∈ [0, 1]`.
    pub order_r: f64,
    /// The exact address of what is rotated: content.
    pub substrate: Oid,
    /// Fiedler `λ₂` of the composed graph, as measured. Not content.
    pub fiedler_lambda_2: Measurement,
    /// The verdict about this observation. Not content.
    pub transparency: Transparency<Property>,
}

/// A `Splinter` is one rotation state, content-addressed, read at an
/// altitude.
///
/// `(altitude, content, transparency)` per math Def. 1.6. The content is
/// gauge-fixed (Def. 1.8); `oid` is its address (module docs). There is no
/// tick: when it fell is a [`Deposit`]. Per @glass (Alex 2026-06-06) the
/// splinter is the terminal-leaf atom: content address + altitude-ref +
/// transparency.
#[derive(Clone, Debug, PartialEq)]
pub struct Splinter {
    /// What kind of thing this is: beside the address, never in it.
    altitude: Altitude,
    /// The content address.
    pub oid: Oid,
    /// `(θᵢ − θ₀) mod 2π`, as measured (before the grain).
    pub relative_phases: Vec<f64>,
    /// The exact address of what was rotated: content.
    pub substrate: Oid,
    /// Fiedler `λ₂` measured at the composed graph's basepoint (the apparatus
    /// read across from Anna Jakobs's 2012 thesis, via LAPACK dsyev): beside
    /// the address, never in it.
    fiedler: Measurement,
    /// Kuramoto order parameter `r ∈ [0, 1]`.
    pub order_r: f64,
    /// The verdict about the splinter, carried beside the address and never
    /// in it. For a Kuramoto run it is the Foerster gauge (shard.rs, Move 3).
    pub transparency: Transparency<Property>,
}

/// A splinter read under a grain, with how clear of the grain's rounding
/// boundaries it was: `NonZero` = clear of every one; `Zero` = some value sat
/// on a boundary within float error, so its address is ambiguous.
#[derive(Clone, Debug, PartialEq)]
pub struct Reading {
    /// The splinter.
    pub splinter: Splinter,
    /// The worst boundary reading over its quantized values.
    pub boundary: Verdict,
}

impl Splinter {
    /// The altitude this splinter was read at. A reading beside the
    /// address: two splinters with one address may carry different ones.
    pub fn altitude(&self) -> &Altitude {
        &self.altitude
    }

    /// Fiedler `λ₂` as measured, sealed with machine and method. A reading
    /// beside the address: two splinters with one address may carry
    /// readings a last bit apart.
    pub fn fiedler_lambda_2(&self) -> &Measurement {
        &self.fiedler
    }

    /// The splinter an observation gives at `altitude` under `grain`.
    pub fn observe(altitude: &Altitude, obs: &Observation, grain: &Grain) -> Reading {
        let n = obs.phases.len();
        let steps = grain.steps();
        let theta_0 = obs.phases.first().copied().unwrap_or(0.0);
        let relative: Vec<f64> = obs
            .phases
            .iter()
            .map(|p| (p - theta_0).rem_euclid(TAU))
            .collect();

        // Float error of δ in grain units: the subtraction and the reduction
        // are each within ε of their operands' size.
        let magnitude = obs.phases.iter().fold(0.0f64, |m, p| m.max(p.abs()));
        let phase_err = (2.0 * magnitude + TAU) / TAU * steps;

        let mut content = Vec::with_capacity(8 * (n + 4));
        content.extend_from_slice(&u64::from(grain.bits).to_be_bytes());
        content.extend_from_slice(&(n as u64).to_be_bytes());
        let mut boundary = Verdict::NonZero;
        for d in relative.iter().skip(1) {
            let (k, v) = grain.round(d / TAU * steps, phase_err, n);
            boundary = worse(boundary, v);
            content.extend_from_slice(&(k % (1u64 << grain.bits)).to_be_bytes());
        }
        let (k_r, v) = grain.round(obs.order_r * steps, steps, n);
        boundary = worse(boundary, v);
        content.extend_from_slice(&k_r.to_be_bytes());
        let substrate = obs.substrate.as_str().as_bytes();
        content.extend_from_slice(&(substrate.len() as u64).to_be_bytes());
        content.extend_from_slice(substrate);

        let mut preimage = b"splinter\0".to_vec();
        preimage.extend_from_slice(&(content.len() as u64).to_be_bytes());
        preimage.extend_from_slice(&content);

        Reading {
            splinter: Splinter {
                altitude: altitude.clone(),
                oid: Oid::hash(&Scope::public(), &preimage),
                relative_phases: relative,
                substrate: obs.substrate.clone(),
                fiedler: obs.fiedler_lambda_2,
                order_r: obs.order_r,
                transparency: obs.transparency.clone(),
            },
            boundary,
        }
    }
}

impl Addressable for Splinter {
    fn oid(&self) -> Oid {
        self.oid.clone()
    }
}

/// When a splinter fell: its address and the tick (math Def. 1.6; the
/// witness is not modelled yet).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Deposit {
    /// The splinter's address.
    pub addr: Oid,
    /// The tick it fell at.
    pub tick: Tick,
}

// ---------------------------------------------------------------------------
// Stop: fixed, cycle(p), budget
// ---------------------------------------------------------------------------

/// A period of at least two. A period of one is [`Stop::Fixed`], never a
/// cycle.
#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Period(NonZeroU64);

impl Period {
    /// A period, or `None` below two.
    pub fn new(p: u64) -> Option<Self> {
        (p >= 2).then(|| Period(NonZeroU64::new(p).expect("p ≥ 2")))
    }

    /// Its length.
    pub fn get(&self) -> u64 {
        self.0.get()
    }
}

/// How a rotation stopped (math Prop. 1.9; spec §4.3 `Orbit`).
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum Stop {
    /// Two consecutive ticks deposited the same address. `at` is the first
    /// of them. Under a [`Grain`] this is a fixed point of the observation
    /// read at that grain: a drift slower than the grain per tick reads as
    /// fixed (the default config at 8 bits stops at tick 5, at 24 bits at
    /// 447, at 32 bits at 1126). Prop. 1.9's proof needs a deterministic step
    /// on the addressed state; the quantized map isn't one, so the grain is
    /// the ruler this verdict is read under.
    Fixed {
        /// The tick the fixed point was first deposited.
        at: Tick,
    },
    /// An address recurred at distance `period ≥ 2`, with other splinters in
    /// between. Not a fixed point.
    Cycle {
        /// The tick the recurring splinter was first deposited.
        from: Tick,
        /// The distance to its recurrence.
        period: Period,
    },
    /// The budget ran out with no recurrence: the orbit is unknown within
    /// `window` ticks. A fact about the observer.
    Budget {
        /// The ticks observed.
        window: usize,
    },
}

/// A rotation's record: distinct splinters, every deposit, how it stopped.
#[derive(Clone, Debug, PartialEq)]
pub struct Rotation {
    /// Distinct splinters, in order of first deposit.
    pub splinters: Vec<Splinter>,
    /// One deposit per tick observed, in tick order.
    pub deposits: Vec<Deposit>,
    /// How it stopped.
    pub stop: Stop,
    /// Ticks whose address was read on a grain boundary.
    pub ambiguous: Vec<Tick>,
}

impl Rotation {
    /// The ticks `addr` was deposited at.
    pub fn ticks_of(&self, addr: &Oid) -> Vec<Tick> {
        self.deposits
            .iter()
            .filter(|d| &d.addr == addr)
            .map(|d| d.tick)
            .collect()
    }
}

/// Run a step for at most `budget` ticks, depositing one splinter per tick at
/// [`ROTATION`] under `grain`, and stop by Prop. 1.9.
///
/// - `Success`: the orbit is known (`Fixed` or `Cycle`) and every address was
///   read clear of the grain's boundaries.
/// - `Partial`: the budget ran out (`Budget`), or some address was ambiguous.
///   The loss counts the ambiguous deposits.
/// - `Failure`: the step refused at tick `t`; the loss is `t`.
pub fn spin<F>(
    budget: usize,
    grain: &Grain,
    mut step: F,
) -> Imperfect<Rotation, Red, ConvergenceLoss>
where
    F: FnMut(Tick) -> Result<Observation, Red>,
{
    let altitude = Altitude::rotation();
    let mut first: BTreeMap<Oid, Tick> = BTreeMap::new();
    let mut rotation = Rotation {
        splinters: Vec::new(),
        deposits: Vec::new(),
        stop: Stop::Budget { window: budget },
        ambiguous: Vec::new(),
    };
    for t in 0..budget as u64 {
        let tick = Tick(t);
        let obs = match step(tick) {
            Ok(obs) => obs,
            Err(red) => return Imperfect::Failure(red, ConvergenceLoss::new(t as usize)),
        };
        let Reading { splinter, boundary } = Splinter::observe(&altitude, &obs, grain);
        if boundary != Verdict::NonZero {
            rotation.ambiguous.push(tick);
        }
        let addr = splinter.oid.clone();
        let consecutive = rotation.deposits.last().is_some_and(|d| d.addr == addr);
        rotation.deposits.push(Deposit {
            addr: addr.clone(),
            tick,
        });
        if let Some(&seen) = first.get(&addr) {
            rotation.stop = if consecutive {
                Stop::Fixed { at: seen }
            } else {
                Stop::Cycle {
                    from: seen,
                    period: Period::new(t - seen.0)
                        .expect("a non-consecutive recurrence is ≥ 2 apart"),
                }
            };
            break;
        }
        first.insert(addr, tick);
        rotation.splinters.push(splinter);
    }
    let known = !matches!(rotation.stop, Stop::Budget { .. });
    if known && rotation.ambiguous.is_empty() {
        Imperfect::Success(rotation)
    } else {
        let loss = ConvergenceLoss::new(rotation.ambiguous.len());
        Imperfect::Partial(rotation, loss)
    }
}

// ---------------------------------------------------------------------------
// RotationConfig — the 1D coupling axis + 1D time axis parameters
// ---------------------------------------------------------------------------

/// Configuration for the rotation primitive.
///
/// Encodes the 1D coupling axis (`coupling_k`) and 1D time axis (`dt`) per Alex
/// 2026-09-02 5-axis decomposition. Omega scale seeds vertex-natural-frequencies
/// with mild spread to admit non-trivial phase evolution. All four are rulers.
#[derive(Clone, Debug)]
pub struct RotationConfig {
    /// Kuramoto coupling strength K. Typical range [0.5, 2.0]. Higher K → faster
    /// synchronization; lower K → slower or non-synchronizing dynamics.
    pub coupling_k: f64,
    /// Time step per tick. Typical range [0.01, 0.1]. Smaller dt → finer temporal
    /// resolution; larger dt → faster progression per tick.
    pub dt: f64,
    /// Natural-frequency scale. Vertex i receives omega_i = omega_scale * (i * 0.1 + 1.0).
    /// Provides mild frequency spread across vertices.
    pub omega_scale: f64,
    /// The grain content is read at.
    pub grain: Grain,
}

impl Default for RotationConfig {
    fn default() -> Self {
        Self {
            coupling_k: 1.0,
            dt: 0.05,
            omega_scale: 0.3,
            grain: Grain::default(),
        }
    }
}

// ---------------------------------------------------------------------------
// infer_via_rotation — the terminal-form primitive
// ---------------------------------------------------------------------------

/// **Inference IS rotation observed through shared substrate.**
///
/// Two-loop case (Phase 1): compose `loops[0]` and `loops[1]` via kleinos ring-and-hub
/// → wedge-at-K_3 basepoint emergent (the shared NOW). Assign Kuramoto phase to each
/// vertex of composed graph. Rotate for at most `ticks` steps via phase_lock (Kuramoto
/// integration), through [`spin`]. Per tick:
///
/// 1. Advance Kuramoto phase (spin the wheel by one dt)
/// 2. the apparatus read across from Anna Jakobs's 2012 thesis observes composed graph Fiedler λ₂ at K_3 basepoint (LAPACK
///    dsyev via FLANG; reading the composed graph's spectrum doesn't change the phases)
/// 3. Compute Kuramoto order parameter r ∈ [0, 1] (coupling-coherence readout)
/// 4. Foerster gauge check: Fiedler λ₂ must monotone-non-decrease across ticks
///    (topology cannot narrow — Rice-safe substrate-scale-invariance per Rec #92)
/// 5. Address the gauge-fixed state under `config.grain` and deposit it at the tick
/// 6. Stop on consecutive equality (`Fixed`), a recurrence (`Cycle`), or the budget
///
/// Returns [`spin`]'s shape. `Failure`: fewer than two loops, kleinos refused,
/// phase_lock FFI failed, or the Foerster gauge was violated; the loss is the tick.
///
/// N-loop case (Phase 2+): reserved for iterative wedge-at-K_n composition.
pub fn infer_via_rotation(
    loops: &[Graph],
    ticks: usize,
    config: RotationConfig,
) -> Imperfect<Rotation, Red, ConvergenceLoss> {
    // Phase 1 requires exactly 2 loops (kleinos K_2→K_3 base case).
    // Phase 2+ will iterate N loops per wedge-at-K_n.
    if loops.len() < 2 {
        return Imperfect::Failure(
            Transparency::single(
                Property::Sovereignty,
                PropertyVerdict::Fail(Diagnostic::new(
                    "infer_via_rotation: requires at least 2 loops for kleinos K_2→K_3 compose (Phase 1 base case)",
                )),
            ),
            ConvergenceLoss::zero(),
        );
    }

    // Compose loops via kleinos: wedge-at-K_3 basepoint (Alex 2026-09-02 metaphor:
    // the shared NOW where all loops rotate through). Phase 1 uses first two loops.
    let composed = match kleinos(&loops[0], &loops[1]) {
        Imperfect::Success(c) => c,
        Imperfect::Partial(c, _loss) => c,
        Imperfect::Failure(red, loss) => return Imperfect::Failure(red, loss),
    };

    // the apparatus read across from Anna Jakobs's 2012 thesis: cached Fiedler λ₂ at composed graph basepoint.
    // Per Phase 1 (topology fixed during rotation), Fiedler is CONSTANT per tick.
    // This IS the substrate-honest observation: the shape of the wedge doesn't
    // change while the phases evolve. Phase 2+ topology-evolving rotation would
    // exercise the Foerster gauge check meaningfully.
    let fiedler = *composed.fiedler();
    let fiedler_at_basepoint = fiedler.value();
    let substrate = composed.oid();

    // Initialize Kuramoto phases + natural frequencies.
    // Phase 1: seed deterministically from vertex indices (canonical BTreeSet
    // order). Vertex i gets phase 2πi/n and omega omega_scale*(i*0.1 + 1.0).
    let n = composed.vertices().count();
    let mut phases: Vec<f64> = (0..n).map(|i| 2.0 * PI * (i as f64) / (n as f64)).collect();
    let omegas: Vec<f64> = (0..n)
        .map(|i| config.omega_scale * ((i as f64) * 0.1 + 1.0))
        .collect();
    let mut previous_fiedler: f64 = fiedler_at_basepoint;

    spin(ticks, &config.grain, |Tick(t)| {
        let tick = t as usize;
        // Advance Kuramoto phase-lock by one dt step.
        let (new_phases, order_r) = phase_lock(&phases, &omegas, config.coupling_k, 1, config.dt)
            .map_err(|info| {
            Transparency::single(
                Property::FiedlerRise,
                PropertyVerdict::Fail(Diagnostic::new(format!(
                    "infer_via_rotation: phase_lock FFI failure at tick {} (info={})",
                    tick, info
                ))),
            )
        })?;
        phases = new_phases;

        // Foerster gauge: Fiedler λ₂ monotone-non-decrease across ticks.
        // Phase 1: trivially holds (topology fixed). Guardrail for Phase 2+
        // topology-evolving rotation.
        if fiedler_at_basepoint < previous_fiedler {
            return Err(Transparency::single(
                Property::FiedlerRise,
                PropertyVerdict::Fail(Diagnostic::new(format!(
                    "infer_via_rotation: Foerster gauge violation at tick {}: fiedler={:.6} < previous={:.6}",
                    tick, fiedler_at_basepoint, previous_fiedler
                ))),
            ));
        }
        previous_fiedler = fiedler_at_basepoint;

        Ok(Observation {
            phases: phases.clone(),
            order_r,
            substrate: substrate.clone(),
            fiedler_lambda_2: fiedler,
            transparency: Transparency::Clear,
        })
    })
}

// ---------------------------------------------------------------------------
// Phase 1 minimum tests (in-module smoke)
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::spectral::graph_from_edges;

    /// Zero ticks observe nothing, so the orbit is unknown: a budget stop,
    /// `Partial`, with no deposits. (Was `Success(vec![])` while a run was a
    /// fixed number of ticks with no stop test.)
    #[test]
    fn rotation_zero_ticks_is_an_empty_budget_stop() {
        let a = graph_from_edges(&[(0, 1), (1, 2)]);
        let b = graph_from_edges(&[(3, 4), (4, 5)]);
        let result = infer_via_rotation(&[a, b], 0, RotationConfig::default());
        match result {
            Imperfect::Partial(rot, loss) => {
                assert!(rot.deposits.is_empty() && rot.splinters.is_empty());
                assert_eq!(rot.stop, Stop::Budget { window: 0 });
                assert!(loss.is_zero());
            }
            other => panic!("expected Partial empty, got {:?}", other),
        }
    }

    #[test]
    fn rotation_single_loop_returns_red() {
        let a = graph_from_edges(&[(0, 1)]);
        let result = infer_via_rotation(&[a], 5, RotationConfig::default());
        match result {
            Imperfect::Failure(_red, _loss) => {}
            other => panic!("expected Red for single-loop input, got {:?}", other),
        }
    }

    /// One deposit per tick until a stop; ten ticks of the default config
    /// are still moving (the phases haven't locked), so all ten are distinct
    /// splinters and the stop is the budget. Each deposit's tick is its index.
    #[test]
    fn rotation_two_disjoint_paths_deposits_one_splinter_per_tick() {
        let a = graph_from_edges(&[(0, 1), (1, 2)]);
        let b = graph_from_edges(&[(3, 4), (4, 5)]);
        let ticks = 10;
        let result = infer_via_rotation(&[a, b], ticks, RotationConfig::default());
        match result {
            Imperfect::Partial(rot, _) => {
                assert_eq!(rot.stop, Stop::Budget { window: ticks });
                assert_eq!(rot.deposits.len(), ticks, "one deposit per tick");
                assert_eq!(rot.splinters.len(), ticks, "still moving: all distinct");
                for (i, d) in rot.deposits.iter().enumerate() {
                    assert_eq!(d.tick, Tick(i as u64), "deposit tick == index");
                }
                for s in &rot.splinters {
                    assert!(
                        s.fiedler_lambda_2().value() > 0.0,
                        "Fiedler must be positive for connected composed graph"
                    );
                    assert!(
                        s.order_r >= 0.0 && s.order_r <= 1.0,
                        "Kuramoto order parameter in [0, 1]; got {}",
                        s.order_r
                    );
                }
            }
            other => panic!(
                "expected a budget stop over {} ticks, got {:?}",
                ticks, other
            ),
        }
    }

    #[test]
    fn rotation_deterministic_splinter_oids() {
        let a = graph_from_edges(&[(0, 1), (1, 2)]);
        let b = graph_from_edges(&[(3, 4), (4, 5)]);
        let cfg = RotationConfig::default();
        let result_1 = infer_via_rotation(&[a.clone(), b.clone()], 5, cfg.clone());
        let result_2 = infer_via_rotation(&[a, b], 5, cfg);
        match (result_1, result_2) {
            (Imperfect::Partial(r1, _), Imperfect::Partial(r2, _)) => {
                assert_eq!(
                    r1.deposits, r2.deposits,
                    "deterministic splinter oids per Rec #82"
                );
            }
            other => panic!("expected two budget-stopped rotations, got {:?}", other),
        }
    }
}
