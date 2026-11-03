//! `prismqueer::discoball` — rotate a ball of facets and read the light
//! off it (prism-012, Ask 3).
//!
//! A small instrument over [`Room`]. The names are provisional.
//!
//! - **A facet is a Prism's reading** ([`Facet`]) on one room H over ℤ:
//!   its operator on H, the spots it lit at each step boundary, and its
//!   signed dependency rules. It is built from the Prism itself, and records
//!   whether that Prism carries the [`Stable`] proof ([`Proof`]).
//! - **A rotation is a basis change over the fixed ring** ([`Rotation`]): a
//!   signed permutation, the unitaries of ℤⁿ, acting as a chart shift
//!   A ↦ RARᵀ. It is never a ring change. ℤ → ℤ/n is a quotient and loses
//!   information; a rotation loses none.
//! - **The measurement is a triple** ([`Triple`]) per facet, read in the
//!   ball's current chart:
//!   1. **lums**: tr(P_S) for the lit set S at each step boundary. How much
//!      light: progress and monotonicity, never identity.
//!   2. **schedule-holonomy**: the horizontal twist. The ℤ/2 holonomy of
//!      the facet's signed dependency cycles, by room's exact [`solve`]: an
//!      odd number of `not`s around a loop is a twist, certified by the
//!      loop; an even number is a choice ([`Holonomy`]).
//!   3. **findings**: the vertical twist. A declaration violated in the
//!      run's true set. prism doesn't decide declarations: a caller-supplied
//!      [`Ruler`] does, and prism carries what it finds.
//!
//!   Beside the triple, the spectrum exactly ([`Charpoly`]), and the
//!   invariant factors ([`Invariants`]), because the spectrum alone misses
//!   a shear.
//! - **A dark spot is received darkness** ([`DarkSpot`]): light that never
//!   arrived, from outside (a hole, unreadable input, the budget). It is
//!   never a facet's doing, so it lives in the ball's outer channel
//!   ([`Reading::dark`]) and in no facet's triple.
//!
//! What `discoball` keeps from collapsing early: the triple stays three
//! readings, never a score; a twist carries its certificate; findings stay
//! the ruler's values, never a count.
//!
//! Credits: the disco-ball image is Lala's (Peecabo); the gauge frame is
//! Ricky Jones's.
//!
//! # Rotation is not a ring change
//!
//! A rotation is a map over ℤ, the ring the ball lives in. The control
//! compiles:
//!
//! ```
//! use prismqueer::discoball::{Ball, Rotation};
//! use prismqueer::room::{Map, Modulus, Room, Vertices, Z};
//! let h: Room<Vertices> = Room::over(&(0..2).collect());
//! let two = Modulus::new(2).unwrap();
//! let id: Map<Vertices, Vertices, Z> = Map::identity(&h);
//! let _ = id.reduce(&two);
//! let mut ball = Ball::new(&h);
//! ball.rotate(&Rotation::new(&id).unwrap()).unwrap();
//! ```
//!
//! A map moved to ℤ/2 isn't a rotation of a room over ℤ:
//!
//! ```compile_fail,E0308
//! use prismqueer::discoball::{Ball, Rotation};
//! use prismqueer::room::{Map, Modulus, Room, Vertices, Z};
//! let h: Room<Vertices> = Room::over(&(0..2).collect());
//! let two = Modulus::new(2).unwrap();
//! let id: Map<Vertices, Vertices, Z> = Map::identity(&h);
//! let _ = id.reduce(&two);
//! let mut ball = Ball::new(&h);
//! ball.rotate(&Rotation::new(&id.reduce(&two)).unwrap()).unwrap();
//! ```
//!
//! Nor is any map a rotation until [`Rotation::new`] checked it unitary:
//!
//! ```compile_fail,E0308
//! use prismqueer::discoball::{Ball, Rotation};
//! use prismqueer::room::{Map, Modulus, Room, Vertices, Z};
//! let h: Room<Vertices> = Room::over(&(0..2).collect());
//! let two = Modulus::new(2).unwrap();
//! let id: Map<Vertices, Vertices, Z> = Map::identity(&h);
//! let _ = id.reduce(&two);
//! let mut ball = Ball::new(&h);
//! ball.rotate(&id).unwrap();
//! ```

use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Debug;

use num_traits::ToPrimitive;

