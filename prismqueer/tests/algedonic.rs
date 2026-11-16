//! Mara's A1–A3 in prism (mirror `mara/roomba-monad` 2739e94c, §4.1, §5.1–5.3,
//! §9): the algedonic pillar split into `cross` (a certified crossing,
//! In | Out | Undecided) and `persist` (Wald's Accept | Reject | Continue over
//! distinct contexts), the signal kept a pair of enclosures, and the signed
//! ruler record with no default.

use num_bigint::BigInt;
use num_rational::BigRational;

use prismqueer::algedonic::{
    cross, decision_oid, persist, Calibration, Clock, Decision, Draft, Edge, Enclosure, Evidence,
    Membership, Persistence, Question, Recorded, Region, Signal, Wald,
};
use prismqueer::oid::Oid;
use prismqueer::poly::ZPoly;

fn q(n: i64, d: i64) -> BigRational {
    BigRational::new(BigInt::from(n), BigInt::from(d))
}

fn exact(n: i64, d: i64) -> Enclosure {
    Enclosure::exact(q(n, d))
}

fn enclose(lo: BigRational, hi: BigRational) -> Enclosure {
    Enclosure::new(lo, hi).expect("lo ≤ hi")
}

fn oid(s: &str) -> Oid {
    Oid::new(s)
}

/// A ruler signed by `by`: crossing above `eps`, under a Wald test with
/// α = β = 1/20. With [`knee`]'s calibration (p₀ = 1/6, p₁ = 1/2) one In
/// multiplies the likelihood ratio by 3, one Out by 3/5.
fn ruler_above(eps: BigRational, by: &str) -> prismqueer::algedonic::Ruler {
    Draft::new(
        Region::above(eps),
        Question::Max,
        Wald::new(q(1, 20), q(1, 20)).expect("valid rates"),
        Clock::agreed(10, 1, 100),
    )
    .seal(oid(by))
}

/// The knee instrument's calibration: p₀ = 1/6, p₁ = 1/2.
fn knee() -> Calibration {
    Calibration::new(oid("knee"), q(1, 6), q(1, 2), "600 labelled contexts")
        .expect("valid calibration")
}

fn decision(c: &Membership<Enclosure, Enclosure, Enclosure>) -> Decision {
    Decision::of(c)
}

// ─── cross: the three outcomes on exact enclosures ──────────────────────────

#[test]
fn cross_is_in_when_the_enclosure_lies_inside_the_region() {
    let r = Region::above(q(1, 1));
    assert!(matches!(cross(&exact(2, 1), &r), Membership::In(_)));
    assert!(matches!(
        cross(&enclose(q(3, 2), q(5, 1)), &r),
        Membership::In(_)
    ));
}

#[test]
fn cross_is_out_when_the_enclosure_and_region_are_disjoint() {
    let r = Region::above(q(1, 1));
    // At the threshold of an open region: certified not crossed.
    assert!(matches!(cross(&exact(1, 1), &r), Membership::Out(_)));
    assert!(matches!(
        cross(&enclose(q(-3, 1), q(1, 2)), &r),
        Membership::Out(_)
    ));
}

#[test]
fn cross_is_undecided_when_the_enclosure_straddles_the_edge() {
    let r = Region::above(q(1, 1));
    assert!(matches!(
        cross(&enclose(q(1, 2), q(3, 2)), &r),
        Membership::Undecided(_)
    ));
    // Touching the open edge from inside still straddles: 1 is not in (1, ∞).
    assert!(matches!(
        cross(&enclose(q(1, 1), q(2, 1)), &r),
        Membership::Undecided(_)
    ));
}

#[test]
/// Zero is a reading, not the absence of one (§6): an exact zero under
/// "above ε" is certified Out, and under a region holding zero it is In.
fn zero_is_a_reading() {
    assert!(matches!(
        cross(&exact(0, 1), &Region::above(q(1, 100))),
        Membership::Out(_)
    ));
    let silence = Region::new(Edge::Closed(q(0, 1)), Edge::Closed(q(0, 1))).expect("a point");
    assert!(matches!(cross(&exact(0, 1), &silence), Membership::In(_)));
}

