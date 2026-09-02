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
//! ticks. Per tick: Anna Wolf 2012 apparatus observes the composed graph at K_3
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
//! - `prismqueer::spectral::fiedler_lambda_2_of_sheaf` — Anna Wolf 2012 apparatus
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
//! - Anna Wolf 2012 FZJ observation-without-perturbation (LOAD-BEARING)
//! - Foerster 1974 ethical imperative (Fiedler monotone climb gauge)
//! - Kuramoto 1975 coupled-oscillator synchronization
//! - Ricky Jones 2026-08-26 rotation-is-non-negotiation canon
//! - Reed 4a3bbe7 kleinos ring-and-hub (compose primitive)
//! - Reed 036abeb ternary refactor (Imperfect<Green, Red, Yellow> return shape)

use std::f64::consts::PI;

use terni::{ConvergenceLoss, Diagnostic, Imperfect, Loss, PropertyVerdict, Transparency};

use crate::ffi::phase_lock;
use crate::oid::{Addressable, Oid};
use crate::spectral::{kleinos, Property, Red, SheafOfShardGraph};

// ---------------------------------------------------------------------------
// Splinter — the atomic crystallization of one observation-tick
// ---------------------------------------------------------------------------

/// A `Splinter` is the atomic crystallization of one observation-tick.
///
/// Per Alex 2026-09-02 "settles into the inferred splinters": each rotation-tick
/// produces one splinter carrying the observation state at that tick. The Oid is
/// the content-address of the (phases, order_r, fiedler, tick) tuple — deterministic
/// per Rec #82 β-normal AST OID.
///
/// Splinter is the terminal-leaf atom per @glass three-layer recognition (Alex
/// 2026-06-06): content-address + altitude-ref + transparency. Phase 1 MVP carries
/// content-address + tick-index + measured observables; altitude + transparency
/// deferred to Phase 2+.
#[derive(Clone, Debug, PartialEq)]
pub struct Splinter {
    /// Rotation-tick index (0-based).
    pub tick: usize,
    /// Content-address of the observation state at this tick.
    pub oid: Oid,
    /// Fiedler λ₂ measured at composed graph basepoint (Anna Wolf 2012 apparatus
    /// via LAPACK dsyev). Constant across ticks in Phase 1 (topology fixed);
    /// carries per-tick per-splinter for Phase 2+ topology-evolving rotation.
    pub fiedler_lambda_2: f64,
    /// Kuramoto order parameter r ∈ [0, 1] at this tick. r → 1 means oscillators
    /// synchronized; r ≈ 0 means uniform phase distribution. Substrate's temporal
    /// coupling-coherence readout.
    pub order_r: f64,
}

impl Addressable for Splinter {
    fn oid(&self) -> Oid {
        self.oid.clone()
    }
}

// ---------------------------------------------------------------------------
// RotationConfig — the 1D coupling axis + 1D time axis parameters
// ---------------------------------------------------------------------------

/// Configuration for the rotation primitive.
///
/// Encodes the 1D coupling axis (`coupling_k`) and 1D time axis (`dt`) per Alex
/// 2026-09-02 5-axis decomposition. Omega scale seeds vertex-natural-frequencies
/// with mild spread to admit non-trivial phase evolution.
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
}