use crate::dependency::{DependencyGraph, Sign, Through};
use crate::field::{Edge, Graph};
use crate::poly::Invariants;
use crate::room::{
    solve, Charpoly, Exactness, Map, Modulus, Place, Room, RoomError, Vertices, ZMod, Z,
};
use crate::{Prism, Stable};

/// Why the ball refused.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum BallError {
    /// The map isn't a unitary of the room over ℤ (a signed permutation),
    /// or the images aren't a permutation of the room's spots.
    NotARotation,
    /// A facet's operator, or a rotation, is on another room.
    Room(RoomError),
    /// A spot (lit, dark, or a rule's atom) isn't on the ball.
    Outside {
        /// The spot.
        spot: usize,
        /// The ball's dimension: spots are `0..dim`.
        dim: usize,
    },
    /// A facet claimed the [`Stable`] proof but carries a rule through a
    /// `not`: stable Prisms have none.
    NotInStable {
        /// The rule's premise.
        premise: usize,
        /// Its head.
        head: usize,
    },
}

impl From<RoomError> for BallError {
    fn from(e: RoomError) -> Self {
        BallError::Room(e)
    }
}

fn place(room: &Room<Vertices>) -> Place {
    Place {
        dim: room.dim(),
        complex: room.complex().clone(),
    }
}

// --- Rotation ----------------------------------------------------------------

/// A rotation of the ball: a unitary of H over ℤ, which is a signed
/// permutation. It sends spot `j` to spot `image()[j]`, up to sign, and acts
/// on an operator A as RARᵀ = RAR⁻¹, a similarity. It is a chart shift:
/// it keeps the ring, so it keeps the spectrum, the invariant factors and
/// every trace.
///
/// Only a map over ℤ can be one, by type (see the module docs); only a
/// unitary one, checked by [`Rotation::new`].
#[derive(Clone, Debug, PartialEq)]
pub struct Rotation {
    map: Map<Vertices, Vertices, Z>,
    image: Vec<usize>,
}

impl Rotation {
    /// The rotation `map` is, if it is one: a map from a room to itself
    /// with exactly one nonzero entry, ±1, in each row and column.
    pub fn new(map: &Map<Vertices, Vertices, Z>) -> Result<Self, BallError> {
        if map.from() != map.to() {
            return Err(BallError::NotARotation);
        }
        let n = map.from().dim();
        let (zero, one, minus) = (Z::from(0), Z::from(1), Z::from(-1));
        let mut image = Vec::with_capacity(n);
        let mut hit = vec![false; n];
        for col in 0..n {
            let mut found = None;
            for (row, taken) in hit.iter().enumerate() {
                let x = map.entry(row, col).expect("in the room");
                if *x == zero {
                    continue;
                }
                if found.is_some() || *taken || (*x != one && *x != minus) {
                    return Err(BallError::NotARotation);
                }
                found = Some(row);
            }
            let row = found.ok_or(BallError::NotARotation)?;
            hit[row] = true;
            image.push(row);
        }
        Ok(Rotation {
            map: map.clone(),
            image,
        })
    }

    /// The permutation of `room`'s spots sending `j` to `image[j]`.
    pub fn permutation(room: &Room<Vertices>, image: &[usize]) -> Result<Self, BallError> {
        let n = room.dim();
        let mut rows = vec![vec![Z::from(0); n]; n];
        if image.len() != n {
            return Err(BallError::NotARotation);
        }
        for (j, &i) in image.iter().enumerate() {
            if i >= n {
                return Err(BallError::NotARotation);
            }
            rows[i][j] = Z::from(1);
        }
        Rotation::new(&Map::from_rows(room, room, &rows)?)
    }

    /// The rotation that moves nothing.
    pub fn identity(room: &Room<Vertices>) -> Self {
        Rotation {
            map: Map::identity(room),
            image: (0..room.dim()).collect(),
        }
    }

    /// The rotation as a map over ℤ.
    pub fn map(&self) -> &Map<Vertices, Vertices, Z> {
        &self.map
    }

    /// Where each spot goes: spot `j` to `image()[j]`.
    pub fn image(&self) -> &[usize] {
        &self.image
    }

    /// First this rotation, then `next`. Both must turn one room.
    pub fn then(&self, next: &Rotation) -> Result<Rotation, BallError> {
        Ok(Rotation {
            map: self.map.then(&next.map)?,
            image: self.image.iter().map(|&i| next.image[i]).collect(),
        })
    }

