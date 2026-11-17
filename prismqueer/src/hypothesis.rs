//! `prismqueer::hypothesis` — frames, their composite, and the hypothesis
//! over it (prism-021; Mara 54c88f72 §4.1–§4.3, §5, §11 "For prism";
//! decided by Alex as D1–D3, mirror STATE #77–#78). Names are provisional,
//! and Mara's.
//!
//! - **A [`Frame`] is a site** of the decided connection: an address (its
//!   reader, hashed with its stalk dimension) and a stalk ℚⁿ or ℤⁿ. Its
//!   identity is the address, never its method. No metric: the ruler that
//!   adjudicates a reading sits beside the frame, never in it (§4.1).
//! - **A [`Relation`] transports readings** between two frames: an
//!   invertible exact [`Gl`] from one stalk to the other. "Signed" means
//!   *attributed*: a [`Signer`] beside its address, sealed like a ruler, no
//!   default ([`RelationDraft::seal`]). One relation signed twice is one
//!   edge with two signers.
//! - **The [`Composite`] is the composite's interpretation**: everything a
//!   function of the key (frames, relations, faces), so recomputable:
//!   - *agreement*: a basis of H⁰, the readings consistent across every
//!     relation, one [`Interpretation`] per frame;
//!   - *holonomy*: per located loop, the loop, its holonomy (an exact `Gl`)
//!     and its cycle certificate. A basis of loops is only claimed with its
//!     tree named ([`Holonomy::tree`]). Holonomy, **not H¹**: for matrix
//!     transports H¹ counts the fixed directions plus r(β₁ − 1), not the
//!     twist (§4.3, P2–P3);
//!   - *curvature* on declared faces ([`FaceDraft::seal`]: a face is
//!     someone's report that a loop closes);
//!   - *provenance*: every signer.
//! - **A [`Hypothesis`] is what no recompute establishes**: per located
//!   loop, the set of closures that would remove it ([`Closure`]: fill,
//!   twist, absorb, share, bias), grounded on the composite by address.
//!   Unranked: a menu (ADO's Offer), with no ranking field. A ranking is a
//!   separate record under a named ruler.
//!
//! [`hypothesize`] is `Success` when flat (nothing to hypothesise),
//! `Partial` with the located loops and curved faces as its loss, and
//! `Failure` only on a type error ([`CompositionError`]). The empty input
//! is the unit of composition: `Success`, never `Failure`.
//!
//! No metric is referenced anywhere on this path. Composition (H⁰,
//! holonomy, located loops, curvature) needs none; a metric enters only
//! for lengths, angles, thresholds and averages, and an average is a
//! quotient of the composite after a gauge and a ruler (D1; Theorems A–D).
//!
//! A relation must be signed: a draft can't be composed.
//!
//! ```compile_fail,E0308
//! use prismqueer::field::Group;
//! use prismqueer::gl::Gl;
//! use prismqueer::hypothesis::*;
//! use prismqueer::oid::Oid;
//! use prismqueer::room::Q;
//! let (a, b) = (Frame::new(Oid::new("A"), 2), Frame::new(Oid::new("B"), 2));
//! let draft = RelationDraft::new(a.oid().clone(), b.oid().clone(), Gl::<Q>::identity());
//! let _ = hypothesize(&[a, b], &[draft], &[]);
//! ```
//!
//! The control: sealed, it composes.
//!
//! ```
//! use prismqueer::field::Group;
//! use prismqueer::gl::Gl;
//! use prismqueer::hypothesis::*;
//! use prismqueer::oid::Oid;
//! use prismqueer::room::Q;
//! let (a, b) = (Frame::new(Oid::new("A"), 2), Frame::new(Oid::new("B"), 2));
//! let draft = RelationDraft::new(a.oid().clone(), b.oid().clone(), Gl::<Q>::identity());
//! let relation = draft.seal(Oid::new("A"));
//! assert!(hypothesize(&[a, b], &[relation], &[]).is_ok());
//! ```
//!
//! A hypothesis has no ranking: its alternatives are a set per loop.
//!
//! ```compile_fail,E0599
//! use prismqueer::hypothesis::*;
//! use prismqueer::room::Q;
//! use terni::Imperfect;
//! if let Imperfect::Success(h) = hypothesize::<Q>(&[], &[], &[]) {
//!     let _ = h.ranking();
//! }
//! ```
//!
//! The control: its ground and its alternatives.
//!
//! ```
//! use prismqueer::hypothesis::*;
//! use prismqueer::room::Q;
//! use terni::Imperfect;
//! if let Imperfect::Success(h) = hypothesize::<Q>(&[], &[], &[]) {
//!     let _ = (h.ground(), h.alternatives());
//! }
//! ```

