//! kleinos as a pushout along spans (prism-021 D4; Mara 34f70d6d, probe
//! df9cccc3; laws L1–L6 from the frames note 54c88f72 §6.3, L7 from the
//! kleinos note §3).
//!
//! Two layers. The *event* layer: spans, keyed by the full commit pair
//! (emit ‖ receive), composing forward only. The *reading* layer: one
//! invertible transport per span, how the receiver's frame reads what was
//! sent; reading a span back is its inverse transport, never a new event.
//! The emergent third is the set of loops the spans add, each with its
//! holonomy. λ₂ is a reading beside the address: between two parties it is
//! 2k, a count of messages.
//!
//! Mara's first red test (34f70d6d §6 "For prism"): parties A and B, a span
//! A→B read as I and a span B→A read as `not` (ℤ/2, here its regular
//! representation, the 2×2 swap over ℤ). One bigon, one comma. Falsifiers:
//! B→A read as I gives no comma; dropping the span gives zero loops; two
//! spans whose 16-hex labels collide but whose full ids differ are two edges.

#![cfg(all(feature = "exact", feature = "bundle"))]

use std::collections::{BTreeMap, BTreeSet};

use prismqueer::field::Group;
use prismqueer::flux::pillar::{forall, Arbitrary, Sample};
use prismqueer::gl::{Gl, GlError};
use prismqueer::hypothesis::{
    compose, Composite, Face, FaceDraft, Frame, Obstruction, Relation, RelationDraft,
};
use prismqueer::kleinos::{kleinos, Commit, KleinosError, Party, Path, Pushout, Span, SpanId};
use prismqueer::oid::Oid;
use prismqueer::room::Z;
use terni::{Diagnostic, Imperfect, PropertyVerdict};

// --- Fixtures ----------------------------------------------------------------

fn gl(rows: [[i64; 2]; 2]) -> Gl<Z> {
    Gl::new(
        rows.iter()
            .map(|r| r.iter().map(|&x| Z::from(x)).collect())
            .collect(),
    )
    .expect("invertible over ℤ")
}

fn eye() -> Gl<Z> {
    Gl::identity()
}

/// `not` on one bit, as its regular representation: the swap of the two
/// basis states. det −1, a unit of ℤ.
fn not() -> Gl<Z> {
    gl([[0, 1], [1, 0]])
}

fn neg() -> Gl<Z> {
    gl([[-1, 0], [0, -1]])
}

fn shear() -> Gl<Z> {
    gl([[1, 1], [0, 1]])
}

fn frame(name: &str) -> Frame {
    Frame::new(Oid::new(name), 2)
}

fn commit(hex: &str) -> Commit {
    Commit::new(hex).expect("a full commit hash")
}

/// A full 40-hex commit: an 8-hex prefix, then 32 hex.
fn commit_of(prefix: &str, tail: char) -> Commit {
    commit(&format!("{prefix}{}", tail.to_string().repeat(32)))
}

fn span_id(emit: &Commit, receive: &Commit) -> SpanId {
    SpanId::new(emit.clone(), receive.clone()).expect("two commits")
}

fn span(id: &SpanId, from: &Frame, to: &Frame, reading: Gl<Z>) -> Span<Z> {
    Span::new(
        id.clone(),
        from.oid().clone(),
        to.oid().clone(),
        reading,
        Oid::new("Reed"),
    )
}

fn party(frames: &[&Frame]) -> Party<Z> {
    Party::new(
        frames.iter().map(|f| (*f).clone()).collect(),
        vec![],
        vec![],
    )
    .expect("a party with frames")
}

fn ok(p: Result<Pushout<Z>, KleinosError>) -> Pushout<Z> {
    p.unwrap_or_else(|e| panic!("kleinos refused: {e:?}"))
}

/// The reply chain c1 → c2 → c3: c2 receives A's message and emits B's
/// reply (bullet-up-011: "the commit that receives a message is the commit
/// that emits the next one").
struct Chain {
    a: Frame,
    b: Frame,
    s1: SpanId,
    s2: SpanId,
}

