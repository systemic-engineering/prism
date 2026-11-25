//! λ₂ is a reading, not a gauge of choices (Mara, mirror
//! `mara/foerster-gauge-literal` e694dad1, `docs/math/2026-11-25-mara-the-
//! foerster-gauge-literal.md` §3, probe rows K1, K2 and G1).
//!
//! - **K1.** The lawful pushout of K₃ and K₃ over one span succeeds, and
//!   every held gauge rises: frames 3, 3 → 6, relations 3, 3 → 7, b₁ 1, 1 →
//!   2, with no emergent third (the span is a bridge). The quietest
//!   Laplacian mode falls from exactly 3 to the least root of x² − 5x + 2,
//!   (5 − √17)/2, below 1. L(P)'s charpoly is Mara's
//!   x⁶ − 14x⁵ + 74x⁴ − 180x³ + 189x² − 54x = x(x − 3)³(x² − 5x + 2).
//!   Nothing narrowed, so a gate on "λ₂ rose" (the lab's
//!   `Property::FiedlerRise`) would refuse a glue that kept everything.
//! - **K2.** Two one-frame parties joined by k identical spans: λ₂ = 2k
//!   exactly, with k − 1 loops, no comma and one reading. It counts
//!   messages. The lab's float twin is
//!   `prismqueer/tests/kleinos.rs::lambda_2_between_two_parties_counts_messages`.
//! - **G1, one case.** Glue a one-frame party to K₃ by one span (a pendant):
//!   λ₂ goes 3 → 1 while every held gauge rises.
//! - **No gate reads it.** Each glue above is `Ok` and its hypothesis a
//!   `Success`; and nowhere in the floor's code outside `room` is λ₂
//!   computed or read (`quietest`, `laplacian`, `fiedler`).
//!
//! Everything here is exact: L₀ over ℤ, its minimal polynomial, factors
//! over ℚ, roots compared by `Algebraic::compare`. No float is read.

use std::cmp::Ordering;
use std::fs;
use std::path::{Path, PathBuf};

use num_bigint::BigInt;
use num_rational::BigRational;
use prism_laws::field::Group;
use prism_laws::gl::Gl;
use prism_laws::hypothesis::{compose, Composite, Frame, RelationDraft};
use prism_laws::kleinos::{kleinos, Commit, Party, Pushout, Span, SpanId};
use prism_laws::oid::Oid;
use prism_laws::poly::{Algebraic, Budget, ZPoly};
use prism_laws::room::{holes, laplacian, quietest, Quietest, Z};
use terni::Imperfect;

// --- Fixtures ------------------------------------------------------------------

const BUDGET: Budget = Budget(64);

fn frame(name: &str) -> Frame {
    Frame::new(Oid::new(name), 1)
}

/// The identity transport: every reading agrees, so nothing is a comma.
fn eye() -> Gl<Z> {
    Gl::identity()
}

/// K_n as a party: n frames named `<name>0`, `<name>1`, …, every pair
/// related by the identity, signed by `name`.
fn complete(name: &str, n: usize) -> (Party<Z>, Vec<Frame>) {
    let frames: Vec<Frame> = (0..n).map(|i| frame(&format!("{name}{i}"))).collect();
    let relations = (0..n)
        .flat_map(|i| (i + 1..n).map(move |j| (i, j)))
        .map(|(i, j)| {
            RelationDraft::new(frames[i].oid().clone(), frames[j].oid().clone(), eye())
                .seal(Oid::new(name))
        })
        .collect();
    let party = Party::new(frames.clone(), relations, vec![]).expect("relations among its frames");
    (party, frames)
}

/// The k-th span, emitted at `from` and received at `to`, read as the
/// identity: distinct full ids, one reading.
fn span(k: usize, from: &Frame, to: &Frame) -> Span<Z> {
    let commit = |c: usize| Commit::new(&format!("{c:040x}")).expect("a full commit hash");
    let id = SpanId::new(commit(2 * k + 1), commit(2 * k + 2)).expect("two commits");
    Span::new(
        id,
        from.oid().clone(),
        to.oid().clone(),
        eye(),
        Oid::new("Reed"),
    )
}

fn alone(party: &Party<Z>) -> Composite<Z> {
    compose(party.frames(), party.relations(), party.faces()).expect("a party composes")
}