use std::collections::{BTreeMap, BTreeSet};

use terni::Imperfect;

use crate::algedonic::Signer;
use crate::field::{check_flat, Edge, Field, FieldError, Flatness, Graph, Loop};
use crate::gl::{Gl, Scalar};
use crate::oid::{Oid, Scope};
use crate::room::{Coefficient, Z};
use crate::spectral::join::VertexId;

// --- Frames, relations, faces ---------------------------------------------------

/// A frame: a site with a stalk of dimension `dim`. Its address is
/// `"frame\0" ‖ bytes(reader) ‖ u64le(dim)`, in the public scope.
#[derive(Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Frame {
    oid: Oid,
    reader: Oid,
    dim: usize,
}

impl Frame {
    /// The frame read by `reader` (the address of its basis and reader),
    /// with a stalk of dimension `dim`.
    pub fn new(reader: Oid, dim: usize) -> Self {
        let mut bytes = b"frame\0".to_vec();
        delimited(&mut bytes, reader.as_str().as_bytes());
        bytes.extend_from_slice(&(dim as u64).to_le_bytes());
        Frame {
            oid: Oid::hash(&Scope::public(), &bytes),
            reader,
            dim,
        }
    }

    /// Its address.
    pub fn oid(&self) -> &Oid {
        &self.oid
    }

    /// Its reader.
    pub fn reader(&self) -> &Oid {
        &self.reader
    }

    /// Its stalk dimension.
    pub fn dim(&self) -> usize {
        self.dim
    }
}

/// An unsigned relation: a transport from one frame to another, by
/// address. It relates nothing until sealed.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct RelationDraft<R: Scalar> {
    from: Oid,
    to: Oid,
    transport: Gl<R>,
}

impl<R: Scalar> RelationDraft<R> {
    /// Readings at `from` reach `to` as `transport` · reading.
    pub fn new(from: Oid, to: Oid, transport: Gl<R>) -> Self {
        RelationDraft {
            from,
            to,
            transport,
        }
    }

    /// Its address: `"relation\0" ‖ bytes(from) ‖ bytes(to) ‖ transport`.
    /// The signer is never in it.
    pub fn oid(&self) -> Oid {
        let mut bytes = b"relation\0".to_vec();
        delimited(&mut bytes, self.from.as_str().as_bytes());
        delimited(&mut bytes, self.to.as_str().as_bytes());
        self.transport.encode(&mut bytes);
        Oid::hash(&Scope::public(), &bytes)
    }

    /// Seal it as `by`'s: the only way to a [`Relation`].
    pub fn seal(self, by: Oid) -> Relation<R> {
        let signer = Signer::mint(by, self.oid());
        Relation {
            draft: self,
            signer,
        }
    }
}

/// A signed relation: a [`RelationDraft`] sealed by its [`Signer`]. No
/// default, no public constructor.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Relation<R: Scalar> {
    draft: RelationDraft<R>,
    signer: Signer,
}

impl<R: Scalar> Relation<R> {
    /// Its address, the draft's: the signer is beside it.
    pub fn oid(&self) -> &Oid {
        self.signer.over()
    }

    /// Who signed it.
    pub fn signer(&self) -> &Signer {
        &self.signer
    }

    /// The frame it reads from.
    pub fn from(&self) -> &Oid {
        &self.draft.from
    }