#[test]
fn closed_and_unbounded_edges_decide_exactly() {
    let r = Region::new(Edge::Closed(q(1, 1)), Edge::Open(q(2, 1))).expect("nonempty");
    assert!(matches!(cross(&exact(1, 1), &r), Membership::In(_)));
    assert!(matches!(cross(&exact(2, 1), &r), Membership::Out(_)));
    assert!(matches!(
        cross(&enclose(q(3, 2), q(2, 1)), &r),
        Membership::Undecided(_)
    ));
    let below = Region::new(Edge::Unbounded, Edge::Closed(q(0, 1))).expect("nonempty");
    assert!(matches!(cross(&exact(-5, 1), &below), Membership::In(_)));
    assert!(matches!(cross(&exact(1, 3), &below), Membership::Out(_)));
}

#[test]
fn an_empty_region_and_an_inverted_enclosure_are_refused() {
    assert!(Region::new(Edge::Open(q(1, 1)), Edge::Open(q(1, 1))).is_err());
    assert!(Region::new(Edge::Closed(q(2, 1)), Edge::Closed(q(1, 1))).is_err());
    assert!(Enclosure::new(q(2, 1), q(1, 1)).is_err());
    assert!(Enclosure::of_floats(f64::NAN, 1.0).is_err());
    assert!(Enclosure::of_floats(0.0, f64::INFINITY).is_err());
    assert!(Enclosure::of_floats(1.0, 0.5).is_err());
}

#[test]
/// A certified enclosure from prism's exact roots: √2's isolating interval
/// decides "above 1" and straddles "above √2's neighbourhood" honestly.
fn an_isolating_interval_is_a_certified_enclosure() {
    // x² − 2: its positive root √2.
    let p = ZPoly::from_ascending([BigInt::from(-2), BigInt::from(0), BigInt::from(1)]);
    let roots = p.real_roots();
    let sqrt2 = Enclosure::of_isolating(&roots.intervals()[1]);
    assert!(sqrt2.lo() <= &q(1, 1));
    // Wherever its isolating interval lies, √2 itself is in it, and a
    // decided crossing agrees with √2 > 1.
    match cross(&sqrt2, &Region::above(q(1, 1))) {
        Membership::Out(_) => panic!("√2 is above 1; Out contradicts it"),
        Membership::In(_) | Membership::Undecided(_) => {}
    }
    assert!(matches!(
        cross(&sqrt2, &Region::above(q(-100, 1))),
        Membership::In(_)
    ));
}

// ─── Mara's theorem (§5.1): correct enclosures never contradict ─────────────

struct Mix(u64);

impl Mix {
    fn next(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9e37_79b9_7f4a_7c15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
        z ^ (z >> 31)
    }

    /// A small rational in [−4, 4], denominators up to 8.
    fn rational(&mut self) -> BigRational {
        let d = 1 + (self.next() % 8) as i64;
        let n = (self.next() % (8 * d as u64 + 1)) as i64 - 4 * d;
        q(n, d)
    }

    /// A non-negative width, zero one time in four.
    fn width(&mut self) -> BigRational {
        if self.next().is_multiple_of(4) {
            q(0, 1)
        } else {
            q((self.next() % 9) as i64, 1 + (self.next() % 8) as i64)
        }
    }

    fn edge(&mut self, at: BigRational) -> Edge {
        match self.next() % 3 {
            0 => Edge::Open(at),
            1 => Edge::Closed(at),
            _ => Edge::Unbounded,
        }
    }

    fn region(&mut self) -> Region {
        loop {
            let (a, b) = (self.rational(), self.rational());
            let (a, b) = if a <= b { (a, b) } else { (b, a) };
            let (lo, hi) = (self.edge(a), self.edge(b));
            if let Ok(r) = Region::new(lo, hi) {
                return r;
            }
        }
    }
}

#[test]
/// Two correct enclosures of the same v never give In and Out (PROVED, §5.1),
/// and each decided crossing agrees with v itself. Checked over many random
/// enclosure pairs containing v and random regions. Decided-vs-Undecided can
/// differ, and does.
fn correct_enclosures_of_one_value_never_contradict() {
    let mut mix = Mix(0x5eed_a1ed);
    let (mut seen_in, mut seen_out, mut seen_undecided, mut split) = (0, 0, 0, 0);
    for _ in 0..4000 {
        let v = mix.rational();
        let region = mix.region();
        let e1 = enclose(&v - mix.width(), &v + mix.width());
        let e2 = enclose(&v - mix.width(), &v + mix.width());
        let (c1, c2) = (cross(&e1, &region), cross(&e2, &region));
        let truth = region.contains(&v);
        for c in [&c1, &c2] {
            match c {
                Membership::In(_) => {
                    seen_in += 1;
                    assert!(truth, "In, but v = {v} is outside {region:?}");
                }
                Membership::Out(_) => {
                    seen_out += 1;
                    assert!(!truth, "Out, but v = {v} is inside {region:?}");
                }
                Membership::Undecided(_) => seen_undecided += 1,
            }
        }
        assert!(
            !matches!(
                (&c1, &c2),
                (Membership::In(_), Membership::Out(_)) | (Membership::Out(_), Membership::In(_))
            ),
            "contradiction at v = {v}: {e1:?} vs {e2:?} under {region:?}"
        );
        if c1.is_decided() != c2.is_decided() {
            split += 1;
        }
    }
    assert!(seen_in > 0 && seen_out > 0 && seen_undecided > 0);
    assert!(split > 0, "a tighter enclosure decides where a wider can't");
}

