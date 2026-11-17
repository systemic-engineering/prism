//! S3*: the sampled audit of a verdict cache, stated exactly (docs/spec/audit.md).
//!
//! Every number here is a rational, compared exactly; no float enters. The
//! hand checks are in the doc: C(8,3)/C(10,3) = 56/120 = 7/15, and
//! (19/20)^58 > 1/20 ≥ (19/20)^59.

use num_bigint::BigInt;
use num_rational::BigRational;
use prismqueer::audit::{
    audit, bounds_rate, miss_probability, smallest_sample, Bound, Finding, Refusal, Sampled,
};
use terni::{Imperfect, Loss};

fn q(n: i64, d: i64) -> BigRational {
    BigRational::new(BigInt::from(n), BigInt::from(d))
}

// ── miss_probability: C(N−k, n) / C(N, n), without replacement ─────────────

#[test]
fn one_bad_item_in_ten_one_draw_misses_with_nine_tenths() {
    assert_eq!(miss_probability(10, 1, 1), Ok(q(9, 10)));
}

#[test]
fn two_bad_in_ten_three_draws_misses_with_seven_fifteenths() {
    // C(8,3) / C(10,3) = 56 / 120 = 7/15. With replacement it would be
    // (8/10)^3 = 64/125: this vector tells the two apart.
    assert_eq!(miss_probability(10, 2, 3), Ok(q(7, 15)));
}

#[test]
fn a_five_percent_sample_catches_one_bad_item_with_five_percent() {
    // 50 of 1000, one item wrong: missed with C(999,50)/C(1000,50) = 950/1000.
    assert_eq!(miss_probability(1000, 1, 50), Ok(q(19, 20)));
}

#[test]
fn edge_cases_are_exact() {
    assert_eq!(
        miss_probability(10, 0, 4),
        Ok(q(1, 1)),
        "nothing bad: never caught"
    );
    assert_eq!(
        miss_probability(10, 3, 0),
        Ok(q(1, 1)),
        "no sample: never caught"
    );
    assert_eq!(
        miss_probability(10, 3, 10),
        Ok(q(0, 1)),
        "the whole population"
    );
    assert_eq!(
        miss_probability(10, 3, 8),
        Ok(q(0, 1)),
        "n > N − k: always caught"
    );
    assert_eq!(miss_probability(0, 0, 0), Ok(q(1, 1)));
}

#[test]
fn impossible_inputs_are_refused() {
    assert_eq!(
        miss_probability(10, 1, 11),
        Err(Refusal::SampleExceedsPopulation {
            sample: 11,
            population: 10
        })
    );
    assert_eq!(
        miss_probability(10, 11, 1),
        Err(Refusal::BadExceedsPopulation {
            bad: 11,
            population: 10
        })
    );
}

// ── bounds_rate: (1 − p)^n ≤ 1 − c, exactly ────────────────────────────────

#[test]
fn rule_of_three_neighbourhood_59_clean_bound_5_percent_at_95() {
    // (19/20)^58 ≈ 0.05105 > 1/20; (19/20)^59 ≈ 0.04849 ≤ 1/20.
    assert_eq!(bounds_rate(58, &q(1, 20), &q(19, 20)), Ok(false));
    assert_eq!(bounds_rate(59, &q(1, 20), &q(19, 20)), Ok(true));
    assert_eq!(smallest_sample(&q(1, 20), &q(19, 20)), Ok(59));
}

#[test]
fn fifty_clean_samples_bound_the_rate_near_5_8_percent_at_95() {
    // 1 − (1/20)^(1/50) = 0.058155…: the bound holds at 5.82 %, not at 5.81 %.
    assert_eq!(bounds_rate(50, &q(581, 10_000), &q(19, 20)), Ok(false));
    assert_eq!(bounds_rate(50, &q(582, 10_000), &q(19, 20)), Ok(true));
    assert_eq!(bounds_rate(50, &q(58, 1000), &q(19, 20)), Ok(false));
    assert_eq!(bounds_rate(50, &q(59, 1000), &q(19, 20)), Ok(true));
}