fn chain() -> Chain {
    let c1 = commit_of("552e3785", '1');
    let c2 = commit_of("0e97587c", '2');
    let c3 = commit_of("457086e9", '3');
    Chain {
        a: frame("A"),
        b: frame("B"),
        s1: span_id(&c1, &c2),
        s2: span_id(&c2, &c3),
    }
}

// --- Mara's test 2 -------------------------------------------------------------

/// A→B read as I, B→A read as `not`: one bigon, and its holonomy is `not`,
/// a comma. The edges are the two spans, by full id.
#[test]
fn a_message_and_a_reply_read_as_not_make_one_bigon_with_one_comma() {
    let Chain { a, b, s1, s2 } = chain();
    let p = ok(kleinos(
        &party(&[&a]),
        &party(&[&b]),
        &[span(&s1, &a, &b, eye()), span(&s2, &b, &a, not())],
    ));

    // The event layer: exactly the two spans, with their directions.
    let events: Vec<(&SpanId, &(Oid, Oid))> = p.events().iter().collect();
    assert_eq!(events.len(), 2);
    assert_eq!(p.events()[&s1], (a.oid().clone(), b.oid().clone()));
    assert_eq!(p.events()[&s2], (b.oid().clone(), a.oid().clone()));
    assert_eq!(s1.emit().hex().len(), 40, "full hashes, not labels");

    // The room: two frames, two parallel edges (a bigon), each a span.
    let graph = p.composite().graph();
    assert_eq!(graph.vertex_count(), 2);
    assert_eq!(graph.multiplicity((0, 1)), 2);
    let on_edges: BTreeSet<&Oid> = p.composite().edges().iter().collect();
    let witnessed: BTreeSet<&Oid> = p
        .party()
        .relations()
        .iter()
        .filter(|r| r.span().is_some())
        .map(Relation::oid)
        .collect();
    assert_eq!(on_edges, witnessed, "every edge is a span");
    let ids: BTreeSet<&SpanId> = p
        .party()
        .relations()
        .iter()
        .filter_map(Relation::span)
        .collect();
    assert_eq!(ids, BTreeSet::from([&s1, &s2]));

    // The emergent third: one loop, a bigon, holonomy `not`.
    assert_eq!(p.third().len(), 1);
    let bigon = &p.third()[0];
    assert_eq!(bigon.frames().len(), 3, "a closed walk of two steps");
    assert_eq!(bigon.frames()[0], bigon.frames()[2]);
    assert_eq!(bigon.relations().len(), 2);
    assert_eq!(bigon.holonomy(), &not());
    assert!(bigon.is_comma());
    assert_eq!(p.commas().len(), 1);

    // It feeds the hypothesis: one open loop, a menu for it.
    match p.hypothesize() {
        Imperfect::Partial(h, open) => {
            assert_eq!(open.len(), 1);
            assert!(matches!(open.iter().next(), Some(Obstruction::Loop(_))));
            assert_eq!(h.alternatives().len(), 1);
            assert_eq!(h.ground(), p.oid());
        }
        other => panic!("expected Partial with one comma, got {other:?}"),
    }
}

/// Falsifier 1: B→A read as I. Still one bigon, no comma.
#[test]
fn a_reply_read_as_sent_closes_the_bigon() {
    let Chain { a, b, s1, s2 } = chain();
    let p = ok(kleinos(
        &party(&[&a]),
        &party(&[&b]),
        &[span(&s1, &a, &b, eye()), span(&s2, &b, &a, eye())],
    ));
    assert_eq!(p.third().len(), 1, "the loop is there");
    assert!(p.third()[0].holonomy().is_identity());
    assert!(!p.third()[0].is_comma());
    assert!(p.commas().is_empty(), "no comma");
    assert!(matches!(p.hypothesize(), Imperfect::Success(_)));
}

/// Falsifier 2: drop the reply. One span, a tree: zero loops.
#[test]
fn one_span_adds_no_loop() {
    let Chain { a, b, s1, .. } = chain();
    let p = ok(kleinos(
        &party(&[&a]),
        &party(&[&b]),
        &[span(&s1, &a, &b, eye())],
    ));
    assert_eq!(p.events().len(), 1);
    assert!(p.third().is_empty());
    assert!(p.commas().is_empty());
}

