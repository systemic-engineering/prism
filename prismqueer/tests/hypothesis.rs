//! Frames, their composite, and the hypothesis over it (prism-021; Mara
//! 54c88f72 §4.3, §5, §11 "For prism"; probe c012e29f P5).
//!
//! Three frames A, B, C on ℚ² with transports A→B = I, B→C = I and
//! C→A = R, the rational rotation (3/5, 4/5). The composite's
//! interpretation is a function of the key (frames, relations, faces):
//! agreement (a basis of H⁰), the holonomy of every located loop (exact,
//! with its cycle certificate and the tree its basis is read against),
//! curvature on declared faces, provenance. The hypothesis is what no
//! recompute establishes: per located loop, the unranked closures that would
//! remove it (fill, twist, absorb, share, bias).
//!
//! Falsifiers F1–F6, the unit, refusals, and no metric anywhere on the path.

#![cfg(feature = "lapack")]

use std::collections::BTreeSet;

use prismqueer::field::Group;
use prismqueer::gl::{Gl, GlError};
use prismqueer::hypothesis::{
    compose, hypothesize, Closure, Composite, CompositionError, Face, FaceDraft, Frame, Hypothesis,
    Obstruction, Relation, RelationDraft,
};
use prismqueer::oid::Oid;
use prismqueer::room::{coboundary, Coefficient, Entry, Q, Z};
use terni::Imperfect;

// --- Fixtures ----------------------------------------------------------------

fn q(n: i64, d: i64) -> Q {
    Q::new(n, d).expect("non-zero denominator")
}

fn gl(rows: [[(i64, i64); 2]; 2]) -> Gl<Q> {
    Gl::new(
        rows.iter()
            .map(|r| r.iter().map(|&(n, d)| q(n, d)).collect())
            .collect(),
    )
    .expect("invertible")
}

fn eye() -> Gl<Q> {
    gl([[(1, 1), (0, 1)], [(0, 1), (1, 1)]])
}

/// The rotation (3/5, 4/5): [[3/5, −4/5], [4/5, 3/5]].
fn rotation() -> Gl<Q> {
    gl([[(3, 5), (-4, 5)], [(4, 5), (3, 5)]])
}

fn shear() -> Gl<Q> {
    gl([[(1, 1), (1, 1)], [(0, 1), (1, 1)]])
}

fn frame(name: &str) -> Frame {
    Frame::new(Oid::new(name), 2)
}

fn relation(from: &Frame, to: &Frame, t: Gl<Q>, by: &str) -> Relation<Q> {
    RelationDraft::new(from.oid().clone(), to.oid().clone(), t).seal(Oid::new(by))
}

struct Triangle {
    a: Frame,
    b: Frame,
    c: Frame,
    frames: Vec<Frame>,
    relations: Vec<Relation<Q>>,
}

/// A → B, B → C, C → A, signed by their sources.
fn triangle(ab: Gl<Q>, bc: Gl<Q>, ca: Gl<Q>) -> Triangle {
    let (a, b, c) = (frame("A"), frame("B"), frame("C"));
    let relations = vec![
        relation(&a, &b, ab, "A"),
        relation(&b, &c, bc, "B"),
        relation(&c, &a, ca, "C"),
    ];
    Triangle {
        frames: vec![a.clone(), b.clone(), c.clone()],
        a,
        b,
        c,
        relations,
    }
}

fn composite(t: &Triangle, faces: &[Face]) -> Composite<Q> {
    compose(&t.frames, &t.relations, faces).expect("well-typed")
}

fn oids<'a>(xs: impl IntoIterator<Item = &'a Oid>) -> BTreeSet<Oid> {
    xs.into_iter().cloned().collect()
}

fn trace(g: &Gl<Q>, n: usize) -> Q {
    let rows = g.rows(n);
    (0..n).fold(Q::from(0), |s, i| s.add(&rows[i][i]))
}

// --- The main test -------------------------------------------------------------

