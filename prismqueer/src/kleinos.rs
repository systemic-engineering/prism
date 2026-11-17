//! `prismqueer::kleinos` — two parties' frames glued along declared spans: a
//! pushout (prism-021 D4; Mara 34f70d6d §3, probe df9cccc3; laws L1–L6 from
//! the frames note 54c88f72 §6.3, L7 from 34f70d6d §3). Names are
//! provisional, and Mara's.
//!
//! **Two layers.**
//! - **The event layer is a category of spans.** A span is a message: the
//!   commit that emitted it and the commit that received it, [`SpanId`] =
//!   emit ‖ receive, by *full* hash. It carries identity, direction and
//!   provenance. Spans compose only forward, when one's receive commit is
//!   the next one's emit commit ([`Path::then`]): lineage. Nothing inverts
//!   but an identity (L7).
//! - **The reading layer is a groupoid.** Each span carries one invertible
//!   transport ([`Span::reading`]): how the receiver's frame reads what the
//!   emitter's frame sent. Inverting a span means *reading it back*
//!   ([`Pushout::read_back`]): a transport on the same edge, never a new
//!   event, never an id. Holonomy lives here.
//!
//! [`kleinos`] takes two [`Party`]s (frames, relations, faces) and spans
//! between their frames, and glues: frames with one address are one frame,
//! every relation and every span is its own edge, keyed by address (a span
//! relation's address includes its full span id), and nothing else is added.
//! The room is a multigraph, so k spans between two frames are k parallel
//! edges. The result's interpretation is [`compose`]'s, and its hypothesis
//! is [`hypothesize`]'s ([`Pushout::hypothesize`]): this module doesn't
//! recompute either.
//!
//! **The emergent third** ([`Pushout::third`]) is the set of loops the spans
//! add: b₁(pushout) − b₁(a ∪ b) of them, read against a spanning forest
//! that takes the parties' own relations first and then the spans by id.
//! Each has its holonomy, compared exactly; a comma is one that isn't the
//! identity. k spans between two one-frame parties add k − 1 loops. The
//! trace DAG is not the third: it's the ground, kept by address.
//!
//! **Refused, typed, never a panic** ([`KleinosError`]): a hash that isn't a
//! full commit; a span from a commit to itself; an empty party; a frame no
//! party holds; one span id with two pairs of endpoints (a collision, not an
//! edge) or two readings; and whatever [`compose`] refuses. A reading that
//! doesn't invert never gets this far: [`Gl::new`] refuses it.
//!
//! **λ₂ is a reading, not a law** ([`Pushout::fiedler`]): measured on
//! demand, sealed with machine and method, never in the address. Between two
//! parties joined by k spans it is 2k: it counts messages.
//!
//! What this doesn't do: check that an emit commit is an ancestor of its
//! receive commit (that needs the commit DAG), or sign a span at both ends
//! (the record is attributed at both ends and sealed by one holder, Mara
//! 34f70d6d D4).
//!
//! A span id's 16-hex [`Label`] is for display (the live OTel span id). It
//! is never a key: it isn't ordered.
//!
//! ```compile_fail,E0599
//! use std::collections::BTreeSet;
//! use prismqueer::kleinos::{Commit, SpanId};
//! let c = |t: &str| Commit::new(&t.repeat(40)).expect("hex");
//! let id = SpanId::new(c("1"), c("2")).expect("two commits");
//! let mut keys = BTreeSet::new();
//! keys.insert(id.label());
//! ```
//!
//! The control: a full id is a key.
//!
//! ```
//! use std::collections::BTreeSet;
//! use prismqueer::kleinos::{Commit, SpanId};
//! let c = |t: &str| Commit::new(&t.repeat(40)).expect("hex");
//! let id = SpanId::new(c("1"), c("2")).expect("two commits");
//! let mut keys = BTreeSet::new();
//! keys.insert(id);
//! ```
//!
//! A path of spans has no inverse (L7): reading back is a transport.
//!
//! ```compile_fail,E0599
//! use prismqueer::kleinos::{Commit, Path, SpanId};
//! let c = |t: &str| Commit::new(&t.repeat(40)).expect("hex");
//! let id = SpanId::new(c("1"), c("2")).expect("two commits");
//! let _ = Path::span(&id).inverse();
//! ```
//!
//! The control: it composes forward.
//!
//! ```
//! use prismqueer::kleinos::{Commit, Path, SpanId};
//! let c = |t: &str| Commit::new(&t.repeat(40)).expect("hex");
//! let (s, t) = (
//!     SpanId::new(c("1"), c("2")).expect("two commits"),
//!     SpanId::new(c("2"), c("3")).expect("two commits"),
//! );
//! assert!(Path::span(&s).then(&Path::span(&t)).is_some());
//! ```