/// Falsifier 3 (S3d): two spans whose 16-hex labels collide but whose full
/// ids differ are two edges, not one.
#[test]
fn spans_whose_labels_collide_are_two_edges() {
    let (a, b) = (frame("A"), frame("B"));
    let x = span_id(
        &commit(&format!("aaaa0000{}", "f".repeat(32))),
        &commit(&format!("bbbb1111{}", "0".repeat(32))),
    );
    let y = span_id(
        &commit(&format!("aaaa0000{}", "e".repeat(32))),
        &commit(&format!("bbbb1111{}", "1".repeat(32))),
    );
    assert_eq!(x.label().as_str(), "aaaa0000bbbb1111");
    assert_eq!(x.label(), y.label(), "one label");
    assert_ne!(x, y, "two events");

    let p = ok(kleinos(
        &party(&[&a]),
        &party(&[&b]),
        &[span(&x, &a, &b, eye()), span(&y, &a, &b, eye())],
    ));
    assert_eq!(p.events().len(), 2, "two events");
    assert_eq!(p.composite().graph().multiplicity((0, 1)), 2, "two edges");
    assert_eq!(p.composite().edges().len(), 2);
    assert_eq!(p.third().len(), 1, "one bigon between them");
    assert!(p.commas().is_empty());
}

// --- Refusals ------------------------------------------------------------------

/// A reading that doesn't invert is never a span's: `Gl` refuses it.
#[test]
fn a_reading_that_does_not_invert_is_refused() {
    let singular = Gl::new(vec![
        vec![Z::from(1), Z::from(1)],
        vec![Z::from(1), Z::from(1)],
    ]);
    assert_eq!(singular, Err(GlError::Singular));
    let doubling = Gl::new(vec![
        vec![Z::from(2), Z::from(0)],
        vec![Z::from(0), Z::from(1)],
    ]);
    assert_eq!(doubling, Err(GlError::NotUnit));
}

/// One id, two pairs of endpoints: a collision, not an edge.
#[test]
fn one_id_with_two_pairs_of_endpoints_is_a_collision() {
    let Chain { a, b, s1, .. } = chain();
    let c = frame("C");
    let got = kleinos(
        &party(&[&a]),
        &party(&[&b, &c]),
        &[span(&s1, &a, &b, eye()), span(&s1, &a, &c, eye())],
    );
    assert_eq!(got.err(), Some(KleinosError::Collision(s1.clone())));
    let reversed = kleinos(
        &party(&[&a]),
        &party(&[&b]),
        &[span(&s1, &a, &b, eye()), span(&s1, &b, &a, eye())],
    );
    assert_eq!(reversed.err(), Some(KleinosError::Collision(s1)));
}

/// One id, one pair of endpoints, two readings: refused too.
#[test]
fn one_id_read_two_ways_is_refused() {
    let Chain { a, b, s1, .. } = chain();
    let got = kleinos(
        &party(&[&a]),
        &party(&[&b]),
        &[span(&s1, &a, &b, eye()), span(&s1, &a, &b, not())],
    );
    assert_eq!(got.err(), Some(KleinosError::TwoReadings(s1)));
}

/// The same span given twice is one event and one edge (a keyed union).
#[test]
fn the_same_span_twice_is_one_edge() {
    let Chain { a, b, s1, .. } = chain();
    let p = ok(kleinos(
        &party(&[&a]),
        &party(&[&b]),
        &[span(&s1, &a, &b, eye()), span(&s1, &a, &b, eye())],
    ));
    assert_eq!(p.events().len(), 1);
    assert_eq!(p.composite().edges().len(), 1);
}

/// A collision is caught across the parties too: a span already inside a
/// glued party, given again between other frames.
#[test]
fn a_collision_with_a_span_already_glued_is_refused() {
    let Chain { a, b, s1, .. } = chain();
    let c = frame("C");
    let ab = ok(kleinos(
        &party(&[&a]),
        &party(&[&b]),
        &[span(&s1, &a, &b, eye())],
    ));
    let got = kleinos(ab.party(), &party(&[&c]), &[span(&s1, &b, &c, eye())]);
    assert_eq!(got.err(), Some(KleinosError::Collision(s1)));
}

