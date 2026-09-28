//! S1 (Mara constrained-inference-floor.md) and the standing question as a type.
//!
//! A search over seeds never returns Success: sampling cannot settle a
//! universal claim. A clean run is Partial, carrying its coverage. A failure
//! keeps its buffer, and the buffer replays to the failing value.
//!
//! Coverage is a Progression of takes under Polyphony: distinct takes commute
//! (the first chord is the set of distinct runs), repeats stack (height is
//! the worst repeat). Coverage has no time, so it lives on the commutative
//! rung of the ladder.

#![cfg(feature = "bundle")]

use prismqueer::flux::pillar::{search, Arbitrary, Sample};
use terni::Imperfect;

fn seeds(values: impl IntoIterator<Item = u64>) -> Vec<Vec<u8>> {
    values.into_iter().map(|v| v.to_le_bytes().to_vec()).collect()
}

#[test]
fn a_clean_run_is_never_success_it_reports_coverage() {
    match search::<u64>(seeds(0..64), |_| true) {
        Imperfect::Partial((), coverage) => {
            assert_eq!(coverage.len(), Some(64), "every run is counted");
            assert_eq!(coverage.height(), Some(1), "64 distinct takes commute into one chord");
        }
        other => panic!("a clean search must be Partial, got {other:?}"),
    }
}

#[test]
fn repeated_takes_are_counted_not_merged() {
    match search::<u64>(seeds([5, 5, 9, 5]), |_| true) {
        Imperfect::Partial((), coverage) => {
            assert_eq!(coverage.len(), Some(4));
            assert_eq!(coverage.foata().unwrap()[0].len(), 2, "two distinct takes");
            assert_eq!(coverage.height(), Some(3), "take 5 was repeated three times");
        }
        other => panic!("expected Partial, got {other:?}"),
    }
}

#[test]
fn a_failure_keeps_its_buffer_and_the_buffer_replays() {
    match search::<u64>(seeds(0..64), |x| x % 7 != 3) {
        Imperfect::Failure(counterexample, coverage) => {
            let replayed = u64::arbitrary(&mut Sample::from_bytes(counterexample.buffer.clone()));
            assert_eq!(replayed, 3, "the kept buffer must replay to the failing value");
            assert_eq!(coverage.len(), Some(4), "coverage counts the runs up to the failure");
        }
        other => panic!("expected Failure, got {other:?}"),
    }
}

#[test]
fn search_is_deterministic() {
    let a = search::<u64>(seeds(0..32), |x| x % 11 != 10);
    let b = search::<u64>(seeds(0..32), |x| x % 11 != 10);
    assert_eq!(format!("{a:?}"), format!("{b:?}"));
}
