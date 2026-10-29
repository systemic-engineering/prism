//! O3, the first ouroboros closure: prismqueer tests prismqueer, steered by
//! prismqueer.
//!
//! Property under test: the Instrument's kernel has dimension b0 + b1
//! (tonics plus commas). b0 and b1 come from union-find, an instrument
//! outside the loop, so the closure cannot certify itself.
//!
//! Steering: prismqueer's own spectrum chooses the next input. The measure is
//! the quietest non-kernel mode of the Instrument, how close the graph is to a
//! new tonic or comma appearing: the hardest cases for the kernel count.
//!
//! Stop: the search stops moving when the chord of distinct graphs stops
//! growing. A clean search is Partial, never Success.

#![cfg(all(feature = "lapack", feature = "bundle"))]

use std::collections::BTreeSet;

use prismqueer::flux::pillar::{steer, Arbitrary, Budget, Patience, Sample, Stop};
use prismqueer::oid::Addressable;
use prismqueer::spectral::instrument::Instrument;
use prismqueer::spectral::{graph_from_edges, Graph};
use terni::Imperfect;

/// A small graph with every vertex on some edge.
struct Field {
    n: u64,
    edges: Vec<(u64, u64)>,
}

impl Field {
    fn graph(&self) -> Graph {
        graph_from_edges(&self.edges)
    }
}

fn covered(sample: &mut Sample, n: u64) -> Field {
    let mut edges = Vec::new();
    for i in 0..n {
        for j in (i + 1)..n {
            if sample.draw_bool() {
                edges.push((i, j));
            }
        }
    }
    for v in 0..n {
        if !edges.iter().any(|&(a, b)| a == v || b == v) {
            let w = (v + 1) % n;
            edges.push((v.min(w), v.max(w)));
        }
    }
    Field { n, edges }
}

struct Seven(Field);
impl Arbitrary for Seven {
    fn arbitrary(sample: &mut Sample) -> Self {
        Seven(covered(sample, 7))
    }
}

struct Tiny(Field);
impl Arbitrary for Tiny {
    fn arbitrary(sample: &mut Sample) -> Self {
        let n = sample.draw_integer(2, 3) as u64;
        Tiny(covered(sample, n))
    }
}

/// Union-find, outside the loop: (components, independent cycles).
fn betti(f: &Field) -> (usize, usize) {
    let mut parent: Vec<usize> = (0..f.n as usize).collect();
    fn root(p: &mut Vec<usize>, i: usize) -> usize {
        if p[i] == i {
            i
        } else {
            let r = root(p, p[i]);
            p[i] = r;
            r
        }
    }
    let mut edges = BTreeSet::new();
    let mut b0 = f.n as usize;
    for &(u, v) in &f.edges {
        if !edges.insert((u, v)) {
            continue;
        }
        let (ru, rv) = (root(&mut parent, u as usize), root(&mut parent, v as usize));
        if ru != rv {
            parent[ru] = rv;
            b0 -= 1;
        }
    }
    (b0, edges.len() + b0 - f.n as usize)
}

fn kernel_is_tonics_plus_commas(f: &Field) -> bool {
    let (b0, b1) = betti(f);
    Instrument::of(&f.graph())
        .map(|d| d.kernel_dimension() == Imperfect::Success(b0 + b1))
        .unwrap_or(false)
}

/// The quietest non-kernel mode: prismqueer measuring where it is hardest.
fn quietest(f: &Field) -> f64 {
    let d = Instrument::of(&f.graph()).expect("measured");
    let mut modes: Vec<f64> = d.eigenvalues().iter().map(|x| x.abs()).collect();
    modes.sort_by(f64::total_cmp);
    modes
        .get(d.kernel_dimension().ok().expect("an exact count"))
        .copied()
        .unwrap_or(f64::INFINITY)
}

fn seeds(range: std::ops::Range<u64>) -> Vec<Vec<u8>> {
    range.map(|s| s.to_le_bytes().to_vec()).collect()
}