#[test]
fn a_tighter_enclosure_decides_where_a_wider_one_cannot() {
    let r = Region::above(q(1, 1));
    let tight = enclose(q(11, 10), q(12, 10));
    let wide = enclose(q(9, 10), q(12, 10));
    assert!(matches!(cross(&tight, &r), Membership::In(_)));
    assert!(matches!(cross(&wide, &r), Membership::Undecided(_)));
}

// ─── persist: Wald over distinct contexts ───────────────────────────────────

#[test]
/// α = β = 1/20 gives A = 19; each In multiplies the ratio by 3. Two
/// distinct contexts give 9 (Continue), three give 27 (Accept).
fn three_distinct_contexts_reach_accept() {
    let ruler = ruler_above(q(1, 1), "engineer");
    let mut ev = Evidence::at(oid("leg"));
    for (i, n) in ["song-a", "song-b", "song-c"].iter().enumerate() {
        assert_eq!(
            persist(&ev, &ruler, &knee()).persistence(),
            Persistence::Continue,
            "after {i}"
        );
        let r = ev.record(oid(n), oid("knee"), &ruler, Decision::In);
        assert_eq!(r, Ok(Recorded::New));
    }
    assert_eq!(
        persist(&ev, &ruler, &knee()).persistence(),
        Persistence::Accept
    );
}

#[test]
/// Each Out multiplies the ratio by 3/5; B = 1/19. (3/5)⁵ > 1/19 ≥ (3/5)⁶.
fn six_distinct_certified_outs_reach_reject() {
    let ruler = ruler_above(q(1, 1), "engineer");
    let mut ev = Evidence::at(oid("leg"));
    for i in 0..6 {
        assert_eq!(
            persist(&ev, &ruler, &knee()).persistence(),
            Persistence::Continue,
            "after {i}"
        );
        ev.record(oid(&format!("n{i}")), oid("knee"), &ruler, Decision::Out)
            .expect("new");
    }
    assert_eq!(
        persist(&ev, &ruler, &knee()).persistence(),
        Persistence::Reject
    );
}

#[test]
/// Undecided never counts as evidence (§5.1 d): float noise doesn't
/// accumulate into reality.
fn undecided_evidence_does_not_move_persist() {
    let ruler = ruler_above(q(1, 1), "engineer");
    let mut ev = Evidence::at(oid("leg"));
    for i in 0..50 {
        ev.record(
            oid(&format!("u{i}")),
            oid("knee"),
            &ruler,
            Decision::Undecided,
        )
        .expect("new");
    }
    assert_eq!(
        persist(&ev, &ruler, &knee()).persistence(),
        Persistence::Continue
    );
    ev.record(oid("a"), oid("knee"), &ruler, Decision::In)
        .expect("new");
    ev.record(oid("b"), oid("knee"), &ruler, Decision::In)
        .expect("new");
    assert_eq!(
        persist(&ev, &ruler, &knee()).persistence(),
        Persistence::Continue,
        "two Ins: 9 < 19"
    );
    ev.record(oid("c"), oid("knee"), &ruler, Decision::In)
        .expect("new");
    assert_eq!(
        persist(&ev, &ruler, &knee()).persistence(),
        Persistence::Accept
    );
}

#[test]
/// One visit counts once per (ruler, decision): no contraction (§5.2). The
/// same context walked three times is one context.
fn reusing_one_visit_does_not_count_twice() {
    let ruler = ruler_above(q(1, 1), "engineer");
    let mut ev = Evidence::at(oid("leg"));
    assert_eq!(
        ev.record(oid("song"), oid("knee"), &ruler, Decision::In),
        Ok(Recorded::New)
    );
    for _ in 0..5 {
        assert_eq!(
            ev.record(oid("song"), oid("knee"), &ruler, Decision::In),
            Ok(Recorded::Repeat)
        );
    }
    assert_eq!(ev.contexts(&ruler), 1);
    assert_eq!(
        persist(&ev, &ruler, &knee()).persistence(),
        Persistence::Continue
    );
    // A repeat that disagrees with the first reading is refused: a
    // deterministic instrument over the same bytes reads the same.
    assert!(ev
        .record(oid("song"), oid("knee"), &ruler, Decision::Out)
        .is_err());
}