/// The empty party holds no frame, so a span into it names a frame no
/// party holds: a span needs a frame on each side.
#[test]
fn a_span_into_the_empty_party_is_refused() {
    let Chain { a, b, s1, s2 } = chain();
    let empty = Party::<Z>::new(vec![], vec![], vec![]);
    let only_a = party(&[&a]);
    assert_eq!(
        empty
            .clone()
            .and_then(|e| kleinos(&e, &only_a, &[span(&s1, &a, &b, eye())]))
            .err(),
        Some(KleinosError::UnknownFrame(b.oid().clone()))
    );
    assert_eq!(
        empty
            .and_then(|e| kleinos(&only_a, &e, &[span(&s2, &b, &a, not())]))
            .err(),
        Some(KleinosError::UnknownFrame(b.oid().clone()))
    );
}

/// A party's relations and faces stay inside its frames.
#[test]
fn a_party_names_only_its_own_frames() {
    let (a, b) = (frame("A"), frame("B"));
    let r = RelationDraft::new(a.oid().clone(), b.oid().clone(), eye()).seal(Oid::new("A"));
    assert_eq!(
        Party::new(vec![a.clone()], vec![r], vec![]).err(),
        Some(KleinosError::UnknownFrame(b.oid().clone()))
    );
    let f = FaceDraft::new([a.oid().clone(), b.oid().clone(), frame("C").oid().clone()])
        .seal(Oid::new("A"));
    assert_eq!(
        Party::<Z>::new(vec![a], vec![], vec![f]).err(),
        Some(KleinosError::UnknownFrame(b.oid().clone()))
    );
}

/// A span between frames neither party holds is refused.
#[test]
fn a_span_to_nowhere_is_refused() {
    let Chain { a, b, s1, .. } = chain();
    let nowhere = frame("nowhere");
    let got = kleinos(
        &party(&[&a]),
        &party(&[&b]),
        &[span(&s1, &a, &nowhere, eye())],
    );
    assert_eq!(
        got.err(),
        Some(KleinosError::UnknownFrame(nowhere.oid().clone()))
    );
}

#[test]
fn a_commit_is_a_full_hash_and_a_span_is_two() {
    assert!(matches!(
        Commit::new("552e3785"),
        Err(KleinosError::NotACommit(_))
    ));
    assert!(matches!(
        Commit::new(&"G".repeat(40)),
        Err(KleinosError::NotACommit(_))
    ));
    assert!(Commit::new(&"a".repeat(64)).is_ok(), "SHA-256 object ids");
    let c = commit_of("552e3785", '1');
    assert_eq!(
        SpanId::new(c.clone(), c.clone()).err(),
        Some(KleinosError::SameCommit(c))
    );
}

// --- λ₂: a reading, not a law ----------------------------------------------------

/// Between two parties joined by k spans, L = [[k, −k], [−k, k]] and
/// λ₂ = 2k (Mara S7). It counts messages: what the readings say never moves
/// it, and it stays beside the address. k spans add k − 1 loops.
// λ₂ is measured by LAPACK: under `lapack`.
#[cfg(feature = "lapack")]
#[test]
fn lambda_2_between_two_parties_counts_messages() {
    use prismqueer::kleinos::Fiedler;
    let (a, b) = (frame("A"), frame("B"));
    let tails = ['1', '2', '3', '4', '5', '6'];
    for k in 1..=5usize {
        for reading in [eye(), not()] {
            let spans: Vec<Span<Z>> = (0..k)
                .map(|i| {
                    let id = span_id(
                        &commit_of("552e3785", tails[i]),
                        &commit_of("0e97587c", tails[i + 1]),
                    );
                    match i % 2 {
                        0 => span(&id, &a, &b, eye()),
                        _ => span(&id, &b, &a, reading.clone()),
                    }
                })
                .collect();
            let p = ok(kleinos(&party(&[&a]), &party(&[&b]), &spans));
            let lambda = p.fiedler().expect("measured").value();
            assert!(
                (lambda - 2.0 * k as f64).abs() < 1e-9,
                "k = {k}: λ₂ = {lambda}"
            );
            assert_eq!(p.third().len(), k - 1, "k − 1 loops");
        }
    }
}