    /// The frame it reads into.
    pub fn to(&self) -> &Oid {
        &self.draft.to
    }

    /// The transport.
    pub fn transport(&self) -> &Gl<R> {
        &self.draft.transport
    }
}

/// An unsigned face: a triangle of frames declared filled.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct FaceDraft {
    corners: [Oid; 3],
}

impl FaceDraft {
    /// The triangle on these frames, by address.
    pub fn new(corners: [Oid; 3]) -> Self {
        FaceDraft { corners }
    }

    /// Its address: `"face\0"` then its corners, ascending (one triangle,
    /// one address, whichever way round it was named).
    pub fn oid(&self) -> Oid {
        let mut corners = self.corners.clone();
        corners.sort();
        let mut bytes = b"face\0".to_vec();
        for c in &corners {
            delimited(&mut bytes, c.as_str().as_bytes());
        }
        Oid::hash(&Scope::public(), &bytes)
    }

    /// Seal it as `by`'s report: the only way to a [`Face`].
    pub fn seal(self, by: Oid) -> Face {
        let signer = Signer::mint(by, self.oid());
        Face {
            draft: self,
            signer,
        }
    }
}

/// A reported face: someone's report that the loop around it closes, or
/// how it doesn't (curvature on the face).
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Face {
    draft: FaceDraft,
    signer: Signer,
}

impl Face {
    /// Its address: the signer is beside it.
    pub fn oid(&self) -> &Oid {
        self.signer.over()
    }

    /// Who reported it.
    pub fn signer(&self) -> &Signer {
        &self.signer
    }

    /// Its corners, as named.
    pub fn corners(&self) -> &[Oid; 3] {
        &self.draft.corners
    }
}

/// A reading inside one frame: a value that carries its frame by address.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Interpretation<R> {
    frame: Oid,
    value: Vec<R>,
}

impl<R> Interpretation<R> {
    /// The frame it is read in.
    pub fn frame(&self) -> &Oid {
        &self.frame
    }

    /// The reading, in that frame's stalk.
    pub fn value(&self) -> &[R] {
        &self.value
    }
}

// --- The composite ---------------------------------------------------------------

/// A located loop's address: its closed walk, frame and relation by
/// address, in the field's canonical orientation.
#[derive(Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct LoopId(Oid);

impl LoopId {
    /// The address.
    pub fn oid(&self) -> &Oid {
        &self.0
    }
}

/// One loop whose holonomy isn't the identity and that bounds no declared
/// face: where the frames can't be brought to one reading.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Located<R: Scalar> {
    id: LoopId,
    frames: Vec<Oid>,
    relations: Vec<Oid>,
    holonomy: Gl<R>,
    certificate: Vec<Z>,
}

impl<R: Scalar> Located<R> {
    /// Its address.
    pub fn id(&self) -> &LoopId {
        &self.id
    }

    /// The closed walk, first frame repeated last: starting at its smallest
    /// frame address, heading to the smaller neighbour.
    pub fn frames(&self) -> &[Oid] {
        &self.frames
    }

    /// The relations along it: `relations()[k]` joins `frames()[k]` and
    /// `frames()[k + 1]`.
    pub fn relations(&self) -> &[Oid] {
        &self.relations
    }

    /// The transport around it, in path order, exact.
    pub fn holonomy(&self) -> &Gl<R> {
        &self.holonomy
    }

    /// Its cycle certificate: the 1-chain z on [`Composite::graph`]'s edges
    /// (+1 walked from an edge's smaller end, −1 against) with ∂z = 0.
    pub fn certificate(&self) -> &[Z] {
        &self.certificate
    }
}

/// The holonomy of a composite: the located loops, and the spanning tree
/// whose fundamental cycles they are. Only the set of loops is canonical;
/// a basis is claimed with its tree.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Holonomy<R: Scalar> {
    tree: Vec<Oid>,
    loops: Vec<Located<R>>,
}

impl<R: Scalar> Holonomy<R> {
    /// The relations of the spanning forest, in edge order.
    pub fn tree(&self) -> &[Oid] {
        &self.tree
    }