#[test]
/// Evidence is relative to the instrument (§5.1 b): two instruments in one
/// neighbourhood are two contexts, kept apart, and never conflict.
fn evidence_from_two_instruments_stays_keyed_apart() {
    let ruler = ruler_above(q(1, 1), "engineer");
    let mut ev = Evidence::at(oid("leg"));
    ev.record(oid("song"), oid("knee"), &ruler, Decision::In)
        .expect("new");
    assert_eq!(
        ev.record(oid("song"), oid("ear"), &ruler, Decision::Undecided),
        Ok(Recorded::New)
    );
    assert_eq!(
        ev.record(oid("song"), oid("hand"), &ruler, Decision::In),
        Ok(Recorded::New)
    );
    assert_eq!(ev.contexts(&ruler), 3);
    assert_eq!(
        persist(&ev, &ruler, &knee()).persistence(),
        Persistence::Continue,
        "two Ins"
    );
    let ids: std::collections::BTreeSet<Oid> = ["knee", "ear", "hand"]
        .iter()
        .map(|i| {
            decision_oid(
                &oid("leg"),
                &oid("song"),
                &oid(i),
                &ruler.oid(),
                Decision::In,
            )
        })
        .collect();
    assert_eq!(ids.len(), 3, "the instrument is in the hash");
}

#[test]
/// Evidence under one ruler says nothing under another: two rulers at one
/// site judge separately (s9).
fn evidence_is_kept_per_ruler() {
    let stool = ruler_above(q(1, 1), "stool");
    let engineer = ruler_above(q(3, 1), "engineer");
    assert_ne!(stool.oid(), engineer.oid());
    let mut ev = Evidence::at(oid("leg"));
    for n in ["a", "b", "c"] {
        ev.record(oid(n), oid("knee"), &stool, Decision::In)
            .expect("new");
    }
    assert_eq!(
        persist(&ev, &stool, &knee()).persistence(),
        Persistence::Accept
    );
    assert_eq!(
        persist(&ev, &engineer, &knee()).persistence(),
        Persistence::Continue
    );
    assert_eq!(ev.contexts(&engineer), 0);
}

// ─── Hashing (§5.1 a): the decision, never the bits ─────────────────────────

#[test]
fn the_decision_hash_ignores_the_enclosure_bits() {
    let ruler = ruler_above(q(1, 1), "engineer");
    // Two machines' enclosures of one value, a last bit apart.
    let here = Enclosure::of_floats(1.5, 2.0).expect("finite");
    let there = Enclosure::of_floats(1.5f64.next_up(), 2.0).expect("finite");
    assert_ne!(here, there);
    let (a, b) = (
        decision(&cross(&here, ruler.region())),
        decision(&cross(&there, ruler.region())),
    );
    assert_eq!(a, Decision::In);
    assert_eq!(a, b);
    let at = |d| decision_oid(&oid("leg"), &oid("song"), &oid("knee"), &ruler.oid(), d);
    assert_eq!(at(a), at(b));
    // Every coordinate is in the hash.
    let base = at(Decision::In);
    for other in [
        decision_oid(
            &oid("lag"),
            &oid("song"),
            &oid("knee"),
            &ruler.oid(),
            Decision::In,
        ),
        decision_oid(
            &oid("leg"),
            &oid("sung"),
            &oid("knee"),
            &ruler.oid(),
            Decision::In,
        ),
        decision_oid(
            &oid("leg"),
            &oid("song"),
            &oid("knew"),
            &ruler.oid(),
            Decision::In,
        ),
        decision_oid(
            &oid("leg"),
            &oid("song"),
            &oid("knee"),
            &oid("other"),
            Decision::In,
        ),
        at(Decision::Out),
        at(Decision::Undecided),
    ] {
        assert_ne!(base, other);
    }
}