/// The held gauges Mara reads (§2.2): frames, relations, b₁. b₁ is the
/// graph's: the free generators of H₁ with no faces, flat loops included
/// (the composite's located loops are only the ones that don't close).
fn held(c: &Composite<Z>) -> (usize, usize, usize) {
    let b1 = holes::<Z>(&(), c.graph(), &[]).expect("no faces").len();
    (c.graph().vertices().count(), c.edges().len(), b1)
}

fn poly(descending: &[i64]) -> ZPoly {
    ZPoly::from_descending(descending.iter().map(|&c| BigInt::from(c)))
}

/// The `index`-th real root of `descending`, as a tag.
fn root(descending: &[i64], index: usize) -> Algebraic {
    Algebraic::new(poly(descending), index)
}

fn mode(q: Quietest) -> Algebraic {
    match q {
        Quietest::Mode(a) => a,
        other => panic!("expected a quietest mode, got {other:?}"),
    }
}

fn green(p: &Pushout<Z>) {
    assert!(
        matches!(p.hypothesize(), Imperfect::Success(_)),
        "the glue's hypothesis is a Success: nothing refused it"
    );
}

fn rational(n: i64, d: i64) -> BigRational {
    BigRational::new(BigInt::from(n), BigInt::from(d))
}

// --- K1 ------------------------------------------------------------------------

/// Mara's probe K1 (§3.1): kleinos(K₃, K₃, one span). Every held gauge
/// rises, and λ₂ falls from exactly 3 to (5 − √17)/2 < 1.
#[test]
fn k1_gluing_two_triangles_by_one_span_keeps_everything_and_lowers_lambda_2() {
    let (a, fa) = complete("A", 3);
    let (b, fb) = complete("B", 3);
    let p = kleinos(&a, &b, &[span(0, &fa[0], &fb[0])]).expect("the lawful glue succeeds");
    green(&p);

    // The held gauges rise: frames 3, 3 → 6; relations 3, 3 → 7; b₁ 1, 1 → 2.
    let (ca, cb) = (alone(&a), alone(&b));
    assert_eq!(held(&ca), (3, 3, 1));
    assert_eq!(held(&cb), (3, 3, 1));
    assert_eq!(held(p.composite()), (6, 7, 2));
    // The span is a bridge: the third is b₁(P) − b₁(a ∪ b) = 0, and no comma.
    assert!(p.third().is_empty(), "no emergent third");
    assert!(p.commas().is_empty());

    // Each party's quietest mode is exactly 3: L(K₃) has minimal x(x − 3).
    let three = root(&[1, -3], 0);
    assert_eq!(mode(quietest(ca.graph(), BUDGET)), three);
    assert_eq!(mode(quietest(cb.graph(), BUDGET)), three);

    // L(P)'s charpoly is Mara's, and with x divided out it is
    // (x − 3)³(x² − 5x + 2).
    let charpoly = laplacian(p.composite().graph()).charpoly().poly();
    assert_eq!(charpoly, poly(&[1, -14, 74, -180, 189, -54, 0]));
    let quintic = poly(&[1, -14, 74, -180, 189, -54]);
    assert_eq!(&charpoly, &(&quintic * &poly(&[1, 0])));
    let factor = poly(&[1, -5, 2]);
    let cube = &(&poly(&[1, -3]) * &poly(&[1, -3])) * &poly(&[1, -3]);
    assert_eq!(quintic, &cube * &factor);

    // The glued quietest mode is the least root of that factor, exactly.
    let lambda = mode(quietest(p.composite().graph(), BUDGET));
    assert_eq!(lambda, Algebraic::new(factor.clone(), 0));
    assert_eq!(
        lambda.isolating().map(|i| (i.lo().clone(), i.hi().clone())),
        Some((rational(0, 1), rational(3, 1))),
        "(5 − √17)/2 ≈ 0.438 is isolated in (0, 3]: x² − 5x + 2's other root, ≈ 4.56, is past 3"
    );

    // Below 1, above 0, and so below each party's 3: λ₂ fell by more than 2.
    assert_eq!(lambda.compare(&root(&[1, -1], 0)), Some(Ordering::Less));
    assert_eq!(lambda.compare(&root(&[1, 0], 0)), Some(Ordering::Greater));
    assert_eq!(lambda.compare(&three), Some(Ordering::Less));
    assert_eq!(lambda.compare(&root(&[1, -2], 0)), Some(Ordering::Less));
}

// --- G1, one case ----------------------------------------------------------------