    /// RARᵀ: `a` in the turned chart. R is unitary, so Rᵀ = R⁻¹ and this is
    /// a similarity.
    fn conjugate(&self, a: &Map<Vertices, Vertices, Z>) -> Map<Vertices, Vertices, Z> {
        self.map
            .adjoint()
            .then(a)
            .and_then(|m| m.then(&self.map))
            .expect("one room, checked when the facet joined")
    }
}

// --- Facets ------------------------------------------------------------------

/// Whether a facet's Prism carries the [`Stable`] proof. `Unproved` is the
/// absence of the proof, not a claim of twist.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Proof {
    /// Built from a Prism marked [`Stable`]: no `not`, no declaration.
    Stable,
    /// Built from a Prism without the proof.
    Unproved,
}

/// A Prism's reading on the ball's room H, in its own (home) chart.
///
/// - **operator**: the Prism's action on H, over ℤ;
/// - **light**: the lit spots at each step boundary, in order ([`Facet::step`]);
/// - **rules**: its signed dependency graph, on the spots as atoms.
#[derive(Clone, Debug, PartialEq)]
pub struct Facet {
    name: String,
    proof: Proof,
    operator: Map<Vertices, Vertices, Z>,
    rules: DependencyGraph<usize>,
    light: Vec<BTreeSet<usize>>,
}

impl Facet {
    /// A facet of a stable Prism. Refused if a rule runs through a `not`:
    /// the proof says there is none.
    pub fn stable<P: Stable>(
        _prism: &P,
        name: &str,
        operator: Map<Vertices, Vertices, Z>,
        rules: DependencyGraph<usize>,
    ) -> Result<Self, BallError> {
        if let Some(e) = rules.negative_edges().next() {
            return Err(BallError::NotInStable {
                premise: *e.from(),
                head: *e.to(),
            });
        }
        Ok(Facet::of(name, Proof::Stable, operator, rules))
    }

    /// A facet of a Prism without the [`Stable`] proof.
    pub fn unproved<P: Prism>(
        _prism: &P,
        name: &str,
        operator: Map<Vertices, Vertices, Z>,
        rules: DependencyGraph<usize>,
    ) -> Self {
        Facet::of(name, Proof::Unproved, operator, rules)
    }

    fn of(
        name: &str,
        proof: Proof,
        operator: Map<Vertices, Vertices, Z>,
        rules: DependencyGraph<usize>,
    ) -> Self {
        Facet {
            name: name.to_string(),
            proof,
            operator,
            rules,
            light: Vec::new(),
        }
    }

    /// Record a step boundary: the spots lit there.
    pub fn step(&mut self, lit: impl IntoIterator<Item = usize>) -> &mut Self {
        self.light.push(lit.into_iter().collect());
        self
    }

    /// The facet's name.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Whether its Prism carries the [`Stable`] proof.
    pub fn proof(&self) -> Proof {
        self.proof
    }

    /// Its operator on H, in its own chart.
    pub fn operator(&self) -> &Map<Vertices, Vertices, Z> {
        &self.operator
    }

    /// Its signed dependency rules, in its own chart.
    pub fn rules(&self) -> &DependencyGraph<usize> {
        &self.rules
    }

    /// The lit spots at each step boundary, in its own chart.
    pub fn light(&self) -> &[BTreeSet<usize>] {
        &self.light
    }

    /// The run's true set: the spots lit at the last step boundary, in its
    /// own chart. Empty before the first step.
    pub fn true_set(&self) -> BTreeSet<usize> {
        self.light.last().cloned().unwrap_or_default()
    }

    /// Every spot it names: lit, or an atom of a rule.
    fn spots(&self) -> impl Iterator<Item = usize> + '_ {
        let atoms = self.rules.edges().flat_map(|e| [*e.from(), *e.to()]);
        self.light.iter().flatten().copied().chain(atoms)
    }
}

/// The vertical twist's judge: what a declaration finds violated in a
/// facet's true set. prism never decides declarations, so there is no
/// default ruler and none ships: the caller supplies it, and prism carries
/// its findings.
///
/// The ruler reads the facet in its own chart ([`Facet::true_set`]):
/// findings are about atoms, so turning the ball never moves one.
pub trait Ruler {
    /// What the ruler reports for each violation.
    type Finding: Clone + Debug + PartialEq;
    /// The violations in `facet`'s true set. Empty: no declaration is
    /// violated.
    fn findings(&self, facet: &Facet) -> Vec<Self::Finding>;
}

