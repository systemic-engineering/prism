//! prism-006 ask 3: `Band`, the measurement value type mirror's Resolution
//! rulers wrap. Exact thresholds, a declared scale law, three verdicts.
//! Geometry here; governance (address, approval, record) in mirror.

#![cfg(feature = "lapack")]

use prismqueer::band::{zeros, Band, Fork, ScaleLaw, Verdict};
use prismqueer::field::Ratio;

fn tenth_power(e: u32) -> Ratio {
    Ratio::of(1, 10u64.pow(e))
}

#[test]
fn an_absolute_band_reads_three_ways() {
    let b = Band::new(tenth_power(12), tenth_power(6), ScaleLaw::Absolute).unwrap();
    assert_eq!(b.read(0.0, 1.0), Verdict::Zero);
    assert_eq!(b.read(1e-9, 1.0), Verdict::DontKnow);
    assert_eq!(b.read(-1e-3, 1.0), Verdict::NonZero, "magnitude, not sign");
    assert_eq!(
        b.read(1e-9, 1e6),
        Verdict::DontKnow,
        "absolute ignores the scale"
    );
}

#[test]
fn a_times_scale_band_scales_with_the_operator_and_its_floor() {
    let b = Band::new(
        tenth_power(12),
        tenth_power(6),
        ScaleLaw::TimesScale {
            floor: Ratio::of(1, 1),
        },
    )
    .unwrap();
    assert_eq!(
        b.read(1e-4, 1000.0),
        Verdict::DontKnow,
        "1e-6 × 1000 = 1e-3 > 1e-4"
    );
    assert_eq!(
        b.read(1e-10, 1000.0),
        Verdict::Zero,
        "1e-12 × 1000 = 1e-9 > 1e-10"
    );
    assert_eq!(
        b.read(1e-10, 0.001),
        Verdict::DontKnow,
        "below the floor the scale is 1"
    );
}

#[test]
fn an_epsilon_scaled_band_is_lapacks_error_rule() {
    // 16 · ε · dim · max(scale, 1), with no don't-know region: settle's rule.
    let b = Band::new(
        Ratio::of(1, 1),
        Ratio::of(1, 1),
        ScaleLaw::EpsilonScaled { k: 16, dim: 6 },
    )
    .unwrap();
    let error = 16.0 * f64::EPSILON * 6.0;
    assert_eq!(b.read(error * 0.5, 1.0), Verdict::Zero);
    assert_eq!(b.read(error * 2.0, 1.0), Verdict::NonZero);
}

#[test]
fn a_band_with_its_thresholds_crossed_is_refused() {
    assert!(Band::new(tenth_power(6), tenth_power(12), ScaleLaw::Absolute).is_err());
}

#[test]
fn bands_compose_coarsest_within_a_law_and_fork_across_laws() {
    let a = Band::new(tenth_power(12), tenth_power(6), ScaleLaw::Absolute).unwrap();
    let b = Band::new(tenth_power(9), tenth_power(9), ScaleLaw::Absolute).unwrap();
    let c = a.coarsest(&b).expect("same law");
    assert_eq!(
        c,
        Band::new(tenth_power(12), tenth_power(6), ScaleLaw::Absolute).unwrap(),
        "(min zero_below, max nonzero_from)"
    );
    let other = Band::new(
        tenth_power(12),
        tenth_power(6),
        ScaleLaw::TimesScale {
            floor: Ratio::of(1, 1),
        },
    )
    .unwrap();
    assert_eq!(
        a.coarsest(&other),
        Err(Fork),
        "different scale laws: a fork, never a silent pick"
    );
}

/// mirror RU11: the commutator band and tension's band disagree at 1e-9,
/// and with different scale laws their composite is a fork.
///
/// Taut counted six zeros. Since `c7de979` the Instrument certifies its
/// kernel with settle's rule, so today there are five, and those two are one.
#[test]
fn the_zeros_are_distinct_declared_bands() {
    assert_eq!(
        zeros::instrument(6),
        zeros::settle(6),
        "one rule since c7de979"
    );
    let all = [
        zeros::settle(6),
        zeros::commutator(),
        zeros::spectrum(),
        zeros::tension(),
        zeros::coincidence(),
    ];
    for (i, a) in all.iter().enumerate() {
        for b in &all[i + 1..] {
            assert_ne!(a, b, "each zero is its own ruler");
        }
    }
    assert_eq!(zeros::commutator().read(1e-9, 1.0), Verdict::DontKnow);
    assert_eq!(zeros::tension().read(1e-9, 1.0), Verdict::Zero);
    assert_eq!(zeros::commutator().coarsest(&zeros::tension()), Err(Fork));
}