/// A pendant glued on (probe G1's family, by the lawful kleinos): K₃ and
/// one frame by one span. Everything held rises; λ₂ goes 3 → 1 exactly
/// (L has eigenvalues 0, 1, 3, 4).
#[test]
fn a_pendant_glued_onto_a_triangle_lowers_lambda_2_while_nothing_narrows() {
    let (a, fa) = complete("A", 3);
    let (b, fb) = complete("P", 1);
    let p = kleinos(&a, &b, &[span(0, &fa[0], &fb[0])]).expect("the lawful glue succeeds");
    green(&p);
    assert_eq!(held(&alone(&a)), (3, 3, 1));
    assert_eq!(held(&alone(&b)), (1, 0, 0));
    assert_eq!(held(p.composite()), (4, 4, 1));
    assert_eq!(
        laplacian(p.composite().graph()).charpoly().poly(),
        &(&(&poly(&[1, 0]) * &poly(&[1, -1])) * &poly(&[1, -3])) * &poly(&[1, -4])
    );
    assert_eq!(mode(quietest(alone(&a).graph(), BUDGET)), root(&[1, -3], 0));
    assert_eq!(
        mode(quietest(p.composite().graph(), BUDGET)),
        root(&[1, -1], 0)
    );
}

// --- K2 ----------------------------------------------------------------------

/// Mara's probe K2 (§3.2): two one-frame parties and k identical spans.
/// L = [[k, −k], [−k, k]], so λ₂ = 2k exactly: it counts messages. One
/// reading, k − 1 loops, no comma.
#[test]
fn k2_lambda_2_between_two_one_frame_parties_is_2k() {
    let (a, fa) = complete("A", 1);
    let (b, fb) = complete("B", 1);
    for k in 1..=5usize {
        let spans: Vec<Span<Z>> = (0..k).map(|i| span(i, &fa[0], &fb[0])).collect();
        let p = kleinos(&a, &b, &spans).expect("the lawful glue succeeds");
        green(&p);
        assert_eq!(held(p.composite()), (2, k, k - 1), "k = {k}");
        assert_eq!(p.third().len(), k - 1);
        assert!(p.commas().is_empty());
        assert_eq!(p.composite().agreement().len(), 1, "one reading");
        assert_eq!(
            mode(quietest(p.composite().graph(), BUDGET)),
            root(&[1, -2 * k as i64], 0),
            "k = {k}"
        );
    }
}

// --- No gate reads λ₂ --------------------------------------------------------------

/// The code of a source, comments dropped (floor.rs's rule).
fn code(source: &str) -> String {
    source
        .lines()
        .map(|line| match line.find("//") {
            Some(i) => &line[..i],
            None => line,
        })
        .collect::<Vec<_>>()
        .join("\n")
}

fn sources(dir: &Path, out: &mut Vec<PathBuf>) {
    for entry in fs::read_dir(dir).expect("src/ is readable") {
        let path = entry.expect("an entry").path();
        match path.is_dir() {
            true => sources(&path, out),
            false if path.extension().is_some_and(|e| e == "rs") => out.push(path),
            false => {}
        }
    }
}

/// λ₂ is computed in one place in the floor, `room::quietest` (through
/// `room::laplacian`), and nothing in the floor calls either or names a
/// Fiedler value: so no gate or refusal of `kleinos`, `compose` or
/// `hypothesize` can consult it. The lab's `Pushout::fiedler` is a sealed
/// reading beside the address.
#[test]
fn no_gate_or_refusal_in_the_floor_consults_lambda_2() {
    let src = Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
    let mut files = Vec::new();
    sources(&src, &mut files);
    assert!(files.iter().any(|f| f.ends_with("kleinos.rs")));
    for file in files {
        let body = code(&fs::read_to_string(&file).expect("a source"));
        let name = file.strip_prefix(&src).expect("under src/").display();
        assert!(
            !body.to_lowercase().contains("fiedler"),
            "{name} names a Fiedler value"
        );
        let (q, l) = (
            body.matches("quietest(").count(),
            body.matches("laplacian(").count(),
        );
        match file.ends_with("room.rs") {
            // The definitions, and quietest's own read of L₀.
            true => assert_eq!((q, l), (1, 2), "{name}: quietest and laplacian"),
            false => assert_eq!((q, l), (0, 0), "{name} reads λ₂"),
        }
    }
}
