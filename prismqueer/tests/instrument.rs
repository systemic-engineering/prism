//! The Hodge–Dirac operator of a graph: D = δ + δ* on C⁰ ⊕ C¹.
//!
//! Frame Cohomology is ker D = H⁰ ⊕ H¹, where the recursion stops moving.
//! It is the output of settling, not the space the question acts on (on
//! cohomology D is identically zero). The question of an assertion s is δs,
//! the disagreement along every edge. Heat e^{-tD²} settles; rotation e^{itD}
//! alternates assertion and question without settling. Both are functions of
//! one D, so they commute.
//!
//! Every expected value here comes from an instrument independent of the
//! code under test: union-find for components, a hand-built Laplacian for the
//! spectrum, closed-form projections for the flows.

#![cfg(feature = "lapack")]

use std::collections::BTreeMap;

use prismqueer::spectral::instrument::{Angle, Cochain, Instrument, Solo};
use prismqueer::spectral::settle::Time;
use prismqueer::spectral::{graph_from_edges, Graph};

fn instrument(edges: &[(u64, u64)]) -> (Graph, Instrument) {
    let g = graph_from_edges(edges);
    let d = Instrument::of(&g).expect("LAPACK must measure a small integer graph");
    (g, d)
}

fn components(g: &Graph) -> usize {
    let mut parent: BTreeMap<u64, u64> = g.vertices().map(|v| (v, v)).collect();
    fn root(p: &mut BTreeMap<u64, u64>, v: u64) -> u64 {
        let up = p[&v];
        if up == v {
            v
        } else {
            let r = root(p, up);
            p.insert(v, r);
            r
        }
    }
    for (u, v) in g.edges() {
        let (ru, rv) = (root(&mut parent, u), root(&mut parent, v));
        parent.insert(ru, rv);
    }
    let vs: Vec<u64> = g.vertices().collect();
    let mut roots: Vec<u64> = vs.into_iter().map(|v| root(&mut parent, v)).collect();
    roots.sort();
    roots.dedup();
    roots.len()
}

fn close(a: f64, b: f64) -> bool {
    (a - b).abs() < 1e-9
}

#[test]
fn dirac_is_self_adjoint_and_odd() {
    let (g, d) = instrument(&[(0, 1), (1, 2), (0, 2), (2, 3)]);
    let (n0, n1) = (g.vertex_count(), g.edges().count());
    let n = n0 + n1;
    let m = d.matrix();
    assert_eq!(m.len(), n * n);
    for i in 0..n {
        for j in 0..n {
            assert_eq!(m[i * n + j], m[j * n + i], "not self-adjoint at ({i}, {j})");
            let same_degree = (i < n0) == (j < n0);
            if same_degree {
                assert_eq!(
                    m[i * n + j],
                    0.0,
                    "D must map vertices to edges and back, not to itself"
                );
            }
        }
    }
}

#[test]
fn kernel_counts_components_and_cycles() {
    let cases: [&[(u64, u64)]; 5] = [
        &[(0, 1), (1, 2), (2, 3)],                         // path: b0 = 1, b1 = 0
        &[(0, 1), (1, 2), (0, 2)],                         // triangle: 1, 1
        &[(0, 1), (0, 2), (0, 3), (1, 2), (1, 3), (2, 3)], // K4: 1, 3
        &[(0, 1), (1, 2), (0, 2), (3, 4), (4, 5), (3, 5)], // two triangles: 2, 2
        &[(0, 1), (1, 2), (0, 2), (2, 3), (3, 4), (2, 4)], // bowtie: 1, 2
    ];
    for edges in cases {
        let (g, d) = instrument(edges);
        let b0 = components(&g);
        let b1 = g.edges().count() + b0 - g.vertex_count();
        assert_eq!(
            d.kernel_dimension(),
            terni::Imperfect::Success(b0 + b1),
            "dim ker D != b0 + b1 for {edges:?}"
        );
    }
}

