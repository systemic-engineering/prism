//! T15, one Splinter (prism-004 decision 7; mirror spec
//! `docs/specs/mirror-fixpoint-parser.md` §12 row 7, §13 T15; math
//! `2026-10-28-mara-mirror-fixpoint-parser-math-foundation.md` Def. 1.6,
//! Prop. 1.7, Def. 1.8, Prop. 1.9).
//!
//! - The tick is a deposit coordinate, never in the address (Prop. 1.7).
//! - Content is quotiented by common phase rotation (Def. 1.8).
//! - Stop on consecutive equality; a recurrence at distance p > 1 is
//!   `cycle(p)`, never fixed; a budget cut is its own outcome (Prop. 1.9).
//! - The altitude is in the address (Cor. 1.3); transparency is not (§1.2).

#![cfg(feature = "lapack")]

use std::f64::consts::{PI, TAU};

use prismqueer::band::Verdict;
use prismqueer::brain::Tick;
use prismqueer::oid::{Oid, Scope};
use prismqueer::spectral::graph_from_edges;
use prismqueer::spectral::rotation::{
    infer_via_rotation, spin, Altitude, Grain, Observation, Period, Rotation, RotationConfig,
    Splinter, Stop, ROTATION,
};
use prismqueer::spectral::Property;
use terni::{Diagnostic, Imperfect, Loss, PropertyVerdict, Transparency};

fn observation(phases: &[f64]) -> Observation {
    Observation {
        phases: phases.to_vec(),
        order_r: 0.25,
        fiedler_lambda_2: 1.5,
        transparency: Transparency::Clear,
    }
}

fn rotated(phases: &[f64], theta: f64) -> Vec<f64> {
    phases.iter().map(|p| p + theta).collect()
}

fn rotation_altitude() -> Altitude {
    Altitude::new(ROTATION).expect("the rotation altitude is well formed")
}

/// The address of an observation, asserting it was read clear of every
/// rounding boundary (so an equality below is not luck at the grain's edge).
fn addr(obs: &Observation, grain: &Grain) -> Oid {
    let reading = Splinter::observe(&rotation_altitude(), obs, grain);
    assert_eq!(
        reading.boundary,
        Verdict::NonZero,
        "read clear of the grain's boundaries"
    );
    reading.splinter.oid
}

fn settled(
    result: Imperfect<Rotation, prismqueer::spectral::Red, terni::ConvergenceLoss>,
) -> Rotation {
    match result {
        Imperfect::Success(r) => r,
        other => panic!("expected Success, got {other:?}"),
    }
}

fn unsettled(
    result: Imperfect<Rotation, prismqueer::spectral::Red, terni::ConvergenceLoss>,
) -> Rotation {
    match result {
        Imperfect::Partial(r, _) => r,
        other => panic!("expected Partial, got {other:?}"),
    }
}

const A: [f64; 4] = [0.3, 1.1, 2.9, 5.0];
const B: [f64; 4] = [0.3, 2.1, 2.9, 5.0];
const C: [f64; 4] = [0.3, 1.1, 3.9, 5.0];

// ---------------------------------------------------------------------------
// 1. The tick leaves the address
// ---------------------------------------------------------------------------

#[test]
fn the_same_content_at_two_ticks_is_one_splinter_deposited_twice() {
    let rot = settled(spin(5, &Grain::default(), |_| Ok(observation(&A))));
    assert_eq!(
        rot.deposits.len(),
        2,
        "tick 0, then tick 1 repeats it and stops"
    );
    assert_eq!(rot.deposits[0].addr, rot.deposits[1].addr, "one address");
    assert_eq!(
        rot.deposits[0].tick,
        Tick(0),
        "the tick is read from the deposit"
    );
    assert_eq!(
        rot.deposits[1].tick,
        Tick(1),
        "the tick is read from the deposit"
    );
    assert_eq!(rot.splinters.len(), 1, "one splinter, hash-consed");
    assert_eq!(rot.ticks_of(&rot.deposits[0].addr), vec![Tick(0), Tick(1)]);
    assert_eq!(rot.stop, Stop::Fixed { at: Tick(0) });
}