#[test]
/// Self-delimiting: moving a byte across a field boundary moves the hash.
fn the_decision_hash_is_self_delimiting() {
    let r = oid("r");
    assert_ne!(
        decision_oid(&oid("ab"), &oid("c"), &oid("d"), &r, Decision::In),
        decision_oid(&oid("a"), &oid("bc"), &oid("d"), &r, Decision::In),
    );
}

#[test]
/// The signer sits beside the ruler's address, never inside it: one ruler
/// signed by two people has one address and two signers.
fn the_signer_is_beside_the_ruler_address() {
    let a = ruler_above(q(1, 1), "engineer");
    let b = ruler_above(q(1, 1), "owner");
    assert_eq!(a.oid(), b.oid());
    assert_ne!(a.signer(), b.signer());
    assert_eq!(a.signer().by(), &oid("engineer"));
    assert_eq!(a.signer().over(), &a.oid());
    // Every part of the record moves the address.
    let base = a.oid();
    let wald = Wald::new(q(1, 20), q(1, 20)).expect("valid");
    let other = |region, question, wald, clock| Draft::new(region, question, wald, clock).oid();
    assert_ne!(
        base,
        other(
            Region::above(q(2, 1)),
            Question::Max,
            wald.clone(),
            Clock::agreed(10, 1, 100)
        )
    );
    assert_ne!(
        base,
        other(
            Region::above(q(1, 1)),
            Question::Sum,
            wald.clone(),
            Clock::agreed(10, 1, 100)
        )
    );
    assert_ne!(
        base,
        other(
            Region::above(q(1, 1)),
            Question::Max,
            Wald::new(q(1, 10), q(1, 20)).expect("valid"),
            Clock::agreed(10, 1, 100)
        )
    );
    assert_ne!(
        base,
        other(
            Region::above(q(1, 1)),
            Question::Max,
            Wald::new(q(1, 20), q(1, 10)).expect("valid"),
            Clock::agreed(10, 1, 100)
        )
    );
    assert_ne!(
        base,
        other(
            Region::above(q(1, 1)),
            Question::Max,
            wald,
            Clock::agreed(10, 1, 101)
        )
    );
}

// ─── The canonical address and the calibration (docs/spec/ruler.md) ─────────

#[test]
/// Recalibrating an instrument never moves a ruler: p₀/p₁ are a measurement
/// of the instrument, kept beside the ruler's address. Two calibrations get
/// two addresses of their own, and each verdict names the one it read.
fn recalibration_does_not_move_the_ruler() {
    let ruler = ruler_above(q(1, 1), "engineer");
    let before = knee();
    let after = Calibration::new(oid("knee"), q(1, 10), q(3, 5), "600 labelled contexts")
        .expect("valid calibration");
    assert_ne!(before.oid(), after.oid());
    assert_ne!(before.oid(), ruler.oid());

    let mut ev = Evidence::at(oid("leg"));
    for n in ["a", "b", "c"] {
        ev.record(oid(n), oid("knee"), &ruler, Decision::In)
            .expect("new");
    }
    let (x, y) = (persist(&ev, &ruler, &before), persist(&ev, &ruler, &after));
    // The ruler is the same ruler under both.
    assert_eq!(x.ruler(), &ruler.oid());
    assert_eq!(y.ruler(), &ruler.oid());
    // Each verdict names its calibration.
    assert_eq!(x.calibration(), &before.oid());
    assert_eq!(y.calibration(), &after.oid());
    // And the calibration is read: 3³ = 27 ≥ 19 accepts; 6³ = 216 too, but
    // two Ins split them (9 < 19 ≤ 36).
    assert_eq!(x.persistence(), Persistence::Accept);
    assert_eq!(y.persistence(), Persistence::Accept);
    let mut two = Evidence::at(oid("leg"));
    for n in ["a", "b"] {
        two.record(oid(n), oid("knee"), &ruler, Decision::In)
            .expect("new");
    }
    assert_eq!(
        persist(&two, &ruler, &before).persistence(),
        Persistence::Continue
    );
    assert_eq!(
        persist(&two, &ruler, &after).persistence(),
        Persistence::Accept
    );
}

#[test]
/// Every part of the calibration moves its address, and none of it moves
/// the ruler's.
fn every_part_of_the_calibration_is_in_its_address() {
    let base = knee().oid();
    let cal = |i: &str, p0, p1, m: &str| Calibration::new(oid(i), p0, p1, m).expect("valid").oid();
    let m = "600 labelled contexts";
    assert_eq!(base, cal("knee", q(1, 6), q(1, 2), m));
    for other in [
        cal("ear", q(1, 6), q(1, 2), m),
        cal("knee", q(1, 7), q(1, 2), m),
        cal("knee", q(1, 6), q(1, 3), m),
        cal("knee", q(1, 6), q(1, 2), "601 labelled contexts"),
    ] {
        assert_ne!(base, other);
    }
    // Self-delimiting: a byte moved from the instrument into the method.
    assert_ne!(
        cal("kneeX", q(1, 6), q(1, 2), "m"),
        cal("knee", q(1, 6), q(1, 2), "Xm")
    );
}

