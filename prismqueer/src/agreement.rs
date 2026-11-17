//! The kernel check: is a signed selection inside the consistent readings?
//! (prism-019 ask 4; bullet-up-007 test 5.) Names are provisional.
//!
//! - **Items carry readings**, a 0-cochain s ∈ C⁰ over the ring the
//!   relations live in ([`Z`](crate::room::Z), [`ZMod`](crate::room::ZMod),
//!   [`QModOne`](crate::room::QModOne)).
//! - **Relations are constraints**: each is an edge of a multigraph over the
//!   items, `(from, to, t)`, saying s(to) − s(from) = t exactly. Together
//!   they are a 1-cochain c, the required values.
//! - **The consistent readings** are {s : δs = c}. When c is exact
//!   ([`solve`] gives a potential g) they are the coset g + ker δ, and
//!   ker δ = H⁰ of the graph: one free direction per component, a shift of
//!   every item in it by the same ring element. When c is genuine, there are
//!   none, and [`solve`]'s cycles witness it.
//! - **What governs is a signed selection** (bullet-up-007 item 3): an
//!   adjudicator's [`Selection`], sealed like a ruler. [`check`] says
//!   whether it lies inside the consistent readings ([`Verdict::In`], with
//!   the edge-by-edge certificate), outside them ([`Verdict::Out`], the
//!   violated edges located), or that there are none to lie in
//!   ([`Verdict::NeedsRuling`], with the witness cycles). The last never
//!   picks a reading and is never `Out`: no selection could be `In`.
//!
//! **No "settled" anywhere.** The consistent readings are exposed as data,
//! [`Consistent`]: empty, or a particular solution plus its free directions.
//! There is no flag for "one reading", and a dimension of 1 means one
//! component, not one reading (bullet-up-007 item 8).
//!
//! Membership: terni's [`Membership`](terni::fractal::Membership) has the
//! `In` / `Out` arms, but its third, `Undecided`, is "a fact about the
//! observer, never a claim about the set". An empty kernel is a claim about
//! the set, proved by its cycles, so it gets its own arm here.
//!
//! An unsigned selection can't be checked:
//!
//! ```compile_fail,E0308
//! use prismqueer::agreement::*;
//! use prismqueer::room::Z;
//! let relations = Relations::new(&(), [], [(0, 1, Z::from(3))]).unwrap();
//! let draft = SelectionDraft::new([(0, Z::from(0)), (1, Z::from(3))]);
//! let _ = check(&relations, &draft);
//! ```
//!
//! The control: sealing it is the only way to a [`Selection`], and it
//! checks.
//!
//! ```
//! use prismqueer::agreement::*;
//! use prismqueer::oid::Oid;
//! use prismqueer::room::Z;
//! let relations = Relations::new(&(), [], [(0, 1, Z::from(3))]).unwrap();
//! let draft = SelectionDraft::new([(0, Z::from(0)), (1, Z::from(3))]);
//! let selection = draft.seal(Oid::new("adjudicator"));
//! assert!(matches!(check(&relations, &selection), Ok(Verdict::In(_))));
//! ```

use std::collections::BTreeMap;

use num_bigint::BigInt;

use crate::algedonic::Signer;
use crate::field::{Edge, Graph};
use crate::oid::{Oid, Scope};
use crate::room::{
    coboundary, solve, Cycle, Exactly, Exactness, QModOne, RingPart, RoomError, ZMod, Z,
};
use crate::spectral::kleinos::VertexId;

mod sealed {
    pub trait Sealed {}
    impl Sealed for crate::room::Z {}
    impl Sealed for crate::room::ZMod {}
    impl Sealed for crate::room::QModOne {}
}

/// What a reading can be: an exact coefficient with one canonical encoding,
/// so a selection has an address. [`Z`], [`ZMod`] and [`QModOne`].
pub trait Reading: Exactly + sealed::Sealed {
    /// Its canonical bytes: a ring tag, then self-delimiting integers.
    fn encode(&self, out: &mut Vec<u8>);
}