/// The preimage, from the spec's bytes (§5.0): `"splinter\0" ‖ τ ‖
/// enc(content)`, `τ = UTF-8(altitude) ‖ 0x00 ‖ 0x00` (an engine altitude,
/// root flag), `u64` big-endian, hashed with `Oid::hash` in the public scope.
/// Content: `u64(grain bits) ‖ u64(n) ‖ u64(k₁) … u64(k_{n−1}) ‖ u64(k_r) ‖
/// u64(λ₂ bits)`. No tick anywhere.
#[test]
fn the_address_is_the_documented_preimage_with_no_tick() {
    let grain = Grain::new(2).expect("two bits: quarter turns");
    let obs = Observation {
        phases: vec![0.0, PI],
        order_r: 0.0,
        fiedler_lambda_2: 1.0,
        transparency: Transparency::Clear,
    };
    let mut content = Vec::new();
    for word in [2u64, 2, 2, 0, 1.0f64.to_bits()] {
        // bits, n, k₁ = π as two quarter turns, k_r = 0, λ₂
        content.extend_from_slice(&word.to_be_bytes());
    }
    let mut preimage = b"splinter\0".to_vec();
    preimage.extend_from_slice(b"@spectral/rotation\0\0");
    preimage.extend_from_slice(&(content.len() as u64).to_be_bytes());
    preimage.extend_from_slice(&content);
    assert_eq!(addr(&obs, &grain), Oid::hash(&Scope::public(), &preimage));
}

// ---------------------------------------------------------------------------
// 2. Quotient by common phase rotation (Def. 1.8)
// ---------------------------------------------------------------------------

#[test]
fn a_global_phase_rotation_gives_the_same_address() {
    let grain = Grain::default();
    let base = addr(&observation(&A), &grain);
    for theta in [1.234, -0.7, TAU, 3.0 * TAU + 0.7, -5.0 * TAU - 2.0] {
        assert_eq!(
            addr(&observation(&rotated(&A, theta)), &grain),
            base,
            "rotation by {theta} is free"
        );
    }
}

#[test]
fn a_non_global_change_gives_a_different_address() {
    let grain = Grain::default();
    let base = addr(&observation(&A), &grain);
    assert_ne!(addr(&observation(&B), &grain), base, "one phase moved");
    assert_ne!(addr(&observation(&C), &grain), base, "another phase moved");
    let mut r = observation(&A);
    r.order_r = 0.5;
    assert_ne!(addr(&r, &grain), base, "r is content");
    let mut l = observation(&A);
    l.fiedler_lambda_2 = 2.0;
    assert_ne!(addr(&l, &grain), base, "λ₂ is content");
}

#[test]
fn the_grain_is_part_of_the_address() {
    let coarse = Grain::new(8).expect("in range");
    let fine = Grain::new(16).expect("in range");
    assert_ne!(
        addr(&observation(&[0.0, PI]), &coarse),
        addr(&observation(&[0.0, PI]), &fine)
    );
}

#[test]
fn the_grain_is_a_declared_ruler_in_range() {
    assert!(Grain::new(0).is_none(), "no zero-bit grain");
    assert!(Grain::new(53).is_none(), "finer than an f64 mantissa");
    assert_eq!(Grain::new(52).map(|g| g.bits()), Some(52));
}

/// A phase exactly on a rounding boundary is ambiguous under the ruler: the
/// reading says so, and a run that deposited it is `Partial`, never silent.
#[test]
fn a_phase_on_the_grains_boundary_is_read_as_ambiguous() {
    let grain = Grain::new(2).expect("quarter turns");
    let edge = observation(&[0.0, PI / 4.0]); // half a quarter turn
    let reading = Splinter::observe(&rotation_altitude(), &edge, &grain);
    assert_eq!(reading.boundary, Verdict::Zero, "on the boundary");
    match spin(3, &grain, |_| Ok(edge.clone())) {
        Imperfect::Partial(rot, loss) => {
            assert_eq!(rot.ambiguous, vec![Tick(0), Tick(1)]);
            assert_eq!(loss.steps(), 2, "one per ambiguous deposit");
        }
        other => panic!("expected Partial, got {other:?}"),
    }
}

/// Absolute phases drift every tick and never repeat; the gauge-fixed content
/// does. Def. 1.8's whole point.
#[test]
fn a_drifting_common_phase_is_a_fixed_point() {
    let rot = settled(spin(10, &Grain::default(), |Tick(t)| {
        Ok(observation(&rotated(&A, 0.37 * t as f64)))
    }));
    assert_eq!(rot.stop, Stop::Fixed { at: Tick(0) });
}