// --- The event layer: a category, forward only (L7) --------------------------

#[test]
fn lineage_composes_forward_and_never_back() {
    let Chain { s1, s2, .. } = chain();
    let (p1, p2) = (Path::span(&s1), Path::span(&s2));
    let lineage = p1.then(&p2).expect("c1 → c2 → c3");
    assert_eq!(lineage.source(), s1.emit());
    assert_eq!(lineage.target(), s2.receive());
    assert_eq!(lineage.spans(), &[s1.clone(), s2.clone()]);
    assert!(p2.then(&p1).is_none(), "no arrow c3 → c1 follows");
    let at = Path::identity(s1.emit().clone());
    assert_eq!(at.then(&p1), Some(p1.clone()));
    assert_eq!(p1.then(&Path::identity(s1.receive().clone())), Some(p1));
}

/// Reading a span back is the inverse transport, on the same edge; no
/// event is made.
#[test]
fn reading_back_is_a_transport_not_an_event() {
    let Chain { a, b, s1, s2 } = chain();
    let p = ok(kleinos(
        &party(&[&a]),
        &party(&[&b]),
        &[span(&s1, &a, &b, shear()), span(&s2, &b, &a, not())],
    ));
    let before = p.events().clone();
    let back = p.read_back(&s1).expect("s1 is a span here");
    assert_eq!(back, shear().inverse());
    assert!(p.reading(&s1).expect("read").compose(&back).is_identity());
    assert_eq!(p.events(), &before, "no new event");
    assert_eq!(p.read_back(&span_id(s2.receive(), s1.emit())), None);
}

// --- Laws L1–L7 as properties --------------------------------------------------

/// A seeded world: three parties and spans between them, drawn from a small
/// pool of commits whose 8-hex prefixes collide (so 16-hex labels collide
/// while full ids differ, S3c/S3d), with spans that chain (colliding paths:
/// two routes between one pair of frames, Seam 6601cd62 §13).
#[derive(Clone, Debug)]
struct World {
    a: Party<Z>,
    b: Party<Z>,
    c: Party<Z>,
    ab: Vec<Span<Z>>,
    bc: Vec<Span<Z>>,
    ac: Vec<Span<Z>>,
    /// Some two distinct span ids drawn here share a label.
    labels_collide: bool,
}

/// splitmix64, from one drawn word.
struct Mix(u64);

impl Mix {
    fn next(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }
    fn below(&mut self, n: usize) -> usize {
        (self.next() % n as u64) as usize
    }
}

fn reading_of(k: usize) -> Gl<Z> {
    [eye(), not(), neg(), shear()][k % 4].clone()
}

/// A party named `name`: one to three frames (frames of `shared` may be
/// borrowed: overlapping parties glue), relations among them.
fn draw_party(mix: &mut Mix, name: &str, shared: &[Frame]) -> (Party<Z>, Vec<Frame>) {
    let mut frames: Vec<Frame> = (0..1 + mix.below(3))
        .map(|i| frame(&format!("{name}{i}")))
        .collect();
    if !shared.is_empty() && mix.below(4) == 0 {
        frames.push(shared[mix.below(shared.len())].clone());
    }
    let mut relations = Vec::new();
    for _ in 0..mix.below(4) {
        let (u, v) = (mix.below(frames.len()), mix.below(frames.len()));
        if u != v {
            relations.push(
                RelationDraft::new(
                    frames[u].oid().clone(),
                    frames[v].oid().clone(),
                    reading_of(mix.below(4)),
                )
                .seal(Oid::new(name)),
            );
        }
    }
    let party = Party::new(frames.clone(), relations, vec![]).expect("a party");
    (party, frames)
}

/// Commits from a pool of four prefixes; tails make them distinct.
fn draw_commit(mix: &mut Mix) -> Commit {
    let prefix = ["aaaa0000", "bbbb1111", "aaaa0000", "cccc2222"][mix.below(4)];
    let tail = "0123456789abcdef".as_bytes()[mix.below(6)] as char;
    commit_of(prefix, tail)
}