#[test]
/// A calibration speaks for its instrument only: p₀/p₁ measured on the knee
/// say nothing about what the ear certified. The ear's Ins don't enter the
/// knee's verdict.
fn a_calibration_reads_only_its_instruments_decisions() {
    let ruler = ruler_above(q(1, 1), "engineer");
    let mut ev = Evidence::at(oid("leg"));
    for n in ["a", "b", "c"] {
        ev.record(oid(n), oid("ear"), &ruler, Decision::In)
            .expect("new");
    }
    assert_eq!(
        persist(&ev, &ruler, &knee()).persistence(),
        Persistence::Continue
    );
    let ear =
        Calibration::new(oid("ear"), q(1, 6), q(1, 2), "600 labelled contexts").expect("valid");
    assert_eq!(
        persist(&ev, &ruler, &ear).persistence(),
        Persistence::Accept
    );
}

/// A decoder for the ruler preimage, written from `docs/spec/ruler.md`.
/// It returns the judgment and refuses trailing bytes, so a preimage that
/// decodes back to its judgment is self-delimiting.
fn decode(bytes: &[u8]) -> Option<Draft> {
    struct Cursor<'a>(&'a [u8]);
    impl Cursor<'_> {
        fn take(&mut self, n: usize) -> Option<&[u8]> {
            (self.0.len() >= n).then(|| {
                let (head, tail) = self.0.split_at(n);
                self.0 = tail;
                head
            })
        }
        fn byte(&mut self) -> Option<u8> {
            self.take(1).map(|b| b[0])
        }
        fn u64(&mut self) -> Option<u64> {
            Some(u64::from_le_bytes(self.take(8)?.try_into().ok()?))
        }
        fn field(&mut self) -> Option<&[u8]> {
            let n = usize::try_from(self.u64()?).ok()?;
            self.take(n)
        }
        fn rational(&mut self) -> Option<BigRational> {
            let sign = self.byte()?;
            let n = num_bigint::BigUint::from_bytes_le(self.field()?);
            let d = num_bigint::BigUint::from_bytes_le(self.field()?);
            let sign = match sign {
                0 => num_bigint::Sign::NoSign,
                1 => num_bigint::Sign::Plus,
                2 => num_bigint::Sign::Minus,
                _ => return None,
            };
            Some(BigRational::new(
                BigInt::from_biguint(sign, n),
                BigInt::from(d),
            ))
        }
        fn edge(&mut self) -> Option<Edge> {
            match self.byte()? {
                1 => Some(Edge::Open(self.rational()?)),
                2 => Some(Edge::Closed(self.rational()?)),
                3 => Some(Edge::Unbounded),
                _ => None,
            }
        }
    }
    let mut c = Cursor(bytes.strip_prefix(b"ruler\0")?);
    let region = Region::new(c.edge()?, c.edge()?).ok()?;
    let question = match c.byte()? {
        1 => Question::Any,
        2 => Question::Max,
        3 => Question::Product,
        4 => Question::Sum,
        _ => return None,
    };
    let wald = Wald::new(c.rational()?, c.rational()?).ok()?;
    let clock = Clock::agreed(c.u64()?, c.u64()?, c.u64()?);
    c.0.is_empty()
        .then(|| Draft::new(region, question, wald, clock))
}