// ---------------------------------------------------------------------------
// 3. Stopping (Prop. 1.9)
// ---------------------------------------------------------------------------

#[test]
fn a_two_cycle_reports_cycle_two_never_fixed() {
    let rot = settled(spin(10, &Grain::default(), |Tick(t)| {
        Ok(observation(if t % 2 == 0 { &A } else { &B }))
    }));
    let two = Period::new(2).expect("a period of two");
    assert_eq!(
        rot.stop,
        Stop::Cycle {
            from: Tick(0),
            period: two
        }
    );
    assert!(
        !matches!(rot.stop, Stop::Fixed { .. }),
        "A, B, A is not fixed"
    );
    assert_eq!(rot.deposits.len(), 3, "A at 0, B at 1, A again at 2");
    assert_eq!(rot.splinters.len(), 2);
}

#[test]
fn a_cycle_after_a_tail_reports_where_it_starts_and_its_period() {
    // X, Y, then A, B, C, A: μ = 2, λ = 3 (spec §4.3 F5).
    let x = [0.0, 0.5, 0.0, 0.0];
    let y = [0.0, 0.0, 0.5, 0.0];
    let seq = [x, y, A, B, C];
    let rot = settled(spin(20, &Grain::default(), |Tick(t)| {
        let t = t as usize;
        Ok(observation(&seq[if t < 2 { t } else { 2 + (t - 2) % 3 }]))
    }));
    let three = Period::new(3).expect("a period of three");
    assert_eq!(
        rot.stop,
        Stop::Cycle {
            from: Tick(2),
            period: three
        }
    );
    assert_eq!(rot.deposits.len(), 6);
}

#[test]
fn a_genuine_fixed_point_reports_fixed() {
    // A, B, C, C: consecutive equality at ticks 2 and 3.
    let seq = [A, B, C];
    let rot = settled(spin(20, &Grain::default(), |Tick(t)| {
        Ok(observation(&seq[(t as usize).min(2)]))
    }));
    assert_eq!(rot.stop, Stop::Fixed { at: Tick(2) });
    assert_eq!(rot.deposits.len(), 4);
    assert_eq!(rot.ticks_of(&rot.deposits[3].addr), vec![Tick(2), Tick(3)]);
}

#[test]
fn a_budget_cut_reports_budget() {
    let rot = unsettled(spin(4, &Grain::default(), |Tick(t)| {
        Ok(observation(&[0.0, 0.1 * (t + 1) as f64]))
    }));
    assert_eq!(rot.stop, Stop::Budget { window: 4 });
    assert_eq!(rot.deposits.len(), 4);
    assert!(rot.ambiguous.is_empty(), "unknown, not ambiguous");
}

#[test]
fn a_zero_budget_observes_nothing_and_knows_nothing() {
    let rot = unsettled(spin(0, &Grain::default(), |_| Ok(observation(&A))));
    assert_eq!(rot.stop, Stop::Budget { window: 0 });
    assert!(rot.deposits.is_empty());
}

#[test]
fn a_period_is_at_least_two() {
    assert!(Period::new(0).is_none());
    assert!(
        Period::new(1).is_none(),
        "a period of one is Fixed, never Cycle"
    );
    assert_eq!(Period::new(5).map(|p| p.get()), Some(5));
}

#[test]
fn a_failing_step_is_red_at_its_tick() {
    let red = Transparency::single(
        Property::Sovereignty,
        PropertyVerdict::Fail(Diagnostic::new("step refused")),
    );
    let result = spin(10, &Grain::default(), |Tick(t)| {
        if t < 3 {
            Ok(observation(&[0.0, t as f64]))
        } else {
            Err(red.clone())
        }
    });
    match result {
        Imperfect::Failure(_, loss) => assert_eq!(loss.steps(), 3),
        other => panic!("expected Failure, got {other:?}"),
    }
}