    /// The located loops.
    pub fn loops(&self) -> &[Located<R>] {
        &self.loops
    }
}

/// Curvature on a declared face: the face doesn't close.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Curvature<R: Scalar> {
    face: Oid,
    frames: Vec<Oid>,
    holonomy: Gl<R>,
}

impl<R: Scalar> Curvature<R> {
    /// The face's address.
    pub fn face(&self) -> &Oid {
        &self.face
    }

    /// Around the face, canonically, first frame repeated last.
    pub fn frames(&self) -> &[Oid] {
        &self.frames
    }

    /// The transport around it.
    pub fn holonomy(&self) -> &Gl<R> {
        &self.holonomy
    }
}

/// The composite's interpretation: a function of the key (frames,
/// relations, faces). Recomputable, so cacheable and shareable wherever its
/// inputs are.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Composite<R: Scalar> {
    oid: Oid,
    graph: Graph,
    edges: Vec<Oid>,
    agreement: Vec<Vec<Interpretation<R>>>,
    holonomy: Holonomy<R>,
    curvature: Vec<Curvature<R>>,
    provenance: Vec<Signer>,
}

impl<R: Scalar> Composite<R> {
    /// Its address: `"composite\0"`, then the frames, relations and faces,
    /// each counted and ascending by address. Signers sit beside.
    pub fn oid(&self) -> &Oid {
        &self.oid
    }

    /// The multigraph of frames (vertex k is the k-th frame by address)
    /// and relations.
    pub fn graph(&self) -> &Graph {
        &self.graph
    }

    /// The relation on each edge of [`graph`](Self::graph), in edge order.
    pub fn edges(&self) -> &[Oid] {
        &self.edges
    }

    /// A basis of H⁰: each a section, one [`Interpretation`] per frame (by
    /// address), consistent across every relation. Empty when the frames
    /// share nothing that survives every route.
    pub fn agreement(&self) -> &[Vec<Interpretation<R>>] {
        &self.agreement
    }

    /// The located loops, with their tree.
    pub fn holonomy(&self) -> &Holonomy<R> {
        &self.holonomy
    }

    /// Curvature on the declared faces that don't close.
    pub fn curvature(&self) -> &[Curvature<R>] {
        &self.curvature
    }

    /// Every signer, of relations and faces, ascending.
    pub fn provenance(&self) -> &[Signer] {
        &self.provenance
    }
}

/// Why frames, relations and faces don't compose: a type error, never a
/// loop.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CompositionError {
    /// One reader named as two frames (two dimensions).
    DuplicateFrame(Oid),
    /// A relation or face names a frame that isn't given.
    UnknownFrame(Oid),
    /// A relation between frames of different dimension: no invertible
    /// transport between them.
    DimensionMismatch {
        /// The relation.
        relation: Oid,
        /// Its source frame's dimension.
        from: usize,
        /// Its target frame's dimension.
        to: usize,
    },
    /// A transport larger than the frames it relates.
    TooLarge {
        /// The relation.
        relation: Oid,
        /// The transport's size.
        size: usize,
        /// The frames' dimension.
        dim: usize,
    },
    /// A face whose sides aren't single relations.
    Face(FieldError),
}