#[test]
fn vertex_and_edge_laplacians_share_their_nonzero_spectrum() {
    let edges = [(0, 1), (1, 2), (0, 2), (2, 3), (3, 4)];
    let (g, d) = instrument(&edges);

    // Spectrum of D is symmetric: D is odd.
    let mut values: Vec<f64> = d.eigenvalues().to_vec();
    values.sort_by(f64::total_cmp);
    let mut mirrored: Vec<f64> = values.iter().map(|x| -x).collect();
    mirrored.sort_by(f64::total_cmp);
    for (a, b) in values.iter().zip(&mirrored) {
        assert!(close(*a, *b), "spectrum of D is not symmetric: {values:?}");
    }

    // Squares of the positive eigenvalues are the nonzero eigenvalues of L,
    // computed here from a Laplacian built by hand.
    let n = g.vertex_count();
    let mut l = vec![0.0; n * n];
    for (u, v) in edges {
        let (u, v) = (u as usize, v as usize);
        l[u * n + u] += 1.0;
        l[v * n + v] += 1.0;
        l[u * n + v] -= 1.0;
        l[v * n + u] -= 1.0;
    }
    let (mut lap, _) = prismqueer::ffi::eigensystem(n, &l).expect("Laplacian eigensystem");
    lap.retain(|x| x.abs() > 1e-9);
    lap.sort_by(f64::total_cmp);
    let mut squared: Vec<f64> = values
        .iter()
        .filter(|x| **x > 1e-9)
        .map(|x| x * x)
        .collect();
    squared.sort_by(f64::total_cmp);
    assert_eq!(
        squared.len(),
        lap.len(),
        "δ must intertwine every nonzero mode"
    );
    for (a, b) in squared.iter().zip(&lap) {
        assert!(close(*a, *b), "D² and L disagree: {squared:?} vs {lap:?}");
    }
}

#[test]
fn a_settled_assertion_asks_nothing() {
    let (_, d) = instrument(&[(0, 1), (1, 2), (0, 2)]);

    let settled = Cochain::new([(0, 2.0), (1, 2.0), (2, 2.0)], []);
    let q = d.question(&settled).expect("on the complex");
    assert!(q.norm() < 1e-12, "a constant assertion raised a question");

    let unsettled = Cochain::new([(0, 0.0), (1, 1.0), (2, 3.0)], []);
    let q = d.question(&unsettled).expect("on the complex");
    assert!(
        close(q.interval((0, 1)).unwrap(), 1.0),
        "δs on (0,1) is s(1) - s(0)"
    );
    assert!(close(q.interval((1, 2)).unwrap(), 2.0));
    assert!(close(q.interval((0, 2)).unwrap(), 3.0));
    for v in 0..3 {
        assert!(
            close(q.pitch(v).unwrap_or(0.0), 0.0),
            "an assertion's question lives on edges only"
        );
    }
}

#[test]
fn decay_keeps_the_comma() {
    // Four-cycle. The circulation around it is a harmonic 1-cochain: a
    // disagreement that is not the question of any assertion.
    let (_, d) = instrument(&[(0, 1), (1, 2), (2, 3), (0, 3)]);
    let circulation = [((0, 1), 1.0), ((1, 2), 1.0), ((2, 3), 1.0), ((0, 3), -1.0)];

    let x = Cochain::new([(0, 4.0), (1, 0.0), (2, 0.0), (3, 0.0)], circulation);
    let settled = d.decay(Time(60.0), &x).expect("on the complex");
    for v in 0..4 {
        assert!(
            close(settled.pitch(v).unwrap(), 1.0),
            "vertex part must settle to the mean"
        );
    }
    for (e, h) in circulation {
        assert!(
            close(settled.interval(e).unwrap(), h),
            "the circulation must survive untouched"
        );
    }

    // Control: a disagreement on one edge only is partly answerable. It
    // settles to its projection onto the circulation, <e, h>/<h, h> = 1/4.
    let one_edge = Cochain::new([], [((0, 1), 1.0)]);
    let settled = d.decay(Time(60.0), &one_edge).expect("on the complex");
    for (e, h) in circulation {
        assert!(
            close(settled.interval(e).unwrap(), h / 4.0),
            "edge {e:?} did not settle to h/4"
        );
    }
}