// --- Darkness ----------------------------------------------------------------

/// Where received darkness came from: the world, never a facet.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Received {
    /// A hole: something not yet known.
    Hole,
    /// Input that couldn't be read.
    Unreadable,
    /// The budget ran out before the light arrived.
    Budget,
}

/// A spot where light never arrived, and why.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DarkSpot {
    spot: usize,
    received: Received,
}

impl DarkSpot {
    /// Darkness received at `spot`.
    pub fn new(spot: usize, received: Received) -> Self {
        DarkSpot { spot, received }
    }

    /// The spot.
    pub fn spot(&self) -> usize {
        self.spot
    }

    /// Where the darkness came from.
    pub fn received(&self) -> Received {
        self.received
    }
}

// --- The reading -------------------------------------------------------------

/// The horizontal twist of one facet: room's exact verdict on its signed
/// dependency cycles over ℤ/2, with the graph and cochain it was read on,
/// so every certificate can be checked.
///
/// Only the rules on a directed cycle are read (an edge whose head reaches
/// its premise): a stratified `not` closes no loop and never twists, even
/// when its undirected shadow has an odd cycle. In a strongly connected
/// digraph the directed cycles span the cycle space, so on those edges the
/// ℤ/2 verdict is exactly "some dependency loop has an odd number of
/// `not`s".
#[derive(Clone, Debug, PartialEq)]
pub struct Holonomy {
    graph: Graph,
    cochain: Vec<ZMod>,
    verdict: Exactness<ZMod>,
}

impl Holonomy {
    /// The cyclic rules as an undirected multigraph, in the current chart.
    pub fn graph(&self) -> &Graph {
        &self.graph
    }

    /// The signs as a ℤ/2 cochain on [`Holonomy::graph`]'s edges, in its
    /// edge order: a `not` is 1, a plain dependency 0.
    pub fn cochain(&self) -> &[ZMod] {
        &self.cochain
    }

    /// [`solve`]'s verdict: `Exact`, a consistent choice (the potential);
    /// `Genuine`, a twist, with the loops that certify it.
    pub fn verdict(&self) -> &Exactness<ZMod> {
        &self.verdict
    }

    /// Whether some loop twists.
    pub fn twisted(&self) -> bool {
        matches!(self.verdict, Exactness::Genuine(_))
    }

    fn of(rules: &DependencyGraph<usize>, image: &[usize]) -> Self {
        let two = Modulus::new(2).expect("2 ≥ 2");
        let cyclic: Vec<(u64, u64, Sign)> = rules
            .edges()
            .filter(|e| {
                rules
                    .reach(&BTreeSet::from([*e.to()]), Through::Any)
                    .contains(e.from())
            })
            .map(|e| (image[*e.from()] as u64, image[*e.to()] as u64, e.sign()))
            .collect();
        let mut nth: BTreeMap<(u64, u64), usize> = BTreeMap::new();
        let mut sign = BTreeMap::new();
        for &(a, b, s) in &cyclic {
            let k = nth.entry((a.min(b), a.max(b))).or_insert(0);
            sign.insert(Edge::between(a, b, *k), s);
            *k += 1;
        }
        let graph = Graph::new([], cyclic.iter().map(|&(a, b, _)| (a, b)));
        let cochain: Vec<ZMod> = graph
            .edges()
            .map(|e| match sign[&e] {
                Sign::Positive => ZMod::new(0, &two),
                Sign::Negative => ZMod::new(1, &two),
            })
            .collect();
        let verdict = solve(&two, &graph, &cochain).expect("one value per edge, in ℤ/2");
        Holonomy {
            graph,
            cochain,
            verdict,
        }
    }
}

/// The measurement triple of one facet.
#[derive(Clone, Debug, PartialEq)]
pub struct Triple<F> {
    lums: Vec<u64>,
    holonomy: Holonomy,
    findings: Vec<F>,
}

impl<F> Triple<F> {
    /// Lums now: at the last step boundary (0 before the first).
    pub fn lums(&self) -> u64 {
        self.lums.last().copied().unwrap_or(0)
    }

    /// Lums at every step boundary, tr(P_S) in the current chart.
    pub fn lums_trace(&self) -> &[u64] {
        &self.lums
    }