#[test]
fn the_boundary_itself_counts_as_bounded() {
    // (1 − 1/2)^1 = 1/2 = 1 − 1/2: equality satisfies ≤.
    assert_eq!(bounds_rate(1, &q(1, 2), &q(1, 2)), Ok(true));
    assert_eq!(smallest_sample(&q(1, 2), &q(1, 2)), Ok(1));
    // (1/2)^3 = 1/8 = 1 − 7/8.
    assert_eq!(smallest_sample(&q(1, 2), &q(7, 8)), Ok(3));
}

#[test]
fn smallest_sample_is_least() {
    for (p, c) in [
        (q(1, 20), q(19, 20)),
        (q(1, 100), q(99, 100)),
        (q(3, 10), q(9, 10)),
    ] {
        let n = smallest_sample(&p, &c).unwrap();
        assert_eq!(bounds_rate(n, &p, &c), Ok(true));
        assert_eq!(bounds_rate(n - 1, &p, &c), Ok(false));
    }
}

#[test]
fn a_claim_outside_the_open_unit_interval_is_refused() {
    for (p, c) in [
        (q(0, 1), q(1, 2)),
        (q(1, 1), q(1, 2)),
        (q(1, 2), q(0, 1)),
        (q(1, 2), q(1, 1)),
    ] {
        assert_eq!(bounds_rate(10, &p, &c), Err(Refusal::NotAClaim));
        assert!(smallest_sample(&p, &c).is_err());
    }
}

// ── the gate's verdict over a sample ───────────────────────────────────────

#[test]
fn a_clean_sample_is_partial_and_names_its_bound() {
    match audit(1000, 50, 0, &q(582, 10_000), &q(19, 20)) {
        Imperfect::Partial((), loss) => assert_eq!(
            loss.bounds(),
            &[Bound {
                population: 1000,
                sampled: 50,
                rate: q(582, 10_000),
                confidence: q(19, 20),
                established: true,
            }]
        ),
        other => panic!("a sampled audit is never Success: {other:?}"),
    }
}

#[test]
fn a_clean_sample_too_small_for_its_claim_is_partial_and_says_so() {
    match audit(1000, 50, 0, &q(581, 10_000), &q(19, 20)) {
        Imperfect::Partial((), loss) => assert!(!loss.bounds()[0].established),
        other => panic!("expected Partial, got {other:?}"),
    }
}

#[test]
fn only_a_full_recompute_is_success() {
    assert!(matches!(
        audit(40, 40, 0, &q(1, 20), &q(19, 20)),
        Imperfect::Success(())
    ));
}

#[test]
fn a_drifted_item_in_the_sample_is_a_failure() {
    match audit(1000, 50, 2, &q(1, 20), &q(19, 20)) {
        Imperfect::Failure(Finding::Drift { found, sampled }, _) => {
            assert_eq!((found, sampled), (2, 50));
        }
        other => panic!("expected Failure(Drift), got {other:?}"),
    }
}

#[test]
fn composing_audits_unions_their_bounds_lawfully() {
    let bound = |sampled| Bound {
        population: 1000,
        sampled,
        rate: q(1, 20),
        confidence: q(19, 20),
        established: sampled >= 59,
    };
    let (a, b, c) = (
        Sampled::of(bound(50)),
        Sampled::of(bound(60)),
        Sampled::of(bound(70)),
    );
    assert_eq!(
        a.clone().combine(b.clone()).combine(c.clone()),
        a.clone().combine(b.clone().combine(c.clone())),
        "associative"
    );
    assert_eq!(Sampled::zero().combine(a.clone()), a, "left identity");
    assert_eq!(a.clone().combine(Sampled::zero()), a, "right identity");
    assert_eq!(
        b.clone().combine(a.clone()),
        a.clone().combine(b.clone()),
        "a set"
    );
    assert_eq!(a.clone().combine(a.clone()), a, "each bound once");
    assert!(
        Sampled::total().combine(a.clone()).is_total(),
        "total absorbs"
    );
    assert!(Sampled::zero().is_zero() && !a.is_zero());
}

#[test]
fn an_impossible_audit_is_refused_not_judged() {
    assert!(matches!(
        audit(10, 11, 0, &q(1, 20), &q(19, 20)),
        Imperfect::Failure(Finding::Refused(Refusal::SampleExceedsPopulation { .. }), _)
    ));
    assert!(matches!(
        audit(10, 5, 6, &q(1, 20), &q(19, 20)),
        Imperfect::Failure(Finding::Refused(Refusal::DriftExceedsSample { .. }), _)
    ));
}