fn draw_spans(
    mix: &mut Mix,
    from: &[Frame],
    to: &[Frame],
    seen: &mut BTreeMap<SpanId, Span<Z>>,
) -> Vec<Span<Z>> {
    let mut spans = Vec::new();
    let mut last: Option<Commit> = None;
    for _ in 0..mix.below(5) {
        // Chain on from the last receive half of the time: paths that compose.
        let emit = match (&last, mix.below(2)) {
            (Some(c), 0) => c.clone(),
            _ => draw_commit(mix),
        };
        let receive = draw_commit(mix);
        let Ok(id) = SpanId::new(emit, receive.clone()) else {
            continue;
        };
        last = Some(receive);
        // An id drawn for another pair of parties would name frames these
        // two don't hold: skip it (one id stays one span everywhere).
        if seen.contains_key(&id) && !spans.iter().any(|s: &Span<Z>| s.id() == &id) {
            continue;
        }
        let s = seen.entry(id.clone()).or_insert_with(|| {
            let (u, v) = (&from[mix.below(from.len())], &to[mix.below(to.len())]);
            let forward = mix.below(2) == 0;
            let (u, v) = if forward { (u, v) } else { (v, u) };
            span(&id, u, v, reading_of(mix.below(4)))
        });
        spans.push(s.clone());
    }
    spans
}

impl Arbitrary for World {
    fn arbitrary(sample: &mut Sample) -> Self {
        let mut mix = Mix(u64::arbitrary(sample));
        let (a, fa) = draw_party(&mut mix, "a", &[]);
        let (b, fb) = draw_party(&mut mix, "b", &fa);
        let (c, fc) = draw_party(&mut mix, "c", &fb);
        let mut seen = BTreeMap::new();
        let ab = draw_spans(&mut mix, &fa, &fb, &mut seen);
        let bc = draw_spans(&mut mix, &fb, &fc, &mut seen);
        let ac = draw_spans(&mut mix, &fa, &fc, &mut seen);
        let labels: BTreeSet<String> = seen
            .keys()
            .map(|id| id.label().as_str().to_owned())
            .collect();
        World {
            a,
            b,
            c,
            ab,
            bc,
            ac,
            labels_collide: labels.len() < seen.len(),
        }
    }
}

const SEED_RUNS: usize = 96;

fn check(holds: bool, what: &str) -> PropertyVerdict {
    if holds {
        PropertyVerdict::Pass
    } else {
        PropertyVerdict::Fail(Diagnostic::new(what.to_owned()))
    }
}

fn passes(v: PropertyVerdict) {
    assert!(matches!(v, PropertyVerdict::Pass), "{v:?}");
}

/// The composite of a plain union: what a pushout must equal.
fn union(parties: &[&Party<Z>], spans: &[&[Span<Z>]]) -> Composite<Z> {
    let frames: Vec<Frame> = parties.iter().flat_map(|p| p.frames().to_vec()).collect();
    let mut relations: Vec<Relation<Z>> = parties
        .iter()
        .flat_map(|p| p.relations().to_vec())
        .collect();
    relations.extend(
        spans
            .iter()
            .flat_map(|s| s.iter().map(|s| s.relation().clone())),
    );
    let faces: Vec<Face> = parties.iter().flat_map(|p| p.faces().to_vec()).collect();
    compose(&frames, &relations, &faces).expect("the union composes")
}

/// L1, associativity (54c88f72 §6.3; 34f70d6d §3: on the nose, with full
/// ids): (a ⊔ b) ⊔ c = a ⊔ (b ⊔ c), one address, one event set.
#[test]
fn l1_kleinos_is_associative() {
    let mut collided = 0;
    passes(forall::<World, _>(b"kleinos L1", SEED_RUNS, |w| {
        collided += usize::from(w.labels_collide);
        let left = ok(kleinos(
            &ok(kleinos(&w.a, &w.b, &w.ab)).party().clone(),
            &w.c,
            &w.bc,
        ));
        let right = ok(kleinos(&w.a, ok(kleinos(&w.b, &w.c, &w.bc)).party(), &w.ab));
        let whole = union(&[&w.a, &w.b, &w.c], &[&w.ab, &w.bc]);
        check(
            left.oid() == right.oid()
                && left.events() == right.events()
                && left.oid() == whole.oid(),
            "L1: two bracketings, two pushouts",
        )
    }));
    assert!(collided > 0, "some worlds had colliding labels");
}