/// Compose frames along their relations, with faces declared: the
/// composite's interpretation. Fails only on a type error.
pub fn compose<R: Scalar>(
    frames: &[Frame],
    relations: &[Relation<R>],
    faces: &[Face],
) -> Result<Composite<R>, CompositionError> {
    // Frames, ascending by address; one reader, one frame.
    let mut by_reader: BTreeMap<&Oid, &Frame> = BTreeMap::new();
    for f in frames {
        if by_reader.insert(f.reader(), f).is_some_and(|g| g != f) {
            return Err(CompositionError::DuplicateFrame(f.reader().clone()));
        }
    }
    let sites: BTreeMap<&Oid, &Frame> = by_reader.values().map(|f| (f.oid(), *f)).collect();
    let vertex: BTreeMap<&Oid, VertexId> = sites
        .keys()
        .enumerate()
        .map(|(i, &oid)| (oid, i as VertexId))
        .collect();
    let order: Vec<&Frame> = sites.values().copied().collect();
    let site = |oid: &Oid| {
        vertex
            .get(oid)
            .map(|&v| (v, order[v as usize].dim()))
            .ok_or_else(|| CompositionError::UnknownFrame(oid.clone()))
    };

    // Relations, one per address, typed.
    let mut unique: BTreeMap<&Oid, &Relation<R>> = BTreeMap::new();
    let mut provenance: BTreeSet<Signer> = BTreeSet::new();
    for r in relations {
        unique.insert(r.oid(), r);
        provenance.insert(r.signer().clone());
    }
    let mut typed = Vec::with_capacity(unique.len());
    for r in unique.values() {
        let ((u, du), (v, dv)) = (site(r.from())?, site(r.to())?);
        if du != dv {
            return Err(CompositionError::DimensionMismatch {
                relation: r.oid().clone(),
                from: du,
                to: dv,
            });
        }
        if r.transport().size() > du {
            return Err(CompositionError::TooLarge {
                relation: r.oid().clone(),
                size: r.transport().size(),
                dim: du,
            });
        }
        typed.push((u, v, du, *r));
    }

    // Faces, one per address.
    let mut reported: BTreeMap<&Oid, &Face> = BTreeMap::new();
    for f in faces {
        reported.insert(f.oid(), f);
        provenance.insert(f.signer().clone());
    }
    let mut corners = Vec::with_capacity(reported.len());
    for f in reported.values() {
        let mut c = [0; 3];
        for (k, oid) in f.corners().iter().enumerate() {
            c[k] = site(oid)?.0;
        }
        corners.push((c, f.oid().clone()));
    }

    // The connection: one edge per relation, numbered as the field numbers
    // parallel edges (by ends, in the order given).
    let mut nth: BTreeMap<(VertexId, VertexId), usize> = BTreeMap::new();
    let mut relation_on: BTreeMap<Edge, Oid> = BTreeMap::new();
    for &(u, v, _, r) in &typed {
        let k = nth.entry((u.min(v), u.max(v))).or_insert(0);
        relation_on.insert(Edge::between(u, v, *k), r.oid().clone());
        *k += 1;
    }
    let field = Field::multigraph_over(
        0..order.len() as VertexId,
        typed
            .iter()
            .map(|&(u, v, _, r)| ((u, v), r.transport().clone())),
    )
    .with_faces(corners.iter().map(|(c, _)| *c))
    .map_err(CompositionError::Face)?;
    let graph = field.graph().clone();
    let edges: Vec<Oid> = graph.edges().map(|e| relation_on[&e].clone()).collect();
    let index: BTreeMap<Edge, usize> = graph.edges().enumerate().map(|(i, e)| (e, i)).collect();
    let chords: BTreeSet<usize> = graph.chords().into_iter().collect();
    let tree = (0..edges.len())
        .filter(|i| !chords.contains(i))
        .map(|i| edges[i].clone())
        .collect();

    let name = |v: &VertexId| order[*v as usize].oid().clone();
    let (loops, curved) = match check_flat(&field) {
        Flatness::Flat(_) => (Vec::new(), Vec::new()),
        Flatness::Curved(c) => (c.loops().to_vec(), c.faces().to_vec()),
    };
    let loops = loops
        .into_iter()
        .map(|l| located(&l, &name, &relation_on, &index, edges.len()))
        .collect();
    let curvature = curved
        .into_iter()
        .map(|l| {
            let around: BTreeSet<VertexId> = l.vertices().iter().copied().collect();
            let face = corners
                .iter()
                .find(|(c, _)| c.iter().copied().collect::<BTreeSet<_>>() == around)
                .map(|(_, oid)| oid.clone())
                .unwrap_or_else(Oid::dark);
            Curvature {
                face,
                frames: l.vertices().iter().map(name).collect(),
                holonomy: l.holonomy().clone(),
            }
        })
        .collect();

    // Agreement: ker of s ↦ (s(to) − T·s(from)) over every relation.
    let offsets: Vec<usize> = order
        .iter()
        .scan(0, |at, f| {
            let here = *at;
            *at += f.dim();
            Some(here)
        })
        .collect();
    let width = order.iter().map(|f| f.dim()).sum();
    let (zero, one) = (R::zero(&()), R::one(&()));
    let mut rows = Vec::new();
    for &(u, v, dim, r) in &typed {
        let t = r.transport().rows(dim);
        for (i, t_row) in t.iter().enumerate() {
            let mut row = vec![zero.clone(); width];
            row[offsets[v as usize] + i] = row[offsets[v as usize] + i].add(&one);
            for (j, t_ij) in t_row.iter().enumerate() {
                let at = offsets[u as usize] + j;
                row[at] = row[at].add(&t_ij.neg());
            }
            rows.push(row);
        }
    }
    let agreement = R::kernel(&rows, width)
        .into_iter()
        .map(|s| {
            order
                .iter()
                .zip(&offsets)
                .map(|(f, &at)| Interpretation {
                    frame: f.oid().clone(),
                    value: s[at..at + f.dim()].to_vec(),
                })
                .collect()
        })
        .collect();

    // The address: the key, signers beside.
    let mut bytes = b"composite\0".to_vec();
    bytes.extend_from_slice(&(order.len() as u64).to_le_bytes());
    for f in &order {
        delimited(&mut bytes, f.oid().as_str().as_bytes());
    }
    bytes.extend_from_slice(&(unique.len() as u64).to_le_bytes());
    for oid in unique.keys() {
        delimited(&mut bytes, oid.as_str().as_bytes());
    }
    bytes.extend_from_slice(&(reported.len() as u64).to_le_bytes());
    for oid in reported.keys() {
        delimited(&mut bytes, oid.as_str().as_bytes());
    }

    Ok(Composite {
        oid: Oid::hash(&Scope::public(), &bytes),
        graph,
        edges,
        agreement,
        holonomy: Holonomy { tree, loops },
        curvature,
        provenance: provenance.into_iter().collect(),
    })
}