/// A self-delimiting integer: u64le(length) ‖ two's-complement bytes, LE.
fn integer(out: &mut Vec<u8>, n: &BigInt) {
    let bytes = n.to_signed_bytes_le();
    out.extend_from_slice(&(bytes.len() as u64).to_le_bytes());
    out.extend_from_slice(&bytes);
}

impl Reading for Z {
    /// `0x00 ‖ n`.
    fn encode(&self, out: &mut Vec<u8>) {
        out.push(0);
        integer(out, self.value());
    }
}

impl Reading for ZMod {
    /// `0x01 ‖ n ‖ residue`.
    fn encode(&self, out: &mut Vec<u8>) {
        out.push(1);
        integer(out, self.modulus().value());
        integer(out, self.residue());
    }
}

impl Reading for QModOne {
    /// `0x02 ‖ numerator ‖ denominator`.
    fn encode(&self, out: &mut Vec<u8>) {
        out.push(2);
        integer(out, self.numerator());
        integer(out, self.denominator());
    }
}

// --- Relations ---------------------------------------------------------------

/// The relations among items: a multigraph over them, and on each edge the
/// exact transport its two readings must satisfy. The required values c ∈ C¹,
/// in the edges' canonical orientation (smaller end first).
#[derive(Clone, Debug, PartialEq)]
pub struct Relations<V: Reading> {
    ring: V::Ring,
    graph: Graph,
    required: Vec<V>,
}

impl<V: Reading> Relations<V> {
    /// Items (beyond those the relations name) and relations `(from, to, t)`:
    /// s(to) − s(from) = t. A pair given twice is two parallel relations; a
    /// relation from an item to itself requires t = 0.
    ///
    /// Refused ([`RoomError::Modulus`]) when a value lives in another ring
    /// than `ring`.
    pub fn new(
        ring: &V::Ring,
        items: impl IntoIterator<Item = VertexId>,
        relations: impl IntoIterator<Item = (VertexId, VertexId, V)>,
    ) -> Result<Self, RoomError> {
        let mut seen: BTreeMap<(VertexId, VertexId), usize> = BTreeMap::new();
        let mut given: BTreeMap<Edge, V> = BTreeMap::new();
        let mut ends = Vec::new();
        for (from, to, t) in relations {
            ring.admits(t.ring())?;
            let key = (from.min(to), from.max(to));
            let nth = seen.entry(key).or_insert(0);
            let t = if from <= to { t } else { t.neg() };
            given.insert(Edge::between(from, to, *nth), t);
            *nth += 1;
            ends.push((from, to));
        }
        let graph = Graph::new(items, ends);
        let required = graph.edges().map(|e| given[&e].clone()).collect();
        Ok(Relations {
            ring: ring.clone(),
            graph,
            required,
        })
    }

    /// The ring the readings live in.
    pub fn ring(&self) -> &V::Ring {
        &self.ring
    }

    /// The items and their relations, as a multigraph.
    pub fn graph(&self) -> &Graph {
        &self.graph
    }

    /// c: one required value per edge of [`graph`](Self::graph), in its
    /// order, along the edge from its smaller end.
    pub fn required(&self) -> &[V] {
        &self.required
    }

    /// The consistent readings, {s : δs = c}, as data.
    ///
    /// [`solve`] decides it: a potential g gives the coset g + H⁰, g 0 at
    /// each component's smallest item; genuine cycles give none. Refused only
    /// as `solve` refuses, which these relations, built in their ring, never
    /// are.
    pub fn consistent(&self) -> Result<Consistent<V>, RoomError> {
        Ok(match solve(&self.ring, &self.graph, &self.required)? {
            Exactness::Genuine(cycles) => Consistent::Empty(cycles),
            Exactness::Exact(g) => Consistent::Readings {
                particular: self.graph.vertices().zip(g).collect(),
                free: components(&self.graph),
            },
        })
    }
}