    /// The first step at which lums fell, if any. A stable Prism never
    /// shows one; a drop proves the facet darkened itself, but not every
    /// darkening shows one.
    pub fn first_drop(&self) -> Option<usize> {
        self.lums
            .windows(2)
            .position(|w| w[1] < w[0])
            .map(|i| i + 1)
    }

    /// The schedule-holonomy: the horizontal twist and its certificate.
    pub fn holonomy(&self) -> &Holonomy {
        &self.holonomy
    }

    /// Whether the light arrives twisted horizontally: an odd `not` loop.
    pub fn horizontal(&self) -> bool {
        self.holonomy.twisted()
    }

    /// The ruler's findings, as it gave them.
    pub fn findings(&self) -> &[F] {
        &self.findings
    }

    /// Whether the light arrives twisted vertically: a declaration violated.
    pub fn vertical(&self) -> bool {
        !self.findings.is_empty()
    }
}

/// How a facet's own light exits (prism-012's corrected table). Received
/// darkness (exit 2, unknown) is never a facet's: it is the ball's outer
/// channel, [`Reading::dark`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Exit {
    /// 0: the light arrives, untwisted.
    Clear,
    /// 1: the light arrives twisted horizontally, an odd `not` loop (an
    /// `undefined` atom): the ternary's `Partial`.
    Uncertain,
    /// 255: the light arrives twisted vertically, a declaration violated.
    /// A certain paradox comes first, so it outranks a horizontal twist.
    Paradox,
}

impl Exit {
    /// The exit code.
    pub fn code(&self) -> u8 {
        match self {
            Exit::Clear => 0,
            Exit::Uncertain => 1,
            Exit::Paradox => 255,
        }
    }
}

/// One facet, read in the ball's current chart.
#[derive(Clone, Debug, PartialEq)]
pub struct FacetReport<F> {
    name: String,
    proof: Proof,
    spots: BTreeSet<usize>,
    triple: Triple<F>,
    charpoly: Charpoly,
    invariants: Invariants,
}

impl<F> FacetReport<F> {
    /// The facet's name.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Whether its Prism carries the [`Stable`] proof.
    pub fn proof(&self) -> Proof {
        self.proof
    }

    /// Where its true set is now: the lit spots, turned with the ball.
    pub fn spots(&self) -> &BTreeSet<usize> {
        &self.spots
    }

    /// The measurement triple.
    pub fn triple(&self) -> &Triple<F> {
        &self.triple
    }

    /// The spectrum, exactly: det(xI − A) of its operator.
    pub fn charpoly(&self) -> &Charpoly {
        &self.charpoly
    }

    /// Its operator's invariant factors: the complete similarity invariant,
    /// which tells a shear from the identity where the charpoly can't.
    pub fn invariants(&self) -> &Invariants {
        &self.invariants
    }

    /// How its light exits: a vertical twist first, then a horizontal one.
    pub fn exit(&self) -> Exit {
        if self.triple.vertical() {
            Exit::Paradox
        } else if self.triple.horizontal() {
            Exit::Uncertain
        } else {
            Exit::Clear
        }
    }
}

/// What the ball shows now: every facet's report, and the darkness it
/// received, apart.
#[derive(Clone, Debug, PartialEq)]
pub struct Reading<F> {
    facets: Vec<FacetReport<F>>,
    dark: Vec<DarkSpot>,
}

impl<F> Reading<F> {
    /// The facets' reports, in the order they joined.
    pub fn facets(&self) -> &[FacetReport<F>] {
        &self.facets
    }

    /// The outer channel: received darkness, in the current chart, in the
    /// order it arrived.
    pub fn dark(&self) -> &[DarkSpot] {
        &self.dark
    }

    /// Total lums now: the facets' lums summed.
    pub fn lums(&self) -> u64 {
        self.facets.iter().map(|f| f.triple.lums()).sum()
    }
}

// --- The ball ----------------------------------------------------------------

/// The disco ball: facets on one room H over ℤ, an orientation, and the
/// darkness it received.
#[derive(Clone, Debug, PartialEq)]
pub struct Ball {
    room: Room<Vertices>,
    facets: Vec<Facet>,
    /// In the home chart.
    dark: Vec<DarkSpot>,
    orientation: Rotation,
}

impl Ball {
    /// An empty ball on `room`, unturned.
    pub fn new(room: &Room<Vertices>) -> Self {
        Ball {
            room: room.clone(),
            facets: Vec::new(),
            dark: Vec::new(),
            orientation: Rotation::identity(room),
        }
    }

