//! T2 (Mara nonlinear-inference.md): a Progression is a lawful Loss.
//!
//! combine is concatenation in the trace monoid M(E, I), plus one adjoined
//! absorbing element, `total()`: the history was lost. A lost history has no
//! known length or shape, so its measures are `None`, not 0.

use std::collections::BTreeSet;

use prismqueer::oid::{Addressable, Oid};
use prismqueer::time::{Polyphony, Progression, Touches};
use terni::Loss;

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
struct Step {
    name: &'static str,
    reads: BTreeSet<Oid>,
    writes: BTreeSet<Oid>,
}

impl Addressable for Step {
    fn oid(&self) -> Oid {
        Oid::hash(self.name.as_bytes())
    }
}

impl Touches for Step {
    fn reads(&self) -> &BTreeSet<Oid> {
        &self.reads
    }
    fn writes(&self) -> &BTreeSet<Oid> {
        &self.writes
    }
}

fn step(name: &'static str, reads: &[&str], writes: &[&str]) -> Step {
    let set = |s: &[&str]| s.iter().map(|x| Oid::hash(x.as_bytes())).collect();
    Step {
        name,
        reads: set(reads),
        writes: set(writes),
    }
}

/// Every word of length 0..=2 over an alphabet with both independent and
/// dependent pairs.
fn words() -> Vec<Vec<Step>> {
    let alphabet = [
        step("a", &[], &["x"]),
        step("b", &[], &["y"]),
        step("c", &["x"], &["z"]),
        step("d", &["y"], &["y"]),
    ];
    let mut out = vec![vec![]];
    for p in &alphabet {
        out.push(vec![p.clone()]);
        for q in &alphabet {
            out.push(vec![p.clone(), q.clone()]);
        }
    }
    out
}

type P = Progression<Step, Polyphony>;

#[test]
fn combine_is_concatenation_in_the_trace_monoid() {
    let ws = words();
    for u in &ws {
        for v in &ws {
            let joined: Vec<Step> = u.iter().chain(v).cloned().collect();
            assert!(
                P::from_word(u.clone()).combine(P::from_word(v.clone())) == P::from_word(joined),
                "combine is not a homomorphism from words"
            );
        }
    }
}

#[test]
fn progression_is_a_monoid_with_an_adjoined_total() {
    let ps: Vec<P> = words().into_iter().map(P::from_word).collect();
    assert!(P::zero().is_zero() && !P::total().is_zero());
    for a in &ps {
        assert!(
            P::zero().combine(a.clone()) == *a && a.clone().combine(P::zero()) == *a,
            "identity"
        );
        assert!(
            P::total().combine(a.clone()) == P::total(),
            "total absorbs from the left"
        );
        assert!(
            a.clone().combine(P::total()) == P::total(),
            "total absorbs from the right"
        );
        assert!(*a != P::total(), "a known history is never the lost one");
        for b in &ps {
            for c in &ps {
                let l = a.clone().combine(b.clone()).combine(c.clone());
                let r = a.clone().combine(b.clone().combine(c.clone()));
                assert!(l == r, "combine is not associative");
            }
        }
    }
}

#[test]
fn a_lost_history_has_no_measures() {
    let lost = P::total();
    assert!(lost.is_total());
    assert_eq!(lost.len(), None);
    assert_eq!(lost.height(), None);
    assert!(lost.foata().is_none());
    assert!(
        lost.oid() != P::zero().oid(),
        "lost and empty must not share an address"
    );
    assert_eq!(P::zero().len(), Some(0));
}