/// H⁰ of the graph over any ring: the locally constant readings, one free
/// direction per component, by smallest item. Union by smaller root, so
/// each root is its component's smallest item.
fn components(graph: &Graph) -> Vec<Direction> {
    let mut parent: BTreeMap<VertexId, VertexId> = graph.vertices().map(|v| (v, v)).collect();
    let root = |parent: &BTreeMap<VertexId, VertexId>, mut v: VertexId| {
        while parent[&v] != v {
            v = parent[&v];
        }
        v
    };
    for edge in graph.edges() {
        let (a, b) = edge.ends();
        let (ra, rb) = (root(&parent, a), root(&parent, b));
        parent.insert(ra.max(rb), ra.min(rb));
    }
    let mut by: BTreeMap<VertexId, Vec<VertexId>> = BTreeMap::new();
    for v in graph.vertices() {
        by.entry(root(&parent, v)).or_default().push(v);
    }
    by.into_values().map(|items| Direction { items }).collect()
}

/// The consistent readings of a set of relations. Never a scalar.
#[derive(Clone, Debug, PartialEq)]
pub enum Consistent<V> {
    /// None: the relations are curved. Every basis cycle whose twist isn't
    /// zero, as [`solve`]'s. Never empty. Choosing what governs here needs a
    /// declared ruling.
    Empty(Vec<Cycle<V>>),
    /// The coset `particular + Σ shiftₖ · freeₖ`, one shift per free
    /// direction, each any element of the ring.
    Readings {
        /// One consistent reading, 0 at each component's smallest item.
        particular: BTreeMap<VertexId, V>,
        /// H⁰ of the graph: one direction per component.
        free: Vec<Direction>,
    },
}

/// A free direction of the consistent readings: shifting every item here by
/// the same ring element keeps every relation. One per component.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Direction {
    items: Vec<VertexId>,
}

impl Direction {
    /// The component's items, ascending.
    pub fn items(&self) -> &[VertexId] {
        &self.items
    }
}

// --- Selections ---------------------------------------------------------------

/// An unsigned selection: a reading for each item. It governs nothing until
/// sealed, and can't be checked.
#[derive(Clone, Debug, PartialEq)]
pub struct SelectionDraft<V: Reading> {
    readings: BTreeMap<VertexId, V>,
}

impl<V: Reading> SelectionDraft<V> {
    /// The readings, by item. An item given twice keeps its last reading.
    pub fn new(readings: impl IntoIterator<Item = (VertexId, V)>) -> Self {
        SelectionDraft {
            readings: readings.into_iter().collect(),
        }
    }

    /// The canonical preimage: `"selection\0" ‖ u64le(count) ‖ (u64le(item)
    /// ‖ reading)…`, items ascending, every reading self-delimiting
    /// ([`Reading::encode`]). The signer is never in it.
    pub fn preimage(&self) -> Vec<u8> {
        let mut bytes = b"selection\0".to_vec();
        bytes.extend_from_slice(&(self.readings.len() as u64).to_le_bytes());
        for (item, reading) in &self.readings {
            bytes.extend_from_slice(&item.to_le_bytes());
            reading.encode(&mut bytes);
        }
        bytes
    }

    /// Its address: the preimage, in the public scope, as a ruler's.
    pub fn oid(&self) -> Oid {
        Oid::hash(&Scope::public(), &self.preimage())
    }

    /// Seal the selection as `by`'s: the only way to a [`Selection`].
    pub fn seal(self, by: Oid) -> Selection<V> {
        let over = self.oid();
        Selection {
            draft: self,
            signer: Signer::mint(by, over),
        }
    }
}

/// A signed selection: a [`SelectionDraft`] sealed by its [`Signer`]. No
/// default, no public constructor.
#[derive(Clone, Debug, PartialEq)]
pub struct Selection<V: Reading> {
    draft: SelectionDraft<V>,
    signer: Signer,
}

impl<V: Reading> Selection<V> {
    /// Its address, the draft's: the signer is beside it.
    pub fn oid(&self) -> Oid {
        self.draft.oid()
    }

    /// Who sealed it, over which address.
    pub fn signer(&self) -> &Signer {
        &self.signer
    }

    /// The readings, by item.
    pub fn readings(&self) -> &BTreeMap<VertexId, V> {
        &self.draft.readings
    }
}

// --- The check ----------------------------------------------------------------

/// One relation, checked: what it requires, and what the selection gives.
#[derive(Clone, Debug, PartialEq)]
pub struct Checked<V> {
    edge: Edge,
    required: V,
    found: V,
}