#[test]
fn steering_by_the_spectrum_beats_blind_sampling() {
    let budget = 200;
    let blind = seeds(0..budget)
        .into_iter()
        .map(|s| quietest(&Seven::arbitrary(&mut Sample::from_bytes(s)).0))
        .fold(f64::INFINITY, f64::min);

    let steered = steer::<Seven>(
        seeds(0..8),
        Budget(budget as usize),
        Patience(budget as usize),
        |s| quietest(&s.0),
        |s| kernel_is_tonics_plus_commas(&s.0),
        |s| s.0.graph().oid(),
    );
    match steered {
        Imperfect::Partial(run, coverage) => {
            eprintln!(
                "blind best {blind:.4}; steered best {:.4} after {:?} runs, {:?}",
                run.best_score.unwrap_or(f64::NAN),
                coverage.len(),
                run.stop
            );
            assert!(
                run.best_score.expect("scored") < blind,
                "steered {:?} did not beat blind {blind}",
                run.best_score
            );
            // Pin, for this deterministic configuration: the steered search
            // reaches Fiedler's extremum, the path on 7 vertices, whose
            // quietest mode is sqrt(2 - 2cos(pi/7)) in closed form.
            let path = (2.0 - 2.0 * (std::f64::consts::PI / 7.0).cos()).sqrt();
            assert!(
                (run.best_score.unwrap() - path).abs() < 1e-9,
                "did not reach the path"
            );
        }
        Imperfect::Failure(cx, _) => panic!("kernel != b0 + b1 on {:?}", cx.buffer),
        Imperfect::Success(_) => panic!("a search must never be Success"),
    }
}

#[test]
fn the_search_stops_moving_when_nothing_new_appears() {
    // Every covered graph on 2 or 3 labeled vertices, counted by brute force.
    let mut all = BTreeSet::new();
    for mask in 0u8..8 {
        let e: Vec<(u64, u64)> = [(0, 1), (0, 2), (1, 2)]
            .into_iter()
            .enumerate()
            .filter(|(i, _)| mask & (1 << i) != 0)
            .map(|(_, e)| e)
            .collect();
        if (0..3).all(|v| e.iter().any(|&(a, b)| a == v || b == v)) {
            all.insert(graph_from_edges(&e).oid());
        }
    }
    all.insert(graph_from_edges(&[(0, 1)]).oid());

    match steer::<Tiny>(
        seeds(0..4),
        Budget(10_000),
        Patience(64),
        |t| quietest(&t.0),
        |t| kernel_is_tonics_plus_commas(&t.0),
        |t| t.0.graph().oid(),
    ) {
        Imperfect::Partial(run, coverage) => {
            assert_eq!(
                run.stop,
                Stop::Saturated,
                "it must stop by saturation, not budget"
            );
            let seen = coverage.foata().expect("known")[0].len();
            assert!(
                seen <= all.len(),
                "saw {seen} distinct graphs, only {} exist",
                all.len()
            );
            assert!(coverage.len().unwrap() < 10_000);
        }
        other => panic!("expected a saturated Partial, got {other:?}"),
    }
}

#[test]
fn a_failure_is_kept_and_replays() {
    // A deliberately false claim: no graph has a comma.
    match steer::<Seven>(
        seeds(0..8),
        Budget(200),
        Patience(200),
        |s| quietest(&s.0),
        |s| betti(&s.0).1 == 0,
        |s| s.0.graph().oid(),
    ) {
        Imperfect::Failure(cx, _) => {
            let replayed = Seven::arbitrary(&mut Sample::from_bytes(cx.buffer.clone()));
            assert!(
                betti(&replayed.0).1 > 0,
                "the kept buffer must replay to a graph with a comma"
            );
        }
        other => panic!("expected Failure, got {other:?}"),
    }
}

#[test]
fn steering_is_deterministic() {
    let run = || {
        steer::<Seven>(
            seeds(0..8),
            Budget(64),
            Patience(64),
            |s| quietest(&s.0),
            |s| kernel_is_tonics_plus_commas(&s.0),
            |s| s.0.graph().oid(),
        )
    };
    assert_eq!(format!("{:?}", run()), format!("{:?}", run()));
}