/// A field loop, read in frames and relations, with its certificate.
fn located<R: Scalar>(
    l: &Loop<Gl<R>>,
    name: &impl Fn(&VertexId) -> Oid,
    relation_on: &BTreeMap<Edge, Oid>,
    index: &BTreeMap<Edge, usize>,
    edges: usize,
) -> Located<R> {
    let frames: Vec<Oid> = l.vertices().iter().map(name).collect();
    let relations: Vec<Oid> = l.edges().iter().map(|e| relation_on[e].clone()).collect();
    let mut certificate = vec![Z::from(0); edges];
    for (e, from) in l.edges().iter().zip(l.vertices()) {
        let step = if e.ends().0 == *from { 1 } else { -1 };
        let at = index[e];
        certificate[at] = certificate[at].add(&Z::from(step));
    }
    let mut bytes = b"loop\0".to_vec();
    bytes.extend_from_slice(&(relations.len() as u64).to_le_bytes());
    for (f, r) in frames.iter().zip(&relations) {
        delimited(&mut bytes, f.as_str().as_bytes());
        delimited(&mut bytes, r.as_str().as_bytes());
    }
    Located {
        id: LoopId(Oid::hash(&Scope::public(), &bytes)),
        frames,
        relations,
        holonomy: l.holonomy().clone(),
        certificate,
    }
}

// --- The hypothesis ----------------------------------------------------------------

/// A face proposed for a loop: the loop declared closed.
#[derive(Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Polygon {
    frames: Vec<Oid>,
}

impl Polygon {
    /// Around it, first frame repeated last.
    pub fn frames(&self) -> &[Oid] {
        &self.frames
    }
}

/// A new relation the frames on a loop would sign together.
#[derive(Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Bridge {
    frames: BTreeSet<Oid>,
}

impl Bridge {
    /// The frames whose signatures it needs.
    pub fn frames(&self) -> &BTreeSet<Oid> {
        &self.frames
    }
}

