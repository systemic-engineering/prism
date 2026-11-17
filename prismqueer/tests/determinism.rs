//! Paths the gate used to seed from the clock take their seed from the caller
//! (docs/spec/checks.md, to-fix 1 and 2). Same seed, same result, in this
//! process and in the next one; different seeds, different results.

use prismqueer::flux::pillar::Sample;

fn draws(seed: &[u8]) -> Vec<i64> {
    let mut s = Sample::new(seed);
    (0..8).map(|_| s.draw_integer(0, 1_000_000)).collect()
}

#[test]
fn a_sample_seeded_alike_draws_alike() {
    assert_eq!(draws(b"determinism::alike"), draws(b"determinism::alike"));
}

#[test]
fn samples_seeded_apart_draw_apart() {
    assert_ne!(draws(b"determinism::one"), draws(b"determinism::two"));
}

/// The stream for a seed is fixed across processes and machines, not just
/// within one run: SHA-256 and SplitMix64 are integer-only.
#[test]
fn a_seeded_sample_draws_its_recorded_stream() {
    assert_eq!(
        draws(b"determinism::pinned"),
        [865330, 585579, 60592, 191527, 101925, 583078, 975058, 344211]
    );
}

#[test]
fn a_seeded_sample_starts_empty_at_depth_zero() {
    let s = Sample::new(b"determinism::empty");
    assert_eq!(s.depth(), 0);
    assert!(s.consumed().is_empty());
}

#[cfg(feature = "fate")]
mod fate {
    use prismqueer::fate::{Fate, FEATURE_DIM};

    fn distribution(seed: u64) -> [f64; 5] {
        Fate::excited(seed)
            .tick(&[1.0; FEATURE_DIM])
            .decision
            .distribution
    }

    #[test]
    fn an_excited_fate_seeded_alike_decides_alike() {
        assert_eq!(distribution(17), distribution(17));
    }

    /// The model each seed picks, recorded. Discrete, so it holds across
    /// platforms whose float rounding differs in the last bit.
    #[test]
    fn excited_fates_pick_their_recorded_models() {
        use prismqueer::fate::Model::{Abyss, Cartographer, Explorer, Introject};
        let models: Vec<_> = (0..8u64)
            .map(|seed| Fate::excited(seed).tick(&[1.0; FEATURE_DIM]).decision.model)
            .collect();
        assert_eq!(
            models,
            [
                Abyss,
                Introject,
                Abyss,
                Cartographer,
                Cartographer,
                Explorer,
                Explorer,
                Abyss
            ]
        );
    }

    #[test]
    fn excited_fates_seeded_apart_decide_apart() {
        assert_ne!(distribution(17), distribution(18));
    }
}