#[test]
fn three_frames_with_a_rotation_agree_on_nothing_and_hold_one_located_loop() {
    let t = triangle(eye(), eye(), rotation());
    let c = composite(&t, &[]);

    // Agreement: a basis of H⁰, empty.
    assert_eq!(c.agreement().len(), 0, "agreement dimension 0");

    // Exactly one loop, A → B → C → A in its canonical orientation.
    let loops = c.holonomy().loops();
    assert_eq!(loops.len(), 1, "exactly one located loop");
    let l = &loops[0];
    assert_eq!(l.frames().len(), 4, "a closed walk on three frames");
    assert_eq!(l.frames().first(), l.frames().last());
    assert_eq!(oids(l.frames()), oids([t.a.oid(), t.b.oid(), t.c.oid()]));
    let all: Vec<Oid> = t.relations.iter().map(|r| r.oid().clone()).collect();
    assert_eq!(oids(l.relations()), oids(&all));

    // Its holonomy is R or R⁻¹ exactly (the base path is the identity), and
    // R⁻¹ = D·R·D⁻¹ with D = diag(1, −1): conjugate to R either way.
    let h = l.holonomy();
    let d = gl([[(1, 1), (0, 1)], [(0, 1), (-1, 1)]]);
    let r = rotation();
    let conj = d.inverse().compose(&r).compose(&d); // D·R·D⁻¹ in path order
    assert_eq!(conj, r.inverse());
    assert!(
        *h == r || *h == conj,
        "holonomy conjugate to R: {:?}",
        h.rows(2)
    );
    assert!(!h.is_identity());

    // Its cycle certificate: a 1-chain z with ∂z = 0, ±1 on the loop's
    // three relations, 0 elsewhere.
    let z = l.certificate();
    let boundary = coboundary(c.graph()).adjoint().apply(z).expect("one C¹");
    assert!(boundary.iter().all(|x| *x == Z::from(0)), "∂z = 0");
    let support: BTreeSet<Oid> = z
        .iter()
        .enumerate()
        .filter(|(_, x)| **x != Z::from(0))
        .map(|(i, x)| {
            assert!(*x == Z::from(1) || *x == Z::from(-1));
            c.edges()[i].clone()
        })
        .collect();
    assert_eq!(support, oids(l.relations()));

    // A basis is claimed with its tree named: two tree relations, the third
    // closes the loop.
    assert_eq!(c.holonomy().tree().len(), 2);
    assert!(c
        .holonomy()
        .tree()
        .iter()
        .all(|e| oids(l.relations()).contains(e)));

    // The hypothesis: grounded on the composite, the unranked menu.
    let Imperfect::Partial(hyp, loss) = hypothesize(&t.frames, &t.relations, &[]) else {
        panic!("a curved composite is Partial");
    };
    assert_eq!(hyp.ground(), c.oid());
    assert_eq!(loss, BTreeSet::from([Obstruction::Loop(l.id().clone())]));
    assert_eq!(hyp.alternatives().len(), 1);
    let menu = &hyp.alternatives()[l.id()];
    assert_eq!(menu.len(), 11, "Fill, 3 Twist, 3 Absorb, Share, 3 Bias");

    let rels = oids(l.relations());
    let frames = oids(l.frames());
    let twists: BTreeSet<Oid> = menu
        .iter()
        .filter_map(|x| match x {
            Closure::Twist(e) => Some(e.clone()),
            _ => None,
        })
        .collect();
    let biases: BTreeSet<Oid> = menu
        .iter()
        .filter_map(|x| match x {
            Closure::Bias(e) => Some(e.clone()),
            _ => None,
        })
        .collect();
    let absorbs: BTreeSet<Oid> = menu
        .iter()
        .filter_map(|x| match x {
            Closure::Absorb(f) => Some(f.clone()),
            _ => None,
        })
        .collect();
    let fills: Vec<_> = menu
        .iter()
        .filter_map(|x| match x {
            Closure::Fill(p) => Some(p),
            _ => None,
        })
        .collect();
    let shares: Vec<_> = menu
        .iter()
        .filter_map(|x| match x {
            Closure::Share(b) => Some(b),
            _ => None,
        })
        .collect();
    assert_eq!(twists, rels, "a Twist on each of the 3 relations");
    assert_eq!(biases, rels, "a Bias on each of the 3 relations");
    assert_eq!(absorbs, frames, "an Absorb by each of the 3 frames");
    assert_eq!(fills.len(), 1);
    assert_eq!(
        fills[0].frames(),
        l.frames(),
        "Fill proposes the loop's face"
    );
    assert_eq!(shares.len(), 1);
    assert_eq!(oids(shares[0].frames()), frames, "Share: all three sign");
}