#[test]
fn decay_and_ring_commute_and_ring_never_settles() {
    let (_, d) = instrument(&[(0, 1), (1, 2), (0, 2), (2, 3)]);
    let x = Cochain::new([(0, 1.0), (1, -2.0), (2, 0.5), (3, 3.0)], [((1, 2), 0.7)]);

    let rotated = d.ring(Angle(0.9), &x).expect("on the complex");
    let energy = rotated.real.norm().powi(2) + rotated.imag.norm().powi(2);
    assert!(
        close(energy, x.norm().powi(2)),
        "rotation must be isometric"
    );

    let heated = d.decay(Time(0.3), &x).expect("on the complex");
    assert!(
        heated.norm() < x.norm(),
        "heat must lose energy off the kernel"
    );

    let heat_then_rotate = d.ring(Angle(0.9), &heated).expect("on the complex");
    let rotate_then_heat_re = d.decay(Time(0.3), &rotated.real).expect("on the complex");
    let rotate_then_heat_im = d.decay(Time(0.3), &rotated.imag).expect("on the complex");
    assert!(
        heat_then_rotate.real.distance(&rotate_then_heat_re) < 1e-9
            && heat_then_rotate.imag.distance(&rotate_then_heat_im) < 1e-9,
        "heat and rotation under one D must commute"
    );
}

// --- Bleed: attending and questioning do not commute at the edge of attention ---

fn boundary(g: &Graph, solo: &[u64]) -> std::collections::BTreeSet<(u64, u64)> {
    g.edges()
        .filter(|(u, v)| solo.contains(u) != solo.contains(v))
        .collect()
}

#[test]
fn bleed_lives_exactly_on_the_solo_boundary() {
    let cases: [(&[(u64, u64)], &[u64]); 3] = [
        (&[(0, 1), (1, 2), (2, 3)], &[0, 1]),
        (
            &[(0, 1), (1, 2), (0, 2), (2, 3), (3, 4), (2, 4)],
            &[0, 1, 2],
        ),
        (&[(0, 1), (0, 2), (0, 3), (1, 2), (1, 3), (2, 3)], &[1]),
    ];
    for (edges, region) in cases {
        let (g, d) = instrument(edges);
        let bleed = d
            .bleed(&Solo::new(region.iter().copied()))
            .expect("on the complex");
        assert_eq!(
            bleed.edges(),
            boundary(&g, region),
            "bleed off the boundary for {edges:?}"
        );
        for x in bleed.matrix() {
            assert!(
                [0.0, 0.5, -0.5].contains(x),
                "bleed entry {x} is not exactly 0 or ±1/2"
            );
        }
    }
}

#[test]
fn soloing_everything_or_nothing_does_not_bleed() {
    let (_, d) = instrument(&[(0, 1), (1, 2), (0, 2), (2, 3)]);
    for region in [vec![], vec![0, 1, 2, 3]] {
        let bleed = d.bleed(&Solo::new(region)).expect("on the complex");
        assert!(
            bleed.edges().is_empty(),
            "a constant solo is in the center and must commute"
        );
    }
}

#[test]
fn attend_then_ask_is_not_ask_then_attend() {
    // A settled assertion asks nothing. Solo it first and it asks exactly at
    // the edge of attention; ask first and there is nothing left to solo.
    let (g, d) = instrument(&[(0, 1), (1, 2), (2, 3), (0, 3)]);
    let region = [0, 1];
    let solo = Solo::new(region);
    let settled = Cochain::new([(0, 1.0), (1, 1.0), (2, 1.0), (3, 1.0)], []);

    let attend_then_ask = d.question(&d.solo(&solo, &settled).unwrap()).unwrap();
    let ask_then_attend = d.solo(&solo, &d.question(&settled).unwrap()).unwrap();

    assert!(
        ask_then_attend.norm() < 1e-12,
        "asking a settled assertion raised a question"
    );
    for (u, v) in g.edges() {
        let q = attend_then_ask.interval((u, v)).unwrap();
        if boundary(&g, &region).contains(&(u, v)) {
            assert!(
                (q.abs() - 1.0).abs() < 1e-12,
                "edge ({u},{v}) crosses attention and must ask"
            );
        } else {
            assert!(
                q.abs() < 1e-12,
                "edge ({u},{v}) is inside or outside attention and must not ask"
            );
        }
    }
}

#[test]
fn a_solo_off_the_complex_fails_closed() {
    let (_, d) = instrument(&[(0, 1), (1, 2)]);
    assert!(
        d.bleed(&Solo::new([7])).is_err(),
        "unknown vertex in the solo"
    );
}

#[test]
fn a_cochain_off_the_complex_fails_closed() {
    let (_, d) = instrument(&[(0, 1), (1, 2)]);
    assert!(
        d.decay(Time(1.0), &Cochain::new([(9, 1.0)], [])).is_err(),
        "unknown vertex"
    );
    assert!(
        d.question(&Cochain::new([], [((0, 2), 1.0)])).is_err(),
        "unknown edge"
    );
}