use std::collections::{BTreeMap, BTreeSet, VecDeque};
use std::fmt;

use terni::Imperfect;

use crate::band::Measurement;
use crate::field::Group;
use crate::gl::{Gl, Scalar};
use crate::hypothesis::{
    compose, hypothesize, Composite, CompositionError, Face, Frame, Hypothesis, Obstruction,
    Relation, RelationDraft,
};
use crate::oid::Oid;
use crate::room::RoomError;

// --- Errors -----------------------------------------------------------------------

/// Why spans and parties don't glue.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum KleinosError {
    /// Not a full commit hash: 40 or 64 lowercase hex digits.
    NotACommit(String),
    /// A span from a commit to itself: an identity, not an event.
    SameCommit(Commit),
    /// A party with no frames holds no event.
    EmptyParty,
    /// A relation, face or span names a frame no party holds.
    UnknownFrame(Oid),
    /// One span id between two different pairs of frames: a collision, not
    /// an edge.
    Collision(SpanId),
    /// One span id, one pair of frames, two readings.
    TwoReadings(SpanId),
    /// The glued frames don't compose: a type error.
    Composition(CompositionError),
}

// --- The event layer ----------------------------------------------------------------

/// A commit, by its full hash: 40 (SHA-1) or 64 (SHA-256) lowercase hex.
#[derive(Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Commit(String);

impl Commit {
    /// The commit with this full hash, if it is one.
    pub fn new(hex: &str) -> Result<Self, KleinosError> {
        let digit = |b: u8| b.is_ascii_digit() || (b'a'..=b'f').contains(&b);
        if matches!(hex.len(), 40 | 64) && hex.bytes().all(digit) {
            Ok(Commit(hex.to_owned()))
        } else {
            Err(KleinosError::NotACommit(hex.to_owned()))
        }
    }

    /// Its full hash.
    pub fn hex(&self) -> &str {
        &self.0
    }

    /// Its first 8 hex digits: half a label.
    fn short(&self) -> &str {
        &self.0[..8]
    }
}

/// A span's identity: the emit commit ‖ the receive commit, full hashes.
/// Derived from two addresses and nothing measured; its order carries the
/// direction. It can be named only once the receive commit exists.
#[derive(Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct SpanId {
    emit: Commit,
    receive: Commit,
}

impl SpanId {
    /// The span emitted at `emit` and received at `receive`.
    pub fn new(emit: Commit, receive: Commit) -> Result<Self, KleinosError> {
        if emit == receive {
            return Err(KleinosError::SameCommit(emit));
        }
        Ok(SpanId { emit, receive })
    }

    /// The commit that emitted it.
    pub fn emit(&self) -> &Commit {
        &self.emit
    }

    /// The commit that received it.
    pub fn receive(&self) -> &Commit {
        &self.receive
    }

    /// The 8 + 8 hex display label (an OTel span id). Scoped to a trace,
    /// not collision-resistant: never a key.
    pub fn label(&self) -> Label {
        Label(format!("{}{}", self.emit.short(), self.receive.short()))
    }