#[test]
/// The ruler preimage decodes uniquely: every random judgment decodes back
/// to itself with no byte left over, and no proper prefix decodes. So two
/// judgments never share a preimage.
fn the_ruler_preimage_decodes_uniquely() {
    let mut mix = Mix(0x0b5e_55ed);
    let wide: BigInt = BigInt::from(1u8) << 70u32;
    let mut seen = std::collections::BTreeMap::new();
    for i in 0..400 {
        let mut region = mix.region();
        if i % 5 == 0 {
            // Edges wider than 64 bits.
            region = Region::new(
                Edge::Closed(mix.rational() * BigRational::from(wide.clone())),
                Edge::Unbounded,
            )
            .expect("half-line");
        }
        let question = [
            Question::Any,
            Question::Max,
            Question::Product,
            Question::Sum,
        ][(mix.next() % 4) as usize];
        let alpha = q(1 + (mix.next() % 9) as i64, 20);
        let beta = q(1 + (mix.next() % 9) as i64, 20);
        let wald = Wald::new(alpha, beta).expect("α + β < 1");
        let clock = Clock::agreed(mix.next(), mix.next() % 3, mix.next() >> (mix.next() % 64));
        let draft = Draft::new(region, question, wald, clock);
        let bytes = draft.preimage();
        assert_eq!(decode(&bytes).as_ref(), Some(&draft), "case {i}");
        for cut in 0..bytes.len() {
            assert!(decode(&bytes[..cut]).is_none(), "case {i}: prefix {cut}");
        }
        assert_eq!(
            draft.oid(),
            prismqueer::oid::Oid::hash(&prismqueer::oid::Scope::public(), &bytes)
        );
        if let Some(earlier) = seen.insert(bytes, draft.clone()) {
            assert_eq!(earlier, draft, "one preimage, two judgments");
        }
    }
}

// ─── The ruler record ───────────────────────────────────────────────────────

#[test]
fn the_wald_rates_are_validated() {
    assert!(Wald::new(q(0, 1), q(1, 20)).is_err(), "α = 0");
    assert!(Wald::new(q(1, 20), q(1, 1)).is_err(), "β = 1");
    assert!(Wald::new(q(1, 2), q(1, 2)).is_err(), "α + β = 1");
    let w = Wald::new(q(1, 20), q(1, 10)).expect("valid");
    assert_eq!((w.alpha(), w.beta()), (&q(1, 20), &q(1, 10)));
}

#[test]
/// p₀ and p₁ are validated where they live now, in the calibration:
/// `0 < p₀ < p₁ < 1`.
fn the_calibration_rates_are_validated() {
    let cal = |p0, p1| Calibration::new(oid("knee"), p0, p1, "m");
    assert!(cal(q(1, 2), q(1, 2)).is_err(), "p₀ = p₁");
    assert!(cal(q(1, 2), q(1, 6)).is_err(), "p₀ > p₁");
    assert!(cal(q(0, 1), q(1, 2)).is_err(), "p₀ = 0");
    assert!(cal(q(1, 6), q(1, 1)).is_err(), "p₁ = 1");
    let k = knee();
    assert_eq!((k.p0(), k.p1()), (&q(1, 6), &q(1, 2)));
    assert_eq!(k.instrument(), &oid("knee"));
    assert_eq!(k.method(), "600 labelled contexts");
}

#[test]
/// T3's clock is data: an agreed delay, read against ticks the caller
/// supplies. prism reads no wall clock.
fn the_clock_is_data() {
    let c = Clock::agreed(10, 3, 100);
    assert_eq!((c.reaction(), c.importance(), c.delay()), (10, 3, 100));
    assert!(!c.expired(5, 104));
    assert!(c.expired(5, 105));
    assert!(!c.expired(5, 4), "a tick before the start is not expiry");
    let ruler = ruler_above(q(1, 1), "engineer");
    assert_eq!(ruler.clock(), &Clock::agreed(10, 1, 100));
    assert_eq!(ruler.question(), Question::Max);
    assert_eq!(ruler.wald().alpha(), &q(1, 20));
}

#[test]
/// The question is part of the ruler (§4.2): one neighbourhood, four
/// questions, four readings.
fn the_question_picks_the_aggregate() {
    let nbhd = [exact(1, 2), exact(1, 2), exact(2, 1)];
    let read = |question, eps: BigRational| {
        Draft::new(
            Region::above(eps),
            question,
            Wald::new(q(1, 20), q(1, 20)).expect("valid"),
            Clock::agreed(1, 1, 1),
        )
        .seal(oid("engineer"))
        .read(&nbhd)
    };
    // Is any edge above 1? Yes, one.
    assert!(matches!(read(Question::Any, q(1, 1)), Membership::In(_)));
    // How bad is the worst edge? 2, never diluted.
    assert!(matches!(read(Question::Max, q(19, 10)), Membership::In(_)));
    // The product along the path: 1/2 · 1/2 · 2 = 1/2.
    assert!(matches!(
        read(Question::Product, q(1, 1)),
        Membership::Out(_)
    ));
    // The total: 3.
    assert!(matches!(read(Question::Sum, q(29, 10)), Membership::In(_)));
    assert!(matches!(read(Question::Sum, q(3, 1)), Membership::Out(_)));
}