/// No metric is referenced anywhere on the path: the signature takes frames,
/// relations and faces only (checked by the compiler), and the code of
/// `gl` and `hypothesis` names no metric, form, weight or float.
#[test]
fn no_metric_is_referenced_on_the_composition_path() {
    type Out = Imperfect<Hypothesis, CompositionError, BTreeSet<Obstruction>>;
    type Hypothesize = fn(&[Frame], &[Relation<Q>], &[Face]) -> Out;
    type Compose = fn(&[Frame], &[Relation<Q>], &[Face]) -> Result<Composite<Q>, CompositionError>;
    let _: Hypothesize = hypothesize::<Q>;
    let _: Compose = compose::<Q>;

    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
    for file in ["gl.rs", "hypothesis.rs"] {
        let text = std::fs::read_to_string(root.join(file)).expect("readable");
        for (n, line) in text.lines().enumerate() {
            let code = line.split("//").next().unwrap_or("");
            let lower = code.to_lowercase();
            for word in [
                "metri", "f64", "f32", "form", "weight", "norm", "inner", "gram",
            ] {
                assert!(
                    !lower.contains(word),
                    "{file}:{}: `{word}` on the composition path: {line}",
                    n + 1
                );
            }
        }
    }
}

// --- F1: flat --------------------------------------------------------------------

#[test]
fn f1_with_the_identity_for_the_rotation_it_is_flat_and_agrees_in_two_dimensions() {
    let t = triangle(eye(), eye(), eye());
    let c = composite(&t, &[]);
    assert_eq!(c.agreement().len(), 2, "agreement dimension 2");
    assert!(c.holonomy().loops().is_empty());
    assert!(c.curvature().is_empty());
    // Every basis section is consistent across every relation.
    for section in c.agreement() {
        let at = |f: &Frame| {
            section
                .iter()
                .find(|i| i.frame() == f.oid())
                .expect("one reading per frame")
                .value()
                .to_vec()
        };
        for (from, to) in [(&t.a, &t.b), (&t.b, &t.c), (&t.c, &t.a)] {
            assert_eq!(at(to), eye().apply(&at(from)));
        }
    }
    let Imperfect::Success(h) = hypothesize(&t.frames, &t.relations, &[]) else {
        panic!("flat is Success");
    };
    assert_eq!(h.ground(), c.oid());
    assert!(h.alternatives().is_empty());
}

// --- F2: gauge ---------------------------------------------------------------------

#[test]
fn f2_a_gauge_transform_at_b_keeps_the_loop_and_its_charpoly_bytes() {
    let base = triangle(eye(), eye(), rotation());
    let c0 = composite(&base, &[]);
    let l0 = &c0.holonomy().loops()[0];
    let p0 = l0.holonomy().charpoly(2).encode();

    let gauges = [
        gl([[(2, 1), (1, 1)], [(1, 1), (1, 1)]]),
        gl([[(0, 1), (-1, 1)], [(1, 1), (0, 1)]]),
        gl([[(7, 3), (0, 1)], [(5, 2), (-1, 9)]]),
        rotation(),
    ];
    for g in gauges {
        // B's basis changes by g: A→B becomes g after it, B→C g⁻¹ before it.
        let t = triangle(eye().compose(&g), g.inverse().compose(&eye()), rotation());
        let c = composite(&t, &[]);
        assert_eq!(c.agreement().len(), 0);
        let loops = c.holonomy().loops();
        assert_eq!(loops.len(), 1);
        assert_eq!(loops[0].frames(), l0.frames(), "the same location");
        assert_eq!(
            loops[0].holonomy().charpoly(2).encode(),
            p0,
            "the holonomy charpoly, byte-identical"
        );
    }
}