    /// Its canonical bytes: both full hashes, each length-delimited.
    pub(crate) fn encode(&self, out: &mut Vec<u8>) {
        for hex in [self.emit.hex(), self.receive.hex()] {
            out.extend_from_slice(&(hex.len() as u64).to_le_bytes());
            out.extend_from_slice(hex.as_bytes());
        }
    }
}

/// A span id's 16-hex display label. Comparable, printable, and on purpose
/// neither ordered nor hashed: it can't key a map.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Label(String);

impl Label {
    /// The 16 hex digits.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for Label {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

/// An arrow of the span category: a lineage from one commit to another,
/// spans in order, each received where the next is emitted. The identity at
/// a commit has no spans. There is no inverse (L7).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Path {
    source: Commit,
    target: Commit,
    spans: Vec<SpanId>,
}

impl Path {
    /// The identity at a commit.
    pub fn identity(at: Commit) -> Self {
        Path {
            source: at.clone(),
            target: at,
            spans: Vec::new(),
        }
    }

    /// One span, as an arrow emit → receive.
    pub fn span(id: &SpanId) -> Self {
        Path {
            source: id.emit.clone(),
            target: id.receive.clone(),
            spans: vec![id.clone()],
        }
    }

    /// First this, then `next`: defined only when `next` starts where this
    /// ends. Forward only.
    pub fn then(&self, next: &Path) -> Option<Path> {
        (self.target == next.source).then(|| Path {
            source: self.source.clone(),
            target: next.target.clone(),
            spans: self.spans.iter().chain(&next.spans).cloned().collect(),
        })
    }

    /// Where it starts.
    pub fn source(&self) -> &Commit {
        &self.source
    }

    /// Where it ends.
    pub fn target(&self) -> &Commit {
        &self.target
    }

    /// Its spans, in order.
    pub fn spans(&self) -> &[SpanId] {
        &self.spans
    }
}

// --- The reading layer ----------------------------------------------------------------

/// A span with its reading: an event between two frames, and how the
/// receiver's frame reads what the emitter's frame sent. A [`Relation`]
/// whose address includes the full span id, sealed by the holder of the
/// record (attributed at both ends, sealed by one).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Span<R: Scalar> {
    id: SpanId,
    relation: Relation<R>,
}

impl<R: Scalar> Span<R> {
    /// Span `id` from frame `emitter` to frame `receiver`, read as
    /// `reading`, sealed by `holder`.
    pub fn new(id: SpanId, emitter: Oid, receiver: Oid, reading: Gl<R>, holder: Oid) -> Self {
        let relation = RelationDraft::along(id.clone(), emitter, receiver, reading).seal(holder);
        Span { id, relation }
    }

    /// Its id.
    pub fn id(&self) -> &SpanId {
        &self.id
    }

    /// The frame that emitted it.
    pub fn emitter(&self) -> &Oid {
        self.relation.from()
    }

    /// The frame that received it.
    pub fn receiver(&self) -> &Oid {
        self.relation.to()
    }

    /// How the receiver reads it.
    pub fn reading(&self) -> &Gl<R> {
        self.relation.transport()
    }

    /// It, as a relation of the composite.
    pub fn relation(&self) -> &Relation<R> {
        &self.relation
    }
}

// --- Parties ----------------------------------------------------------------------------

/// A party: frames, the relations among them, and the faces it reports.
/// Never empty, and it names only its own frames. Canonical: frames by
/// address, relations and faces by address and signer, each once.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Party<R: Scalar> {
    frames: Vec<Frame>,
    relations: Vec<Relation<R>>,
    faces: Vec<Face>,
}