/// One way a located loop could close (Mara §4.3, after *Whose Metric?*
/// §3). Proposed, never picked. The set of moves is READING; their effects
/// are OPEN in general. Ordered only so a set has one order everywhere:
/// the order is not a ranking.
#[derive(Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Closure {
    /// Declare the loop closed: a face no one has reported. Narrows both
    /// sides.
    Fill(Polygon),
    /// One frame changes this relation, inside its own reach. Non-local
    /// damage.
    Twist(Oid),
    /// This frame takes over the others' reach.
    Absorb(Oid),
    /// The frames sign one new relation together; each then holds the loop
    /// inside its own frame.
    Share(Bridge),
    /// A systematic bias on this relation, not a curved world: the two
    /// can't be told apart without a second, independent route.
    Bias(Oid),
}

/// What a `Partial` hypothesis leaves open: a located loop, or a declared
/// face that doesn't close.
#[derive(Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Obstruction {
    /// An open loop: it has alternatives.
    Loop(LoopId),
    /// Curvature on this face: an answer, reported, not a hypothesis.
    Face(Oid),
}

/// A hypothesis over a composite: per located loop, the unranked closures
/// that would remove it. Its ground is the composite, by address.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Hypothesis {
    ground: Oid,
    alternatives: BTreeMap<LoopId, BTreeSet<Closure>>,
}

impl Hypothesis {
    /// The composite it is grounded on.
    pub fn ground(&self) -> &Oid {
        &self.ground
    }

    /// The closures per located loop: a set, a menu, never ranked.
    pub fn alternatives(&self) -> &BTreeMap<LoopId, BTreeSet<Closure>> {
        &self.alternatives
    }
}

/// Every closure of one loop: fill it, twist or bias any relation on it,
/// absorb into any frame on it, or share one new relation among them all.
fn closures<R: Scalar>(l: &Located<R>) -> BTreeSet<Closure> {
    let frames: BTreeSet<Oid> = l.frames().iter().cloned().collect();
    let relations: BTreeSet<&Oid> = l.relations().iter().collect();
    let mut menu = BTreeSet::from([
        Closure::Fill(Polygon {
            frames: l.frames().to_vec(),
        }),
        Closure::Share(Bridge {
            frames: frames.clone(),
        }),
    ]);
    menu.extend(relations.iter().map(|&r| Closure::Twist(r.clone())));
    menu.extend(frames.into_iter().map(Closure::Absorb));
    menu.extend(relations.into_iter().map(|r| Closure::Bias(r.clone())));
    menu
}

/// Compose, then propose: `Success` when flat (no alternatives),
/// `Partial` when loops are open or faces curved (the obstructions as its
/// loss), `Failure` only on a type error. The empty input is the unit:
/// `Success`.
pub fn hypothesize<R: Scalar>(
    frames: &[Frame],
    relations: &[Relation<R>],
    faces: &[Face],
) -> Imperfect<Hypothesis, CompositionError, BTreeSet<Obstruction>> {
    let composite = match compose(frames, relations, faces) {
        Ok(c) => c,
        Err(e) => return Imperfect::Failure(e, BTreeSet::new()),
    };
    let alternatives: BTreeMap<LoopId, BTreeSet<Closure>> = composite
        .holonomy
        .loops
        .iter()
        .map(|l| (l.id.clone(), closures(l)))
        .collect();
    let mut open: BTreeSet<Obstruction> = alternatives
        .keys()
        .cloned()
        .map(Obstruction::Loop)
        .collect();
    open.extend(
        composite
            .curvature
            .iter()
            .map(|c| Obstruction::Face(c.face.clone())),
    );
    let hypothesis = Hypothesis {
        ground: composite.oid,
        alternatives,
    };
    if open.is_empty() {
        Imperfect::Success(hypothesis)
    } else {
        Imperfect::Partial(hypothesis, open)
    }
}

fn delimited(bytes: &mut Vec<u8>, field: &[u8]) {
    bytes.extend_from_slice(&(field.len() as u64).to_le_bytes());
    bytes.extend_from_slice(field);
}