#[test]
fn any_is_undecided_unless_some_crossing_is_in_or_all_are_out() {
    let r = |nbhd: &[Enclosure]| {
        Draft::new(
            Region::above(q(1, 1)),
            Question::Any,
            Wald::new(q(1, 20), q(1, 20)).expect("valid"),
            Clock::agreed(1, 1, 1),
        )
        .seal(oid("engineer"))
        .read(nbhd)
    };
    assert!(matches!(
        r(&[exact(0, 1), enclose(q(0, 1), q(2, 1))]),
        Membership::Undecided(_)
    ));
    assert!(matches!(r(&[exact(0, 1), exact(1, 2)]), Membership::Out(_)));
    assert!(matches!(r(&[]), Membership::Out(_)), "nothing is broken");
}

#[test]
/// Interval product with signs: [−1, 2] · [3, 4] = [−4, 8].
fn the_product_question_is_an_interval_product() {
    let read = |eps| {
        Draft::new(
            Region::new(Edge::Closed(eps), Edge::Unbounded).expect("nonempty"),
            Question::Product,
            Wald::new(q(1, 20), q(1, 20)).expect("valid"),
            Clock::agreed(1, 1, 1),
        )
        .seal(oid("engineer"))
        .read(&[enclose(q(-1, 1), q(2, 1)), enclose(q(3, 1), q(4, 1))])
    };
    assert!(matches!(read(q(-4, 1)), Membership::In(_)));
    assert!(matches!(read(q(-3, 1)), Membership::Undecided(_)));
    assert!(matches!(read(q(9, 1)), Membership::Out(_)));
}

// ─── A1: the signal is a pair of enclosures ─────────────────────────────────

#[test]
/// (0, 0), (0.4, 0.4) and (0.9, 0.9) all net to zero (§4.1, PROVED): netting
/// erases. As pairs they stay distinct, and ordered.
fn pairs_with_equal_net_value_stay_distinct() {
    let i = oid("stool-sensor");
    let calm = Signal::new(exact(0, 1), exact(0, 1), i.clone());
    let mixed = Signal::new(exact(2, 5), exact(2, 5), i.clone());
    let third_stool = Signal::new(exact(9, 10), exact(9, 10), i.clone());
    assert_ne!(calm, mixed);
    assert_ne!(mixed, third_stool);
    assert_eq!(calm.partial_cmp(&mixed), Some(std::cmp::Ordering::Less));
    assert_eq!(
        third_stool.partial_cmp(&calm),
        Some(std::cmp::Ordering::Greater)
    );
    assert_eq!(mixed.pain(), &exact(2, 5));
    assert_eq!(mixed.pleasure(), &exact(2, 5));
    assert_eq!(mixed.instrument(), &i);
}

#[test]
/// Componentwise, a product order: more pain and less pleasure is
/// incomparable to less pain and more pleasure, never one netted scalar.
fn the_pair_is_ordered_componentwise() {
    let i = oid("stool-sensor");
    let s = |p, j| Signal::new(exact(p, 10), exact(j, 10), i.clone());
    assert_eq!(
        s(1, 1).partial_cmp(&s(1, 1)),
        Some(std::cmp::Ordering::Equal)
    );
    assert_eq!(
        s(1, 1).partial_cmp(&s(2, 1)),
        Some(std::cmp::Ordering::Less)
    );
    assert_eq!(
        s(1, 1).partial_cmp(&s(1, 3)),
        Some(std::cmp::Ordering::Less)
    );
    assert_eq!(
        s(3, 1).partial_cmp(&s(1, 3)),
        None,
        "pain up, pleasure down"
    );
    assert_eq!(s(1, 3).partial_cmp(&s(3, 1)), None);
    // Equal net (0) and equal sum (4): still incomparable.
    assert_eq!(s(2, 2).partial_cmp(&s(3, 1)), None);
    // Enclosures order by both ends: [0, 3] and [1, 2] are incomparable.
    let wide = Signal::new(enclose(q(0, 1), q(3, 1)), exact(0, 1), i.clone());
    let narrow = Signal::new(enclose(q(1, 1), q(2, 1)), exact(0, 1), i.clone());
    assert_eq!(wide.partial_cmp(&narrow), None);
    // Two instruments' signals are not compared.
    let other = Signal::new(exact(1, 10), exact(1, 10), oid("ear"));
    assert_eq!(s(1, 1).partial_cmp(&other), None);
}