impl<R: Scalar> Party<R> {
    /// The party with these frames, relations and faces, if it holds a
    /// frame and every relation and face stays inside its frames.
    pub fn new(
        frames: Vec<Frame>,
        relations: Vec<Relation<R>>,
        faces: Vec<Face>,
    ) -> Result<Self, KleinosError> {
        if frames.is_empty() {
            return Err(KleinosError::EmptyParty);
        }
        let held: BTreeSet<&Oid> = frames.iter().map(Frame::oid).collect();
        let named = relations
            .iter()
            .flat_map(|r| [r.from(), r.to()])
            .chain(faces.iter().flat_map(|f| f.corners().iter()));
        if let Some(f) = named.into_iter().find(|f| !held.contains(f)) {
            return Err(KleinosError::UnknownFrame(f.clone()));
        }
        Ok(Party::canonical(frames, relations, faces))
    }

    fn canonical(frames: Vec<Frame>, relations: Vec<Relation<R>>, faces: Vec<Face>) -> Self {
        let frames: BTreeSet<Frame> = frames.into_iter().collect();
        let relations: BTreeMap<_, Relation<R>> = relations
            .into_iter()
            .map(|r| ((r.oid().clone(), r.signer().clone()), r))
            .collect();
        let faces: BTreeMap<_, Face> = faces
            .into_iter()
            .map(|f| ((f.oid().clone(), f.signer().clone()), f))
            .collect();
        Party {
            frames: frames.into_iter().collect(),
            relations: relations.into_values().collect(),
            faces: faces.into_values().collect(),
        }
    }

    /// Its frames, by address.
    pub fn frames(&self) -> &[Frame] {
        &self.frames
    }

    /// Its relations, spans included, by address then signer.
    pub fn relations(&self) -> &[Relation<R>] {
        &self.relations
    }

    /// Its faces, by address then signer.
    pub fn faces(&self) -> &[Face] {
        &self.faces
    }
}

// --- The pushout ------------------------------------------------------------------------

/// One loop a span adds: the span, then the forest's path from its
/// receiver back to its emitter, and the transport around it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Added<R: Scalar> {
    span: SpanId,
    frames: Vec<Oid>,
    relations: Vec<Oid>,
    holonomy: Gl<R>,
}

impl<R: Scalar> Added<R> {
    /// The span that closes it.
    pub fn span(&self) -> &SpanId {
        &self.span
    }

    /// The closed walk, emitter first and last.
    pub fn frames(&self) -> &[Oid] {
        &self.frames
    }

    /// The relations along it: `relations()[k]` joins `frames()[k]` and
    /// `frames()[k + 1]`; the first is the span's.
    pub fn relations(&self) -> &[Oid] {
        &self.relations
    }

    /// The transport around it, in path order, exact.
    pub fn holonomy(&self) -> &Gl<R> {
        &self.holonomy
    }

    /// Whether it's a comma: a round trip read back differently.
    pub fn is_comma(&self) -> bool {
        !self.holonomy.is_identity()
    }
}

/// How [`Pushout::fiedler`] is measured.
pub const FIEDLER_METHOD: &str =
    "L₀ = δ*δ on the span multigraph (room), dsyev('N'), second eigenvalue; counts messages";

/// Two parties glued along spans: the glued party, its composite, the event
/// layer, the readings, and the emergent third.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Pushout<R: Scalar> {
    party: Party<R>,
    composite: Composite<R>,
    events: BTreeMap<SpanId, (Oid, Oid)>,
    readings: BTreeMap<SpanId, Gl<R>>,
    third: Vec<Added<R>>,
}

impl<R: Scalar> Pushout<R> {
    /// Its address: the composite's (frames, relations with their span ids,
    /// faces). Signers and readings of λ₂ sit beside.
    pub fn oid(&self) -> &Oid {
        self.composite.oid()
    }

    /// The glued party: glue it again ([`kleinos`] is associative).
    pub fn party(&self) -> &Party<R> {
        &self.party
    }

    /// The composite's interpretation ([`compose`]).
    pub fn composite(&self) -> &Composite<R> {
        &self.composite
    }