/// L2, commutativity: a ⊔ b = b ⊔ a, the third included.
#[test]
fn l2_kleinos_is_commutative() {
    passes(forall::<World, _>(b"kleinos L2", SEED_RUNS, |w| {
        let (ab, ba) = (
            ok(kleinos(&w.a, &w.b, &w.ab)),
            ok(kleinos(&w.b, &w.a, &w.ab)),
        );
        check(
            ab.oid() == ba.oid() && ab.events() == ba.events() && ab.third() == ba.third(),
            "L2: a ⊔ b ≠ b ⊔ a",
        )
    }));
}

/// L3, identity: no spans glue nothing; the pushout is the disjoint (or
/// overlapping) union, with no third. In the event layer, identities at a
/// commit are units for path composition.
#[test]
fn l3_no_spans_is_the_union() {
    passes(forall::<World, _>(b"kleinos L3", SEED_RUNS, |w| {
        let p = ok(kleinos(&w.a, &w.b, &[]));
        let units = w.ab.iter().all(|s| {
            let path = Path::span(s.id());
            Path::identity(s.id().emit().clone()).then(&path) == Some(path.clone())
                && path.then(&Path::identity(s.id().receive().clone())) == Some(path.clone())
        });
        check(
            p.oid() == union(&[&w.a, &w.b], &[]).oid()
                && p.third().is_empty()
                && p.events().is_empty()
                && units,
            "L3: kleinos(a, b, ∅) ≠ a ⊔ b",
        )
    }));
}

/// L3, the unit (54c88f72 §6.3; `hypothesize`: empty input = unit): the
/// empty frame-graph is a party, and ∅ ⊔ a = a = a ⊔ ∅. One address (the
/// party alone), its events, and no third. Checked on a drawn party and on
/// a glued one, which carries spans.
#[test]
fn l3_the_empty_party_is_the_unit() {
    passes(forall::<World, _>(b"kleinos L3 unit", SEED_RUNS, |w| {
        let Ok(empty) = Party::<Z>::new(vec![], vec![], vec![]) else {
            return check(false, "L3: the empty party is refused");
        };
        let glued = ok(kleinos(&w.a, &w.b, &w.ab)).party().clone();
        let holds = [&w.a, &glued].into_iter().all(|a| {
            let alone = union(&[a], &[]);
            let events: BTreeMap<SpanId, (Oid, Oid)> = a
                .relations()
                .iter()
                .filter_map(|r| {
                    r.span()
                        .map(|id| (id.clone(), (r.from().clone(), r.to().clone())))
                })
                .collect();
            [kleinos(&empty, a, &[]), kleinos(a, &empty, &[])]
                .into_iter()
                .all(|p| {
                    p.is_ok_and(|p| {
                        p.oid() == alone.oid()
                            && p.party() == a
                            && *p.events() == events
                            && p.third().is_empty()
                    })
                })
        });
        check(holds, "L3: ∅ ⊔ a ≠ a or a ⊔ ∅ ≠ a")
    }));
}

/// L4, sovereignty: every relation of the pushout is a's, b's or a span's,
/// and every edge between a frame only a holds and a frame only b holds is a
/// span. Nothing fabricated: no cone, no join edge.
#[test]
fn l4_every_edge_is_declared() {
    passes(forall::<World, _>(b"kleinos L4", SEED_RUNS, |w| {
        let p = ok(kleinos(&w.a, &w.b, &w.ab));
        let declared: BTreeSet<Oid> =
            w.a.relations()
                .iter()
                .chain(w.b.relations())
                .map(|r| r.oid().clone())
                .chain(w.ab.iter().map(|s| s.relation().oid().clone()))
                .collect();
        let held: BTreeSet<Oid> = p
            .party()
            .relations()
            .iter()
            .map(|r| r.oid().clone())
            .collect();
        let on_edges: BTreeSet<Oid> = p.composite().edges().iter().cloned().collect();
        let only = |x: &Party<Z>, y: &Party<Z>| -> BTreeSet<Oid> {
            let theirs: BTreeSet<&Oid> = y.frames().iter().map(Frame::oid).collect();
            x.frames()
                .iter()
                .map(Frame::oid)
                .filter(|f| !theirs.contains(f))
                .cloned()
                .collect()
        };
        let (only_a, only_b) = (only(&w.a, &w.b), only(&w.b, &w.a));
        let crossing_are_spans = p.party().relations().iter().all(|r| {
            let crosses = (only_a.contains(r.from()) && only_b.contains(r.to()))
                || (only_b.contains(r.from()) && only_a.contains(r.to()));
            !crosses || r.span().is_some()
        });
        check(
            held == declared
                && on_edges == declared
                && p.composite().graph().edges().count() == declared.len()
                && crossing_are_spans,
            "L4: an edge nobody declared",
        )
    }));
}