    /// Add a facet, given in the home chart. Returns its index. Refused if
    /// its operator is on another room, or it names a spot off the ball.
    pub fn facet(&mut self, facet: Facet) -> Result<usize, BallError> {
        for side in [facet.operator.from(), facet.operator.to()] {
            if side != &self.room {
                return Err(BallError::Room(RoomError::Mismatch {
                    expected: place(&self.room),
                    found: place(side),
                }));
            }
        }
        self.on_ball(facet.spots())?;
        self.facets.push(facet);
        Ok(self.facets.len() - 1)
    }

    /// Receive darkness at `spot`, in the current chart.
    pub fn receive(&mut self, spot: usize, received: Received) -> Result<(), BallError> {
        self.on_ball([spot])?;
        let home = self
            .orientation
            .image
            .iter()
            .position(|&i| i == spot)
            .expect("a permutation reaches every spot");
        self.dark.push(DarkSpot::new(home, received));
        Ok(())
    }

    /// Turn the ball by `rotation`, which must be of the ball's room.
    pub fn rotate(&mut self, rotation: &Rotation) -> Result<(), BallError> {
        self.orientation = self.orientation.then(rotation)?;
        Ok(())
    }

    /// The turn so far, from the home chart.
    pub fn orientation(&self) -> &Rotation {
        &self.orientation
    }

    /// Read every facet in the current chart, with `ruler` judging the
    /// declarations, and the darkness apart.
    ///
    /// Exact throughout, over ℤ: each step's lums is the trace of the lit
    /// projection turned (RP_SRᵀ), each operator is turned the same way,
    /// then its charpoly (Berkowitz) and invariant factors (Smith over
    /// ℚ[x]) are read. Dense: about n³ per step and per facet, plus the
    /// Smith form.
    pub fn measure<R: Ruler>(&self, ruler: &R) -> Reading<R::Finding> {
        let facets = self.facets.iter().map(|f| self.report(f, ruler)).collect();
        let dark = self
            .dark
            .iter()
            .map(|d| DarkSpot::new(self.orientation.image[d.spot], d.received))
            .collect();
        Reading { facets, dark }
    }

    fn report<R: Ruler>(&self, facet: &Facet, ruler: &R) -> FacetReport<R::Finding> {
        let turned: Vec<Map<Vertices, Vertices, Z>> = facet
            .light
            .iter()
            .map(|lit| self.orientation.conjugate(&self.projection(lit)))
            .collect();
        let diagonal = |p: &Map<Vertices, Vertices, Z>| -> Vec<usize> {
            (0..self.room.dim())
                .filter(|&i| p.entry(i, i) == Some(&Z::from(1)))
                .collect()
        };
        let lums = turned
            .iter()
            .map(|p| {
                (0..self.room.dim())
                    .map(|i| p.entry(i, i).expect("square").value())
                    .sum::<num_bigint::BigInt>()
                    .to_u64()
                    .expect("a trace of a projection counts spots")
            })
            .collect();
        let spots = turned.last().map(diagonal).unwrap_or_default();
        let operator = self.orientation.conjugate(&facet.operator);
        FacetReport {
            name: facet.name.clone(),
            proof: facet.proof,
            spots: spots.into_iter().collect(),
            triple: Triple {
                lums,
                holonomy: Holonomy::of(&facet.rules, &self.orientation.image),
                findings: ruler.findings(facet),
            },
            charpoly: operator.charpoly(),
            invariants: operator.invariant_factors(),
        }
    }

    /// P_S: the projection onto the lit spots, in the home chart.
    fn projection(&self, lit: &BTreeSet<usize>) -> Map<Vertices, Vertices, Z> {
        let n = self.room.dim();
        let rows: Vec<Vec<Z>> = (0..n)
            .map(|i| {
                (0..n)
                    .map(|j| Z::from(i64::from(i == j && lit.contains(&i))))
                    .collect()
            })
            .collect();
        Map::from_rows(&self.room, &self.room, &rows).expect("n rows of n")
    }

    fn on_ball(&self, spots: impl IntoIterator<Item = usize>) -> Result<(), BallError> {
        let dim = self.room.dim();
        match spots.into_iter().find(|&s| s >= dim) {
            Some(spot) => Err(BallError::Outside { spot, dim }),
            None => Ok(()),
        }
    }
}