// --- F3: trace-blind ---------------------------------------------------------------

#[test]
fn f3_a_shear_holonomy_is_a_loop_although_its_trace_is_the_identitys() {
    let t = triangle(eye(), eye(), shear());
    let c = composite(&t, &[]);
    let loops = c.holonomy().loops();
    assert_eq!(loops.len(), 1, "the shear is detected");
    let h = loops[0].holonomy();
    assert_eq!(trace(h, 2), trace(&eye(), 2), "trace 2, as the identity's");
    assert_eq!(
        h.charpoly(2),
        eye().charpoly(2),
        "(x − 1)², as the identity's: the charpoly is blind here too"
    );
    assert_ne!(*h, eye(), "compared exactly, it isn't the identity");
    // One fixed direction survives the shear: agreement dimension 1.
    assert_eq!(c.agreement().len(), 1);
}

// --- F4: refused at construction ------------------------------------------------------

#[test]
fn f4_a_singular_transport_is_refused_at_construction() {
    let singular = Gl::new(vec![vec![q(1, 1), q(2, 1)], vec![q(2, 1), q(4, 1)]]);
    assert_eq!(singular, Err(GlError::Singular));
    let zero = Gl::new(vec![vec![q(0, 1)]]);
    assert_eq!(zero, Err(GlError::Singular));
    let ragged = Gl::new(vec![vec![q(1, 1), q(0, 1)]]);
    assert_eq!(ragged, Err(GlError::NotSquare));
    // Over ℤ the determinant must be a unit: det 2 is refused, det 1 isn't.
    let z = |x: i64| Z::from(x);
    assert_eq!(
        Gl::new(vec![vec![z(2), z(0)], vec![z(0), z(1)]]),
        Err(GlError::NotUnit)
    );
    assert!(Gl::new(vec![vec![z(2), z(1)], vec![z(1), z(1)]]).is_ok());
}

// --- F5: a face turns the loop into curvature --------------------------------------------

#[test]
fn f5_a_face_carrying_cs_report_is_curvature_on_the_face_and_no_open_loop() {
    let t = triangle(eye(), eye(), rotation());
    let face = FaceDraft::new([t.a.oid().clone(), t.b.oid().clone(), t.c.oid().clone()])
        .seal(Oid::new("C"));
    assert_eq!(face.signer().by(), &Oid::new("C"));
    let c = composite(&t, std::slice::from_ref(&face));
    assert!(c.holonomy().loops().is_empty(), "no open loop");
    assert_eq!(c.curvature().len(), 1);
    let curl = &c.curvature()[0];
    assert_eq!(curl.face(), face.oid());
    assert!(!curl.holonomy().is_identity());
    assert_eq!(
        curl.holonomy().charpoly(2),
        rotation().charpoly(2),
        "the rotation's curvature, on the face"
    );
    assert!(c.provenance().contains(face.signer()));

    let Imperfect::Partial(h, loss) =
        hypothesize(&t.frames, &t.relations, std::slice::from_ref(&face))
    else {
        panic!("curvature is not flat");
    };
    assert!(h.alternatives().is_empty(), "no comma left to close");
    assert_eq!(
        loss,
        BTreeSet::from([Obstruction::Face(face.oid().clone())])
    );
}

// --- F6: the averaging control (Theorem C, P5a): documentation, not API ------------------

/// Average at A after carrying B's and C's readings to A along a tree.
fn average_at_a(ab: &Gl<Q>, bc: &Gl<Q>, ca: &Gl<Q>, tree: &str) -> Vec<Q> {
    let (xa, xb, xc) = (
        vec![q(1, 1), q(0, 1)],
        vec![q(0, 1), q(1, 1)],
        vec![q(1, 1), q(1, 1)],
    );
    let b = ab.inverse().apply(&xb);
    let c = match tree {
        "AB,AC" => ca.apply(&xc),
        _ => ab.inverse().apply(&bc.inverse().apply(&xc)),
    };
    (0..2)
        .map(|i| xa[i].add(&b[i]).add(&c[i]).mul(&q(1, 3)))
        .collect()
}