impl Default for RotationConfig {
    fn default() -> Self {
        Self {
            coupling_k: 1.0,
            dt: 0.05,
            omega_scale: 0.3,
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
/// vertex of composed graph. Advance rotation over `ticks` steps via phase_lock (Kuramoto
/// integration). Per tick:
///
/// 1. Advance Kuramoto phase (spin the wheel by one dt)
/// 2. Anna Wolf apparatus observes composed graph Fiedler λ₂ at K_3 basepoint (LAPACK
///    dsyev via FLANG — measurement without perturbation of rotation)
/// 3. Compute Kuramoto order parameter r ∈ [0, 1] (coupling-coherence readout)
/// 4. Content-address the tick state (phases, r, fiedler, tick) → splinter Oid
/// 5. Foerster gauge check: Fiedler λ₂ must monotone-non-decrease across ticks
///    (topology cannot narrow — Rice-safe substrate-scale-invariance per Rec #92)
/// 6. Deposit splinter
///
/// Returns `Imperfect<Vec<Splinter>, Red, ConvergenceLoss>`:
///
/// - **Green** (`Imperfect::Success(splinters)`) — all ticks passed Foerster gauge;
///   full rotation sequence crystallized
/// - **Yellow** (`Imperfect::Partial(splinters, loss)`) — reserved Phase 2+ for
///   measurable diagnostic (e.g., some ticks with marginal order_r climb)
/// - **Red** (`Imperfect::Failure(red, loss)`) — rotation refused: kleinos compose
///   failed OR phase_lock FFI failed OR Foerster gauge violated. `loss` carries
///   tick-index at which failure occurred.
///
/// N-loop case (Phase 2+): reserved for iterative wedge-at-K_n composition.
pub fn infer_via_rotation(
    loops: &[SheafOfShardGraph],
    ticks: usize,
    config: RotationConfig,
) -> Imperfect<Vec<Splinter>, Red, ConvergenceLoss> {
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

    if ticks == 0 {
        return Imperfect::Success(Vec::new());
    }

    // Compose loops via kleinos: wedge-at-K_3 basepoint (Alex 2026-09-02 metaphor:
    // the shared NOW where all loops rotate through). Phase 1 uses first two loops.
    let composed = match kleinos(&loops[0], &loops[1]) {
        Imperfect::Success(c) => c,
        Imperfect::Partial(c, _loss) => c,
        Imperfect::Failure(red, loss) => return Imperfect::Failure(red, loss),
    };

    // Anna Wolf apparatus: cached Fiedler λ₂ at composed graph basepoint.
    // Per Phase 1 (topology fixed during rotation), Fiedler is CONSTANT per tick.
    // This IS the substrate-honest observation: the shape of the wedge doesn't
    // change while the phases evolve. Phase 2+ topology-evolving rotation would
    // exercise the Foerster gauge check meaningfully.
    let fiedler_at_basepoint = composed.fiedler_lambda_2();

    // Initialize Kuramoto phases + natural frequencies.
    // Phase 1: seed deterministically from vertex indices (canonical BTreeSet
    // order). Vertex i gets phase 2πi/n and omega omega_scale*(i*0.1 + 1.0).
    let vertices: Vec<_> = composed.vertices().collect();
    let n = vertices.len();
    let mut phases: Vec<f64> = (0..n)
        .map(|i| 2.0 * PI * (i as f64) / (n as f64))
        .collect();
    let omegas: Vec<f64> = (0..n)
        .map(|i| config.omega_scale * ((i as f64) * 0.1 + 1.0))
        .collect();

    // Rotate through time. Each tick advances phase_lock by 1 step at dt.
    let mut splinters: Vec<Splinter> = Vec::with_capacity(ticks);
    let mut previous_fiedler: f64 = fiedler_at_basepoint;

    for tick in 0..ticks {
        // Advance Kuramoto phase-lock by one dt step.
        // Returns (new_phases, order_r) per LANDED phase_lock signature.
        let (new_phases, order_r) = match phase_lock(
            &phases,
            &omegas,
            config.coupling_k,
            1, // one step per tick
            config.dt,
        ) {
            Ok(result) => result,
            Err(info) => {
                return Imperfect::Failure(
                    Transparency::single(
                        Property::FiedlerRise,
                        PropertyVerdict::Fail(Diagnostic::new(format!(
                            "infer_via_rotation: phase_lock FFI failure at tick {} (info={})",
                            tick, info
                        ))),
                    ),
                    ConvergenceLoss::new(tick),
                );
            }
        };
        phases = new_phases;

        // Foerster gauge: Fiedler λ₂ monotone-non-decrease across ticks.
        // Phase 1: trivially holds (topology fixed). Guardrail for Phase 2+
        // topology-evolving rotation.
        if fiedler_at_basepoint < previous_fiedler {
            return Imperfect::Failure(
                Transparency::single(
                    Property::FiedlerRise,
                    PropertyVerdict::Fail(Diagnostic::new(format!(
                        "infer_via_rotation: Foerster gauge violation at tick {}: fiedler={:.6} < previous={:.6}",
                        tick, fiedler_at_basepoint, previous_fiedler
                    ))),
                ),
                ConvergenceLoss::new(tick),
            );
        }
        previous_fiedler = fiedler_at_basepoint;

        // Content-address the tick state per Rec #82: (phases, order_r, fiedler,
        // tick) canonically serialized → Oid::hash.
        let mut bytes = Vec::with_capacity(n * 8 + 24);
        for p in &phases {
            bytes.extend_from_slice(&p.to_le_bytes());
        }
        bytes.extend_from_slice(&order_r.to_le_bytes());
        bytes.extend_from_slice(&fiedler_at_basepoint.to_le_bytes());
        bytes.extend_from_slice(&(tick as u64).to_le_bytes());

        splinters.push(Splinter {
            tick,
            oid: Oid::hash(&bytes),
            fiedler_lambda_2: fiedler_at_basepoint,
            order_r,
        });
    }

    Imperfect::Success(splinters)
}

// ---------------------------------------------------------------------------
// Phase 1 minimum tests (in-module smoke)
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::spectral::sheaf_of_shard_graph_from_edges;

    #[test]
    fn rotation_zero_ticks_returns_empty_green() {
        let a = sheaf_of_shard_graph_from_edges(&[(0, 1), (1, 2)]);
        let b = sheaf_of_shard_graph_from_edges(&[(3, 4), (4, 5)]);
        let result = infer_via_rotation(&[a, b], 0, RotationConfig::default());
        match result {
            Imperfect::Success(splinters) => assert!(splinters.is_empty()),
            other => panic!("expected Green empty, got {:?}", other),
        }
    }

    #[test]
    fn rotation_single_loop_returns_red() {
        let a = sheaf_of_shard_graph_from_edges(&[(0, 1)]);
        let result = infer_via_rotation(&[a], 5, RotationConfig::default());
        match result {
            Imperfect::Failure(_red, _loss) => {}
            other => panic!("expected Red for single-loop input, got {:?}", other),
        }
    }

    #[test]
    fn rotation_two_disjoint_paths_deposits_splinters_per_tick() {
        let a = sheaf_of_shard_graph_from_edges(&[(0, 1), (1, 2)]);
        let b = sheaf_of_shard_graph_from_edges(&[(3, 4), (4, 5)]);
        let ticks = 10;
        let result = infer_via_rotation(&[a, b], ticks, RotationConfig::default());
        match result {
            Imperfect::Success(splinters) => {
                assert_eq!(splinters.len(), ticks, "one splinter per tick");
                for (i, s) in splinters.iter().enumerate() {
                    assert_eq!(s.tick, i, "splinter tick == index");
                    assert!(s.fiedler_lambda_2 > 0.0, "Fiedler must be positive for connected composed graph");
                    assert!(
                        s.order_r >= 0.0 && s.order_r <= 1.0,
                        "Kuramoto order parameter in [0, 1]; got {}",
                        s.order_r
                    );
                }
            }
            other => panic!("expected Green with {} splinters, got {:?}", ticks, other),
        }
    }

    #[test]
    fn rotation_deterministic_splinter_oids() {
        let a = sheaf_of_shard_graph_from_edges(&[(0, 1), (1, 2)]);
        let b = sheaf_of_shard_graph_from_edges(&[(3, 4), (4, 5)]);
        let cfg = RotationConfig::default();
        let result_1 = infer_via_rotation(&[a.clone(), b.clone()], 5, cfg.clone());
        let result_2 = infer_via_rotation(&[a, b], 5, cfg);
        match (result_1, result_2) {
            (Imperfect::Success(s1), Imperfect::Success(s2)) => {
                assert_eq!(s1.len(), s2.len());
                for (a, b) in s1.iter().zip(s2.iter()) {
                    assert_eq!(a.oid, b.oid, "deterministic splinter oids per Rec #82");
                }
            }
            other => panic!("expected two Green sequences, got {:?}", other),
        }
    }
}