/// The real rotation: Kuramoto on the kleinos wedge of two paths, at the
/// default config and grain, settles. Seam's simulation became stationary
/// gauge-fixed (Def. 1.8, COMPUTED); here it is the stop test that says so.
#[test]
fn the_kuramoto_rotation_reaches_a_fixed_point() {
    let a = graph_from_edges(&[(0, 1), (1, 2)]);
    let b = graph_from_edges(&[(3, 4), (4, 5)]);
    let rot = settled(infer_via_rotation(
        &[a, b],
        20_000,
        RotationConfig::default(),
    ));
    let Stop::Fixed { at } = rot.stop else {
        panic!("expected a fixed point, got {:?}", rot.stop);
    };
    assert_eq!(rot.deposits.last().map(|d| d.tick), Some(Tick(at.0 + 1)));
    for s in &rot.splinters {
        assert!(s.fiedler_lambda_2 > 0.0, "connected wedge");
        assert!((0.0..=1.0).contains(&s.order_r));
    }
}

// ---------------------------------------------------------------------------
// 4. Altitude and transparency
// ---------------------------------------------------------------------------

/// mirror spec §4 newtypes: starts with `@`, contains no NUL, non-empty path
/// segments.
#[test]
fn an_altitude_is_an_at_ref_with_no_nul_and_no_empty_segment() {
    assert_eq!(rotation_altitude().as_str(), "@spectral/rotation");
    for ok in ["@mirror", "@parse/item", "@spectral/rotation"] {
        assert!(Altitude::new(ok).is_some(), "{ok}");
    }
    for bad in ["", "spectral/rotation", "@", "@a//b", "@a/", "@/a", "@a\0b"] {
        assert!(Altitude::new(bad).is_none(), "{bad:?}");
    }
}

#[test]
fn rotation_splinters_are_at_the_rotation_altitude() {
    let rot = settled(spin(5, &Grain::default(), |_| Ok(observation(&A))));
    assert!(rot
        .splinters
        .iter()
        .all(|s| s.altitude == rotation_altitude()));
}

/// Cor. 1.3: the altitude is inside the hashed encoding, so the same content
/// at two altitudes is two addresses.
#[test]
fn the_same_content_at_another_altitude_has_another_address() {
    let grain = Grain::default();
    let other = Altitude::new("@spectral/elsewhere").expect("well formed");
    let here = Splinter::observe(&rotation_altitude(), &observation(&A), &grain);
    let there = Splinter::observe(&other, &observation(&A), &grain);
    assert_ne!(here.splinter.oid, there.splinter.oid);
    assert_eq!(there.splinter.altitude, other);
}

/// §1.2: transparency is not in the address. The verdict about a thing is
/// not the thing.
#[test]
fn transparency_is_carried_but_not_addressed() {
    let grain = Grain::default();
    let clear = observation(&A);
    let mut opaque = observation(&A);
    opaque.transparency = Transparency::single(
        Property::FiedlerRise,
        PropertyVerdict::Fail(Diagnostic::new("a verdict, not content")),
    );
    let c = Splinter::observe(&rotation_altitude(), &clear, &grain).splinter;
    let o = Splinter::observe(&rotation_altitude(), &opaque, &grain).splinter;
    assert_eq!(c.oid, o.oid, "same address");
    assert_eq!(c.transparency, Transparency::Clear);
    assert_eq!(
        o.transparency, opaque.transparency,
        "carried beside the address"
    );
}

/// The Kuramoto run's transparency is the Foerster gauge verdict (shard.rs,
/// Move 3): a violated gauge is Red, so every deposited splinter is Clear.
#[test]
fn the_kuramoto_rotation_deposits_clear_splinters() {
    let a = graph_from_edges(&[(0, 1), (1, 2)]);
    let b = graph_from_edges(&[(3, 4), (4, 5)]);
    let rot = match infer_via_rotation(&[a, b], 50, RotationConfig::default()) {
        Imperfect::Success(r) | Imperfect::Partial(r, _) => r,
        other => panic!("expected a rotation, got {other:?}"),
    };
    assert!(rot
        .splinters
        .iter()
        .all(|s| s.transparency == Transparency::Clear));
    assert!(rot
        .splinters
        .iter()
        .all(|s| s.altitude == rotation_altitude()));
}

#[test]
fn loss_is_zero_for_a_clean_budget_cut() {
    // A budget cut is Partial because the orbit is unknown within the
    // window; nothing was read ambiguously, so the loss is zero.
    match spin(2, &Grain::default(), |Tick(t)| {
        Ok(observation(&[0.0, 0.1 * (t + 1) as f64]))
    }) {
        Imperfect::Partial(_, loss) => assert!(loss.is_zero()),
        other => panic!("expected Partial, got {other:?}"),
    }
}