    /// The event layer: every span, by full id, with its (emitter,
    /// receiver) frames. The parties' earlier spans included.
    pub fn events(&self) -> &BTreeMap<SpanId, (Oid, Oid)> {
        &self.events
    }

    /// How span `id`'s receiver reads it.
    pub fn reading(&self, id: &SpanId) -> Option<&Gl<R>> {
        self.readings.get(id)
    }

    /// Span `id` read back: what the emitter's frame makes of the
    /// receiver's reading. The inverse transport, on the same edge. Not an
    /// event: it has no id.
    pub fn read_back(&self, id: &SpanId) -> Option<Gl<R>> {
        self.readings.get(id).map(Group::inverse)
    }

    /// The loops the spans add, each with its holonomy.
    pub fn third(&self) -> &[Added<R>] {
        &self.third
    }

    /// The loops of the third whose holonomy isn't the identity.
    pub fn commas(&self) -> Vec<&Added<R>> {
        self.third.iter().filter(|l| l.is_comma()).collect()
    }

    /// The hypothesis over the glued party: [`hypothesize`]'s.
    pub fn hypothesize(&self) -> Imperfect<Hypothesis, CompositionError, BTreeSet<Obstruction>> {
        hypothesize(&self.party.frames, &self.party.relations, &self.party.faces)
    }

    /// λ₂ of the room multigraph, measured: a reading beside the address,
    /// never a law. Between two parties joined by k spans it is 2k.
    pub fn fiedler(&self) -> Result<Measurement, RoomError> {
        let spectrum = crate::room::laplacian(self.composite.graph())
            .measure()
            .spectrum()?;
        Ok(Measurement::here(
            spectrum.get(1).copied().unwrap_or(0.0),
            FIEDLER_METHOD,
        ))
    }
}

/// Glue parties `a` and `b` along `spans`: frames with one address become
/// one, every relation and span is its own edge (keyed by address, a span's
/// by its full id), and nothing else is added. Refused, typed, when a span
/// names a frame neither party holds, when one span id has two pairs of
/// endpoints or two readings, or when the result doesn't compose.
pub fn kleinos<R: Scalar>(
    a: &Party<R>,
    b: &Party<R>,
    spans: &[Span<R>],
) -> Result<Pushout<R>, KleinosError> {
    let frames: Vec<Frame> = a.frames.iter().chain(&b.frames).cloned().collect();
    let held: BTreeSet<&Oid> = frames.iter().map(Frame::oid).collect();
    let ends = spans.iter().flat_map(|s| [s.emitter(), s.receiver()]);
    if let Some(f) = ends.into_iter().find(|f| !held.contains(f)) {
        return Err(KleinosError::UnknownFrame(f.clone()));
    }

    // The parties' relations, then the spans: one id, one edge.
    let given: Vec<&Relation<R>> = a.relations.iter().chain(&b.relations).collect();
    let all: Vec<&Relation<R>> = given
        .iter()
        .copied()
        .chain(spans.iter().map(Span::relation))
        .collect();
    let mut events: BTreeMap<SpanId, (Oid, Oid)> = BTreeMap::new();
    let mut readings: BTreeMap<SpanId, Gl<R>> = BTreeMap::new();
    for r in &all {
        let Some(id) = r.span() else { continue };
        let at = (r.from().clone(), r.to().clone());
        match (events.get(id), readings.get(id)) {
            (Some(was), _) if *was != at => return Err(KleinosError::Collision(id.clone())),
            (_, Some(read)) if read != r.transport() => {
                return Err(KleinosError::TwoReadings(id.clone()))
            }
            _ => {
                events.insert(id.clone(), at);
                readings.insert(id.clone(), r.transport().clone());
            }
        }
    }

    let faces: Vec<Face> = a.faces.iter().chain(&b.faces).cloned().collect();
    let party = Party::canonical(frames, all.into_iter().cloned().collect(), faces);
    let composite = compose(&party.frames, &party.relations, &party.faces)
        .map_err(KleinosError::Composition)?;

    let known: BTreeSet<&SpanId> = given.iter().filter_map(|r| r.span()).collect();
    let fresh: BTreeMap<&SpanId, &Relation<R>> = spans
        .iter()
        .filter(|s| !known.contains(s.id()))
        .map(|s| (s.id(), s.relation()))
        .collect();
    let third = added(&party.frames, &given, &fresh);

    Ok(Pushout {
        party,
        composite,
        events,
        readings,
        third,
    })
}