/// L5, the emergent third, counted: the spans add exactly b₁(pushout) −
/// b₁(a ∪ b) loops (Mayer–Vietoris), each with its holonomy; a comma is
/// one whose holonomy isn't the identity, compared exactly.
#[test]
fn l5_the_third_is_the_loops_the_spans_add() {
    passes(forall::<World, _>(b"kleinos L5", SEED_RUNS, |w| {
        let p = ok(kleinos(&w.a, &w.b, &w.ab));
        let before = union(&[&w.a, &w.b], &[]).graph().cycle_rank();
        let after = p.composite().graph().cycle_rank();
        let new: BTreeSet<&SpanId> =
            w.ab.iter()
                .map(Span::id)
                .filter(|id| {
                    !w.a.relations()
                        .iter()
                        .chain(w.b.relations())
                        .any(|r| r.span() == Some(*id))
                })
                .collect();
        let commas = p.commas().len()
            == p.third()
                .iter()
                .filter(|l| !l.holonomy().is_identity())
                .count();
        let closed = p.third().iter().all(|l| {
            l.frames().first() == l.frames().last()
                && l.relations().len() + 1 == l.frames().len()
                && new.contains(l.span())
        });
        check(
            p.third().len() == after - before && commas && closed,
            "L5: the third isn't the added loops",
        )
    }));
}

/// L6, idempotence: a ⊔ a = a (fusion is refused as a policy, never by
/// the law); and spans whose labels collide are never fused.
#[test]
fn l6_a_party_glued_to_itself_is_itself() {
    passes(forall::<World, _>(b"kleinos L6", SEED_RUNS, |w| {
        let p = ok(kleinos(&w.a, &w.a, &[]));
        let ab = ok(kleinos(&w.a, &w.b, &w.ab));
        let ids: BTreeSet<&SpanId> = w.ab.iter().map(Span::id).collect();
        let witnessed = ab
            .party()
            .relations()
            .iter()
            .filter(|r| r.span().is_some())
            .count();
        check(
            p.oid() == union(&[&w.a], &[]).oid()
                && p.third().is_empty()
                && ab.events().len() == ids.len()
                && witnessed == ids.len(),
            "L6: fused",
        )
    }));
}

/// L7, acyclic provenance: span arrows compose only forward (r₁ = e₂),
/// associatively; reading back is a transport and makes no event.
#[test]
fn l7_spans_compose_forward_only() {
    passes(forall::<World, _>(b"kleinos L7", SEED_RUNS, |w| {
        let spans: Vec<&Span<Z>> = w.ab.iter().chain(&w.bc).chain(&w.ac).collect();
        let mut holds = true;
        for x in &spans {
            for y in &spans {
                let (px, py) = (Path::span(x.id()), Path::span(y.id()));
                holds &= px.then(&py).is_some() == (x.id().receive() == y.id().emit());
                for z in &spans {
                    let pz = Path::span(z.id());
                    let left = px.then(&py).and_then(|xy| xy.then(&pz));
                    let right = py.then(&pz).and_then(|yz| px.then(&yz));
                    holds &= left == right;
                }
            }
        }
        let p = ok(kleinos(&w.a, &w.b, &w.ab));
        let before = p.events().clone();
        holds &= w.ab.iter().all(|s| {
            let back = p.read_back(s.id()).expect("a span of the pushout");
            s.reading().compose(&back).is_identity()
        });
        holds &= p.events() == &before;
        check(holds, "L7: an arrow went back")
    }));
}