impl<V> Checked<V> {
    /// The relation's edge.
    pub fn edge(&self) -> Edge {
        self.edge
    }
    /// c(e): s(larger end) − s(smaller end), as required.
    pub fn required(&self) -> &V {
        &self.required
    }
    /// (δs)(e): the same difference, as selected.
    pub fn found(&self) -> &V {
        &self.found
    }
}

/// Why a selection is inside the consistent readings.
#[derive(Clone, Debug, PartialEq)]
pub struct Certificate<V> {
    checked: Vec<Checked<V>>,
    coordinates: Vec<V>,
}

impl<V> Certificate<V> {
    /// Every relation, checked: `found == required` on each.
    pub fn checked(&self) -> &[Checked<V>] {
        &self.checked
    }
    /// Where in the coset: the selection is `particular + Σ coordinatesₖ ·
    /// freeₖ`, one coordinate per [`Consistent::Readings`] direction.
    pub fn coordinates(&self) -> &[V] {
        &self.coordinates
    }
}

/// Whether a signed selection lies inside the consistent readings.
#[derive(Clone, Debug, PartialEq)]
pub enum Verdict<V> {
    /// Inside: every relation holds, with the certificate.
    In(Certificate<V>),
    /// Outside: the relations it breaks, each with what it required and what
    /// it found. Never empty.
    Out(Vec<Checked<V>>),
    /// There are no consistent readings to be inside: the relations are
    /// curved, and these cycles witness it. What governs needs a declared
    /// ruling; no selection is picked or rejected here.
    NeedsRuling(Vec<Cycle<V>>),
}

/// Why a selection couldn't be checked against the relations.
#[derive(Clone, Debug, PartialEq)]
pub enum Refused {
    /// An item of the relations has no reading.
    Missing(VertexId),
    /// A reading for an item the relations don't have.
    Unknown(VertexId),
    /// A reading in another ring.
    Room(RoomError),
}

/// Check a signed selection against the relations, exactly, in their ring.
///
/// The selection must read exactly the relations' items, in their ring
/// ([`Refused`]). Then: no consistent readings is
/// [`NeedsRuling`](Verdict::NeedsRuling) whatever the selection; otherwise
/// δs is compared with c on every edge, `In` when all hold and `Out` with
/// those that don't.
pub fn check<V: Reading>(
    relations: &Relations<V>,
    selection: &Selection<V>,
) -> Result<Verdict<V>, Refused> {
    let s = selection.readings();
    let graph = &relations.graph;
    if let Some(v) = graph.vertices().find(|v| !s.contains_key(v)) {
        return Err(Refused::Missing(v));
    }
    if let Some(&v) = s.keys().find(|&&v| !graph.contains_vertex(v)) {
        return Err(Refused::Unknown(v));
    }
    // The same items, both ascending: s is a vector of C⁰.
    let readings: Vec<V> = s.values().cloned().collect();
    let found = coboundary(graph)
        .act(&relations.ring, &readings)
        .map_err(Refused::Room)?;
    let (particular, free) = match relations.consistent().map_err(Refused::Room)? {
        Consistent::Empty(cycles) => return Ok(Verdict::NeedsRuling(cycles)),
        Consistent::Readings { particular, free } => (particular, free),
    };
    let checked: Vec<Checked<V>> = graph
        .edges()
        .zip(&relations.required)
        .zip(found)
        .map(|((edge, required), found)| Checked {
            edge,
            required: required.clone(),
            found,
        })
        .collect();
    let broken: Vec<Checked<V>> = checked
        .iter()
        .filter(|c| c.found != c.required)
        .cloned()
        .collect();
    if !broken.is_empty() {
        return Ok(Verdict::Out(broken));
    }
    // In: on each component s − particular is constant; read it at the
    // component's smallest item.
    let coordinates = free
        .iter()
        .filter_map(|d| d.items.first())
        .map(|v| s[v].add(&particular[v].neg()))
        .collect();
    Ok(Verdict::In(Certificate {
        checked,
        coordinates,
    }))
}