/// The loops the fresh spans add: a spanning forest grown from the
/// parties' relations (by address), then the spans (by id); each span that
/// closes a cycle closes one loop.
fn added<R: Scalar>(
    frames: &[Frame],
    given: &[&Relation<R>],
    fresh: &BTreeMap<&SpanId, &Relation<R>>,
) -> Vec<Added<R>> {
    let mut forest = Forest::new(frames);
    let given: BTreeMap<&Oid, &Relation<R>> = given.iter().map(|r| (r.oid(), *r)).collect();
    for r in given.values() {
        forest.grow(r);
    }
    let mut third = Vec::new();
    for (&id, &s) in fresh {
        match forest.path(s.to(), s.from()) {
            None => forest.grow(s),
            Some(steps) => {
                let mut frames = vec![s.from().clone(), s.to().clone()];
                let mut relations = vec![s.oid().clone()];
                let mut holonomy = s.transport().clone();
                for (r, forward) in steps {
                    let (next, t) = match forward {
                        true => (r.to(), r.transport().clone()),
                        false => (r.from(), r.transport().inverse()),
                    };
                    frames.push(next.clone());
                    relations.push(r.oid().clone());
                    holonomy = holonomy.compose(&t);
                }
                third.push(Added {
                    span: id.clone(),
                    frames,
                    relations,
                    holonomy,
                });
            }
        }
    }
    third
}

/// A spanning forest of frames: tree relations by frame, grown one
/// relation at a time.
struct Forest<'r, R: Scalar> {
    tree: BTreeMap<Oid, Vec<&'r Relation<R>>>,
}

impl<'r, R: Scalar> Forest<'r, R> {
    fn new(frames: &[Frame]) -> Self {
        Forest {
            tree: frames
                .iter()
                .map(|f| (f.oid().clone(), Vec::new()))
                .collect(),
        }
    }

    /// Add `r` if it joins two trees; a relation that closes a cycle is
    /// left out.
    fn grow(&mut self, r: &'r Relation<R>) {
        if self.path(r.from(), r.to()).is_none() {
            for end in [r.from(), r.to()] {
                self.tree.entry(end.clone()).or_default().push(r);
            }
        }
    }

    /// The tree path from `start` to `end`, breadth-first: each relation,
    /// and whether it's walked from its `from` to its `to`. `None` if they
    /// are in different trees; empty if they're one frame.
    fn path(&self, start: &Oid, end: &Oid) -> Option<Vec<(&'r Relation<R>, bool)>> {
        let mut back: BTreeMap<&Oid, (&Oid, &'r Relation<R>)> = BTreeMap::new();
        let mut queue = VecDeque::from([start]);
        let mut seen = BTreeSet::from([start]);
        while let Some(u) = queue.pop_front() {
            if u == end {
                let mut steps = Vec::new();
                let mut at = u;
                while let Some(&(prev, r)) = back.get(at) {
                    steps.push((r, r.from() == prev));
                    at = prev;
                }
                steps.reverse();
                return Some(steps);
            }
            for &r in self.tree.get(u).into_iter().flatten() {
                let w = if r.from() == u { r.to() } else { r.from() };
                if seen.insert(w) {
                    back.insert(w, (u, r));
                    queue.push_back(w);
                }
            }
        }
        None
    }
}