#[test]
fn f6_with_a_loop_two_honest_trees_give_two_averages_and_flat_they_agree() {
    let (i, r) = (eye(), rotation());
    assert_eq!(average_at_a(&i, &i, &r, "AB,AC"), vec![q(4, 15), q(4, 5)]);
    assert_eq!(average_at_a(&i, &i, &r, "AB,BC"), vec![q(2, 3), q(2, 3)]);
    assert_eq!(
        average_at_a(&i, &i, &i, "AB,AC"),
        average_at_a(&i, &i, &i, "AB,BC")
    );
}

// --- The unit, and refusals -------------------------------------------------------------

#[test]
fn empty_input_is_the_unit_not_a_failure() {
    let c = compose::<Q>(&[], &[], &[]).expect("the unit composes");
    assert!(c.agreement().is_empty());
    assert!(c.holonomy().loops().is_empty());
    assert!(c.curvature().is_empty());
    let Imperfect::Success(h) = hypothesize::<Q>(&[], &[], &[]) else {
        panic!("the empty composition is the unit, not a failure");
    };
    assert_eq!(h.ground(), c.oid());
    assert!(h.alternatives().is_empty());
}

#[test]
fn failure_is_only_a_type_error() {
    let t = triangle(eye(), eye(), rotation());
    let none = BTreeSet::new();

    // A relation to a frame that isn't given.
    let stray = frame("D");
    let mut relations = t.relations.clone();
    relations.push(relation(&t.a, &stray, eye(), "A"));
    assert_eq!(
        hypothesize(&t.frames, &relations, &[]),
        Imperfect::Failure(
            CompositionError::UnknownFrame(stray.oid().clone()),
            none.clone()
        )
    );

    // Frames of different dimension can't be related invertibly.
    let wide = Frame::new(Oid::new("W"), 3);
    let mut frames = t.frames.clone();
    frames.push(wide.clone());
    let r = RelationDraft::new(t.a.oid().clone(), wide.oid().clone(), eye()).seal(Oid::new("A"));
    assert_eq!(
        hypothesize(&frames, std::slice::from_ref(&r), &[]),
        Imperfect::Failure(
            CompositionError::DimensionMismatch {
                relation: r.oid().clone(),
                from: 2,
                to: 3
            },
            none.clone()
        )
    );

    // A 3 × 3 transport between frames on ℚ².
    let big = Gl::new(vec![
        vec![q(1, 1), q(0, 1), q(0, 1)],
        vec![q(0, 1), q(1, 1), q(0, 1)],
        vec![q(0, 1), q(0, 1), q(2, 1)],
    ])
    .expect("invertible");
    let r = relation(&t.a, &t.b, big, "A");
    assert_eq!(
        hypothesize(&t.frames, std::slice::from_ref(&r), &[]),
        Imperfect::Failure(
            CompositionError::TooLarge {
                relation: r.oid().clone(),
                size: 3,
                dim: 2
            },
            none.clone()
        )
    );

    // One reader named twice, at two dimensions.
    let twice = [frame("A"), Frame::new(Oid::new("A"), 3)];
    assert_eq!(
        hypothesize::<Q>(&twice, &[], &[]),
        Imperfect::Failure(
            CompositionError::DuplicateFrame(Oid::new("A")),
            none.clone()
        )
    );

    // A face on a side no relation spans.
    let lone = frame("L");
    let mut frames = t.frames.clone();
    frames.push(lone.clone());
    let face = FaceDraft::new([t.a.oid().clone(), t.b.oid().clone(), lone.oid().clone()])
        .seal(Oid::new("L"));
    assert!(matches!(
        hypothesize(&frames, &t.relations, &[face]),
        Imperfect::Failure(CompositionError::Face(_), _)
    ));
}

#[test]
fn one_relation_signed_twice_is_one_edge_with_two_signers() {
    let t = triangle(eye(), eye(), rotation());
    let again =
        RelationDraft::new(t.c.oid().clone(), t.a.oid().clone(), rotation()).seal(Oid::new("A"));
    assert_eq!(again.oid(), t.relations[2].oid());
    let mut relations = t.relations.clone();
    relations.push(again.clone());
    let c = compose(&t.frames, &relations, &[]).expect("well-typed");
    assert_eq!(c.edges().len(), 3, "one address, one edge");
    assert_eq!(c.holonomy().loops().len(), 1);
    assert!(c.provenance().contains(again.signer()));
    assert!(c.provenance().contains(t.relations[2].signer()));
    assert_eq!(c.oid(), composite(&t, &[]).oid(), "signers sit beside");
}

// --- Over ℤ -----------------------------------------------------------------------------

#[test]
fn over_z_a_quarter_turn_is_a_loop_and_flat_agrees_with_a_z_basis() {
    let z = |x: i64| Z::from(x);
    let i = Gl::new(vec![vec![z(1), z(0)], vec![z(0), z(1)]]).expect("unimodular");
    let quarter = Gl::new(vec![vec![z(0), z(-1)], vec![z(1), z(0)]]).expect("unimodular");
    let (a, b) = (frame("A"), frame("B"));
    let rel = |from: &Frame, to: &Frame, t: &Gl<Z>| {
        RelationDraft::new(from.oid().clone(), to.oid().clone(), t.clone()).seal(Oid::new("A"))
    };
    let frames = [a.clone(), b.clone()];

    // A bigon: two parallel relations A → B, I and a quarter-turn.
    let curved = [rel(&a, &b, &i), rel(&a, &b, &quarter)];
    let c = compose(&frames, &curved, &[]).expect("well-typed");
    assert_eq!(c.agreement().len(), 0);
    assert_eq!(c.holonomy().loops().len(), 1);
    assert_eq!(c.holonomy().loops()[0].frames().len(), 3, "a 2-loop");

    // Flat: a ℤ-basis of H⁰, the two unit sections.
    let flat = [rel(&a, &b, &i), rel(&b, &a, &i)];
    let c = compose(&frames, &flat, &[]).expect("well-typed");
    assert_eq!(c.agreement().len(), 2);
    let mut basis: Vec<Vec<Z>> = c
        .agreement()
        .iter()
        .map(|section| {
            assert_eq!(section[0].value(), section[1].value());
            section[0].value().to_vec()
        })
        .collect();
    basis.sort();
    let units = |v: &Vec<Z>| v.iter().filter(|x| **x != z(0)).count() == 1;
    assert!(basis.iter().all(units), "{basis:?}");
}

// --- Gl is a group, and exact ---------------------------------------------------------------

#[test]
fn gl_is_an_exact_group_with_one_identity_at_every_size() {
    let r = rotation();
    let s = shear();
    let g = gl([[(2, 1), (1, 1)], [(1, 1), (1, 1)]]);
    assert_eq!(eye(), Gl::identity(), "I₂ is the identity");
    assert!(Gl::<Q>::new(vec![vec![q(1, 1)]]).expect("I₁").is_identity());
    assert_eq!(r.compose(&r.inverse()), Gl::identity());
    assert_eq!(r.compose(&s).compose(&g), r.compose(&s.compose(&g)));
    assert_eq!(r.compose(&s).inverse(), s.inverse().compose(&r.inverse()));
    assert_ne!(r.compose(&s), s.compose(&r), "not commutative");
    // Path order: first r, then s, acts as s·r on a column.
    let v = vec![q(1, 1), q(2, 1)];
    assert_eq!(r.compose(&s).apply(&v), s.apply(&r.apply(&v)));
    // A smaller transport acts as the identity past its size.
    let one = Gl::new(vec![vec![q(3, 1)]]).expect("invertible");
    assert_eq!(one.apply(&v), vec![q(3, 1), q(2, 1)]);
    assert_eq!(one.size(), 1);
    assert_eq!(
        one.rows(2),
        vec![vec![q(3, 1), q(0, 1)], vec![q(0, 1), q(1, 1)]]
    );
    assert_eq!(Gl::<Q>::identity().size(), 0);
}
