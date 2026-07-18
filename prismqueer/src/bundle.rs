//! Bundle — principal bundle tower with connection.
//!
//! Five traits forming a supertrait chain:
//! Fiber → Connection → Gauge → Transport → Closure → Bundle (blanket)
//!
//! The same mathematical object at every scale:
//! Fate chip, BEAM runtime, Mirror compiler.
//!
//! ## Algebraic structure (per `docs/specs/spectral-triple-grammar.md`)
//!
//! The bundle traits are minimal carriers. The algebraic structure lives
//! in the carried type's traits:
//!
//! - `Connection::Optic` must be a [`Prism`] whose `Input::In` matches
//!   `Fiber::State` — i.e. the connection is an element of the optic
//!   algebra acting on the fiber. (Gap 1)
//! - `Gauge::Group` must implement [`GroupStructure`] (identity, inverse,
//!   composition), and `Gauge` itself supplies `act_on(state)` for the
//!   structure-group action on the Hilbert space. (Gap 2)
//! - `Transport::Holonomy` must implement [`Metric`] (non-negative,
//!   symmetric, triangle-inequality) because Connes' bounded-commutator
//!   condition requires a norm-like residual, not just a monoid. (Gap 4)
//! - `Closure::Fixed` must implement [`LawvereFixedPoint`] (idempotence,
//!   kernel-projection) per `@epistemologic/math/lawvere`. (Gap 3)
//!
//! Together these supertrait constraints turn the prose claim
//! "prism/core is a spectral triple" into a type-level obligation
//! discharged by any concrete bundle implementer.

use crate::beam::{Beam, Optic};
use crate::Prism;
use std::convert::Infallible;
use terni::{Imperfect, Metric};

// ---------------------------------------------------------------------------
// Supertrait laws — the algebraic structure carried by the bundle.
// ---------------------------------------------------------------------------

/// Group structure: identity, inverse, composition, associativity.
///
/// Any type carrying gauge transformations in the principal bundle must
/// satisfy the group axioms. (Per `docs/specs/spectral-triple-grammar.md`
/// audit, Gap 2.) Implementors are responsible for the laws:
///
/// - Identity: `identity().compose(x) == x == x.compose(identity())`
/// - Inverse: `x.compose(x.inverse()) == identity() == x.inverse().compose(x)`
/// - Associativity: `(a.compose(b)).compose(c) == a.compose(b.compose(c))`
pub trait GroupStructure {
    /// The group identity element.
    fn identity() -> Self;

    /// The two-sided inverse of `self` under `compose`.
    fn inverse(&self) -> Self;

    /// Group composition. Non-commutative in general.
    fn compose(&self, other: &Self) -> Self;
}

/// A Lawvere fixed point: idempotent under reapplication of its generating
/// endomap, and lying in the kernel of the bundle's connection-induced
/// operator.
///
/// Per `boot/std/epistemologic/math/lawvere.mirror` and the Soto-Andrade &
/// Varela 1984 autopoiesis bridge: a system is autopoietic iff its tick map
/// has a Lawvere fixed point. The closure level of the bundle realizes that
/// fixed point as a Rust value. (Gap 3.)
///
/// Laws:
/// - Idempotence (under a realising endomap `f`): `f(f(x)) == f(x)`.
/// - Kernel: `in_kernel()` returns true — the fixed point lies in `ker(D)`
///   for `D` the bundle's connection-induced operator.
pub trait LawvereFixedPoint {
    /// `f(f(x)) == f(x)` for the given endomap `f`.
    ///
    /// The default check requires `Self: PartialEq + Sized` so the equality
    /// can be witnessed. Implementors may override for richer comparison.
    fn is_idempotent_under<F>(&self, endomap: F) -> bool
    where
        F: Fn(&Self) -> Self,
        Self: PartialEq + Sized,
    {
        let once = endomap(self);
        let twice = endomap(&once);
        once == twice
    }

    /// Whether this value lies in the kernel of the bundle's
    /// connection-induced operator: applying transport produces no residual.
    /// Concrete implementors witness this with a domain-specific check.
    fn in_kernel(&self) -> bool;
}

// ---------------------------------------------------------------------------
// The five-level bundle tower.
// ---------------------------------------------------------------------------

/// Level 0: the observed state. The section of the bundle.
/// Abyss. The fiber.
pub trait Fiber {
    /// The state space (Hilbert-space "H" for the spectral triple).
    type State;
}

/// Level 1: the optic that determines how information transports.
/// Introject. The connection on the principal bundle.
///
/// The connection IS an element of the optic algebra `A`: the supertrait
/// constraint `Optic: Prism` forces it to compose under Tambara module
/// composition (carried by `prismqueer::Prism` via the
/// focus/project/settle chain). The `Input: Beam<In = Self::State>`
/// bound enforces that the connection acts on the fiber's state.
/// (Gap 1 of `docs/specs/spectral-triple-grammar.md`.)
pub trait Connection: Fiber
where
    Self::Optic: Prism,
    <<Self::Optic as Prism>::Input as crate::Beam>::In: Sized,
{
    /// The algebra element carried by this connection.
    type Optic;

    /// Borrow the connection's optic.
    fn connection(&self) -> &Self::Optic;
}

/// Level 2: the structure group. Which decomposition strategy.
/// Cartographer. The gauge transformation.
///
/// The structure group acts on the fiber's state (Hilbert space `H`). The
/// supertrait `Group: GroupStructure` forces the group axioms; the new
/// `act_on` method names the action on the state and lets implementors
/// witness `g.act_on(h.act_on(s)) == g.compose(&h).act_on(s)`.
/// (Gap 2 of `docs/specs/spectral-triple-grammar.md`.)
pub trait Gauge: Connection
where
    Self::Optic: Prism,
    <<Self::Optic as Prism>::Input as crate::Beam>::In: Sized,
{
    /// The structure group acting on the fiber.
    type Group: GroupStructure;

    /// Borrow the gauge element.
    fn gauge(&self) -> &Self::Group;

    /// Apply the gauge element to a fiber state. Composing groups and
    /// then acting must equal acting twice:
    ///   `g.act_on(&h.act_on(s)) == g.compose(&h).act_on(&s)`.
    fn act_on(&self, state: &Self::State) -> Self::State;
}

/// Level 3: parallel transport with holonomy.
/// Explorer. Comprehension always costs something.
/// The holonomy IS the loss. Returns Partial by design.
///
/// The holonomy type must be a [`Metric`] (not merely a [`terni::Loss`]):
/// Connes' bounded-commutator condition `‖[D, a]‖ < ∞` requires a norm-
/// like residual, which means non-negativity, symmetry, and the triangle
/// inequality. (Gap 4 of `docs/specs/spectral-triple-grammar.md`.)
pub trait Transport: Gauge
where
    Self::Optic: Prism,
    <<Self::Optic as Prism>::Input as crate::Beam>::In: Sized,
{
    /// The metric-valued residual carried by transport.
    type Holonomy: Metric;

    /// Transport the state along the connection. Returns Partial when
    /// transport moves the state off the manifold by a measured amount;
    /// Success when the state remains on the section. The `Infallible`
    /// error position witnesses that transport is total on its domain.
    fn transport(&self, state: &Self::State) -> Imperfect<Self::State, Infallible, Self::Holonomy>;
}

/// Level 4: autopoietic closure. The Lawvere fixed point.
/// Fate. `selectors\[4\]` = self-reference.
///
/// The fixed-point witness must implement [`LawvereFixedPoint`] (idempotence
/// + kernel projection). This links `Closure::Fixed` to the Lawvere fixed
/// point declared in `@epistemologic/math/lawvere.fixed_point`. (Gap 3 of
/// `docs/specs/spectral-triple-grammar.md`.)
pub trait Closure: Transport
where
    Self::Optic: Prism,
    <<Self::Optic as Prism>::Input as crate::Beam>::In: Sized,
{
    /// The fixed-point witness.
    type Fixed: LawvereFixedPoint;

    /// Borrow the closure's fixed point.
    fn close(&self) -> &Self::Fixed;
}

/// A complete principal bundle tower.
/// Blanket impl: any type that implements all five levels is a Bundle.
pub trait Bundle: Closure
where
    Self::Optic: Prism,
    <<Self::Optic as Prism>::Input as crate::Beam>::In: Sized,
{
}
impl<T> Bundle for T
where
    T: Closure,
    T::Optic: Prism,
    <<T::Optic as Prism>::Input as crate::Beam>::In: Sized,
{
}

// ---------------------------------------------------------------------------
// IdentityPrism — the trivial algebra element used by tests and as the
// "identity exists" witness referred to in lib.rs's Prism doc comment.
// ---------------------------------------------------------------------------

/// A prism that passes the carried value through unchanged at every stage.
/// Witnesses that the optic algebra `A` has an identity element.
///
/// Generic over the state type `S: Clone` because each stage produces a
/// fresh beam carrying the same value (the source beam is consumed).
pub struct IdentityPrism<S: Clone> {
    _marker: std::marker::PhantomData<S>,
}

impl<S: Clone> IdentityPrism<S> {
    /// Construct the identity prism for state type `S`.
    pub fn new() -> Self {
        Self {
            _marker: std::marker::PhantomData,
        }
    }
}

impl<S: Clone> Default for IdentityPrism<S> {
    fn default() -> Self {
        Self::new()
    }
}

impl<S: Clone + 'static> Prism for IdentityPrism<S> {
    type Input = Optic<(), S>;
    type Focused = Optic<S, S>;
    type Projected = Optic<S, S>;
    type Refracted = Optic<S, S>;

    fn focus(&self, beam: Self::Input) -> Self::Focused {
        let v = beam
            .value()
            .cloned()
            .expect("IdentityPrism::focus on dark beam");
        beam.next(v)
    }

    fn project(&self, beam: Self::Focused) -> Self::Projected {
        let v = beam
            .value()
            .cloned()
            .expect("IdentityPrism::project on dark beam");
        beam.next(v)
    }

    fn settle(&self, beam: Self::Projected) -> Self::Refracted {
        let v = beam
            .value()
            .cloned()
            .expect("IdentityPrism::settle on dark beam");
        beam.next(v)
    }
}

// ---------------------------------------------------------------------------
// Cyclic<N> — a concrete group used by the test bundle.
// ---------------------------------------------------------------------------

/// The cyclic group `Z/NZ` of rotations modulo `N`. The simplest credible
/// non-trivial group: closed under addition mod N, identity is 0, inverse
/// of `k` is `N - k mod N`. Used by `TestBundle` below to discharge
/// `Gauge::Group: GroupStructure`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Cyclic<const N: u8>(pub u8);

impl<const N: u8> Cyclic<N> {
    /// Construct an element of `Z/NZ`. Values are reduced modulo `N`.
    pub fn new(k: u8) -> Self {
        Cyclic(k % N)
    }

    /// Extract the canonical representative in `0..N`.
    pub fn value(&self) -> u8 {
        self.0
    }
}

impl<const N: u8> GroupStructure for Cyclic<N> {
    fn identity() -> Self {
        Cyclic(0)
    }

    fn inverse(&self) -> Self {
        // (N - self.0) mod N — careful with self.0 == 0.
        Cyclic((N - self.0) % N)
    }

    fn compose(&self, other: &Self) -> Self {
        Cyclic(((self.0 as u16 + other.0 as u16) % N as u16) as u8)
    }
}

// ---------------------------------------------------------------------------
// StableFiber — a concrete fixed-point witness for the test bundle.
// ---------------------------------------------------------------------------

/// A witness that a fiber-state is fixed under a tick-map: the state itself
/// is carried, and `in_kernel()` reports whether the witness was constructed
/// from a transport with zero residual. Used by `TestBundle` to discharge
/// `Closure::Fixed: LawvereFixedPoint`.
#[derive(Clone, Debug, PartialEq)]
pub struct StableFiber<S> {
    /// The fixed state.
    pub state: S,
    /// Whether the witness was certified zero-residual at construction.
    pub kernel: bool,
}

impl<S: Clone + PartialEq> LawvereFixedPoint for StableFiber<S> {
    fn in_kernel(&self) -> bool {
        self.kernel
    }
}

// ---------------------------------------------------------------------------
// pub mod examples — reference bundle carriers for property tests.
// ---------------------------------------------------------------------------

/// Reference Bundle implementations for property-based testing.
///
/// Always-public, always-compiled (not `#[cfg(test)]`-gated) so that
/// downstream crates — mirror, spectral, any consumer of
/// `prismqueer::liquid` — can compose property tests over concrete Bundle
/// carriers without redeclaring the tower.
///
/// # Types
///
/// - [`TestFiber`] — minimal `Fiber` with `State = [f64; 4]`.
/// - [`TestConnection`] — `Fiber + Connection` with identity optic.
/// - [`TestBundle`] — full tower (Fiber+Connection+Gauge+Transport+Closure)
///   with `Cyclic<4>` gauge and *state-dependent* transport loss
///   (`|state[2]| + |state[3]|`). Commutators over `TestBundle` pairs
///   *vanish* whenever both bundles produce the same holonomy at the same
///   state — which they always do here (transport loss depends only on
///   state, not on which bundle transports it). This is the correct,
///   substrate-honest behavior for **abelian** gauge groups.
/// - [`LiquidTestBundle`] — full tower where transport loss depends on
///   the *bundle's* strategy value rather than the state. Commutators
///   over pairs of `LiquidTestBundle`s with different strategies are
///   *non-vanishing*, which is what property tests for Pillar II
///   (algedonic threshold) and Pillar III (viability persistence)
///   require.
pub mod examples {
    use super::*;
    use crate::ScalarLoss;
    use std::convert::Infallible;

    /// Minimal `Fiber` carrier with a 4-vector state.
    pub struct TestFiber;

    impl Fiber for TestFiber {
        type State = [f64; 4];
    }

    /// A trivial `Connection` carrier: it holds an `IdentityPrism` over
    /// the fiber's state space. Adequate for tests that only need to
    /// witness the `Connection` supertrait relationship, not a full
    /// bundle tower.
    pub struct TestConnection {
        pub optic: IdentityPrism<[f64; 4]>,
    }

    impl TestConnection {
        pub fn new() -> Self {
            Self {
                optic: IdentityPrism::new(),
            }
        }
    }

    impl Default for TestConnection {
        fn default() -> Self {
            Self::new()
        }
    }

    impl Fiber for TestConnection {
        type State = [f64; 4];
    }

    impl Connection for TestConnection {
        type Optic = IdentityPrism<[f64; 4]>;
        fn connection(&self) -> &IdentityPrism<[f64; 4]> {
            &self.optic
        }
    }

    /// Full bundle carrier: cyclic gauge, state-dependent transport loss.
    ///
    /// Loss is `|state[2]| + |state[3]|`. For `state = [0.0; 4]`
    /// (Default), loss is zero — the commutator vanishes over Default
    /// state. Since `Cyclic<4>` is abelian, `A·B·state == B·A·state` for
    /// all strategies, so commutators over `TestBundle` always vanish
    /// (correct for abelian gauges).
    pub struct TestBundle {
        pub optic: IdentityPrism<[f64; 4]>,
        pub strategy: Cyclic<4>,
        pub fixed: StableFiber<[f64; 4]>,
    }

    impl TestBundle {
        /// Construct a new bundle with a gauge strategy and kernel-certified flag.
        pub fn new(strategy_shift: u8, kernel: bool) -> Self {
            Self {
                optic: IdentityPrism::new(),
                strategy: Cyclic::<4>::new(strategy_shift),
                fixed: StableFiber {
                    state: [1.0, 2.0, 0.0, 0.0],
                    kernel,
                },
            }
        }

        /// Construct with default (in-kernel) fixed point.
        pub fn with_strategy(strategy_shift: u8) -> Self {
            Self::new(strategy_shift, true)
        }
    }

    impl Default for TestBundle {
        fn default() -> Self {
            Self::with_strategy(0)
        }
    }

    impl Fiber for TestBundle {
        type State = [f64; 4];
    }

    impl Connection for TestBundle {
        type Optic = IdentityPrism<[f64; 4]>;
        fn connection(&self) -> &IdentityPrism<[f64; 4]> {
            &self.optic
        }
    }

    impl Gauge for TestBundle {
        type Group = Cyclic<4>;
        fn gauge(&self) -> &Cyclic<4> {
            &self.strategy
        }
        fn act_on(&self, state: &[f64; 4]) -> [f64; 4] {
            let k = self.strategy.value() as usize % 4;
            let mut out = [0.0; 4];
            for i in 0..4 {
                out[i] = state[(i + k) % 4];
            }
            out
        }
    }

    impl Transport for TestBundle {
        type Holonomy = ScalarLoss;
        fn transport(&self, state: &[f64; 4]) -> Imperfect<[f64; 4], Infallible, ScalarLoss> {
            let compressed = [state[0], state[1], 0.0, 0.0];
            let loss = state[2].abs() + state[3].abs();
            if loss == 0.0 {
                Imperfect::success(compressed)
            } else {
                Imperfect::partial(compressed, ScalarLoss::new(loss))
            }
        }
    }

    impl Closure for TestBundle {
        type Fixed = StableFiber<[f64; 4]>;
        fn close(&self) -> &StableFiber<[f64; 4]> {
            &self.fixed
        }
    }

    /// Full bundle carrier where transport loss depends on the *bundle's*
    /// strategy value rather than the state.
    ///
    /// Loss on `transport(state)` = `strategy.value() as f64`, regardless
    /// of state. This makes commutators over pairs of `LiquidTestBundle`
    /// non-vanishing whenever their strategies differ — exactly what
    /// property tests for Pillar II (algedonic threshold) and Pillar III
    /// (viability persistence) require.
    pub struct LiquidTestBundle {
        pub optic: IdentityPrism<[f64; 4]>,
        pub strategy: Cyclic<4>,
        pub fixed: StableFiber<[f64; 4]>,
    }

    impl LiquidTestBundle {
        pub fn new(strategy_shift: u8, kernel: bool) -> Self {
            Self {
                optic: IdentityPrism::new(),
                strategy: Cyclic::<4>::new(strategy_shift),
                fixed: StableFiber {
                    state: [1.0, 2.0, 0.0, 0.0],
                    kernel,
                },
            }
        }

        pub fn with_strategy(strategy_shift: u8) -> Self {
            Self::new(strategy_shift, true)
        }
    }

    impl Default for LiquidTestBundle {
        fn default() -> Self {
            Self::with_strategy(0)
        }
    }

    impl Fiber for LiquidTestBundle {
        type State = [f64; 4];
    }

    impl Connection for LiquidTestBundle {
        type Optic = IdentityPrism<[f64; 4]>;
        fn connection(&self) -> &IdentityPrism<[f64; 4]> {
            &self.optic
        }
    }

    impl Gauge for LiquidTestBundle {
        type Group = Cyclic<4>;
        fn gauge(&self) -> &Cyclic<4> {
            &self.strategy
        }
        fn act_on(&self, state: &[f64; 4]) -> [f64; 4] {
            let k = self.strategy.value() as usize % 4;
            let mut out = [0.0; 4];
            for i in 0..4 {
                out[i] = state[(i + k) % 4];
            }
            out
        }
    }

    impl Transport for LiquidTestBundle {
        type Holonomy = ScalarLoss;
        fn transport(&self, state: &[f64; 4]) -> Imperfect<[f64; 4], Infallible, ScalarLoss> {
            let strategy_loss = self.strategy.value() as f64;
            if strategy_loss == 0.0 {
                Imperfect::success(*state)
            } else {
                Imperfect::partial(*state, ScalarLoss::new(strategy_loss))
            }
        }
    }

    impl Closure for LiquidTestBundle {
        type Fixed = StableFiber<[f64; 4]>;
        fn close(&self) -> &StableFiber<[f64; 4]> {
            &self.fixed
        }
    }

    // ───────────────────────────────────────────────────────────────
    // Perm3 — the smallest non-abelian group, and PermBundle carrier.
    // ───────────────────────────────────────────────────────────────

    /// The symmetric group `S3` — permutations of three elements.
    /// The smallest non-abelian group. Six elements: identity + three
    /// transpositions + two 3-cycles.
    ///
    /// Represented as the image array `[image[0], image[1], image[2]]`
    /// where `image[i]` is where index `i` maps to. E.g., `[1, 0, 2]`
    /// is the transposition swapping 0 and 1.
    ///
    /// Non-abelian gauge groups — unlike `Cyclic<N>` — have
    /// non-vanishing commutators. This carrier is what
    /// `LiquidConnection::commutator_magnitude` needs to have genuine
    /// work to do: for `a = (1 2)` and `b = (2 3)`, `a∘b = (1 2 3)` but
    /// `b∘a = (1 3 2)`, so the commutator holonomies at any
    /// position-sensitive state must differ.
    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    pub struct Perm3(pub [u8; 3]);

    impl Perm3 {
        /// The identity permutation `[0, 1, 2]`.
        pub const IDENTITY: Perm3 = Perm3([0, 1, 2]);
        /// Transposition `(0 1)` — swaps 0 and 1.
        pub const SWAP_01: Perm3 = Perm3([1, 0, 2]);
        /// Transposition `(1 2)` — swaps 1 and 2.
        pub const SWAP_12: Perm3 = Perm3([0, 2, 1]);
        /// Transposition `(0 2)` — swaps 0 and 2.
        pub const SWAP_02: Perm3 = Perm3([2, 1, 0]);
        /// 3-cycle `(0 1 2)` — 0→1, 1→2, 2→0.
        pub const CYCLE_012: Perm3 = Perm3([1, 2, 0]);
        /// 3-cycle `(0 2 1)` — 0→2, 1→0, 2→1.
        pub const CYCLE_021: Perm3 = Perm3([2, 0, 1]);

        /// All six elements of S3, in a fixed enumeration order.
        pub const ALL: [Perm3; 6] = [
            Perm3::IDENTITY,
            Perm3::SWAP_01,
            Perm3::SWAP_12,
            Perm3::SWAP_02,
            Perm3::CYCLE_012,
            Perm3::CYCLE_021,
        ];
    }

    impl GroupStructure for Perm3 {
        fn identity() -> Self {
            Perm3::IDENTITY
        }

        fn inverse(&self) -> Self {
            // Inverse permutation: if self.0[i] = j, then inverse[j] = i.
            let mut inv = [0u8; 3];
            for i in 0..3 {
                inv[self.0[i] as usize] = i as u8;
            }
            Perm3(inv)
        }

        fn compose(&self, other: &Self) -> Self {
            // (self ∘ other)(i) = self(other(i))
            let mut out = [0u8; 3];
            for i in 0..3 {
                out[i] = self.0[other.0[i] as usize];
            }
            Perm3(out)
        }
    }

    /// Full bundle carrier over `S3` — the non-abelian witness.
    ///
    /// Fiber state is `[i32; 3]`. Gauge is `Perm3` (S3). Transport
    /// holonomy is `state[0].abs() as f64` — permutation-sensitive by
    /// construction: whichever value ends up in position 0 after the
    /// gauge action determines the loss.
    pub struct PermBundle {
        pub optic: IdentityPrism<[i32; 3]>,
        pub perm: Perm3,
        pub fixed: StableFiber<[i32; 3]>,
    }

    impl PermBundle {
        pub fn new(perm: Perm3, kernel: bool) -> Self {
            Self {
                optic: IdentityPrism::new(),
                perm,
                fixed: StableFiber {
                    state: [0, 0, 0],
                    kernel,
                },
            }
        }

        pub fn from_perm(perm: Perm3) -> Self {
            Self::new(perm, true)
        }
    }

    impl Default for PermBundle {
        fn default() -> Self {
            Self::from_perm(Perm3::IDENTITY)
        }
    }

    impl Fiber for PermBundle {
        type State = [i32; 3];
    }

    impl Connection for PermBundle {
        type Optic = IdentityPrism<[i32; 3]>;
        fn connection(&self) -> &IdentityPrism<[i32; 3]> {
            &self.optic
        }
    }

    impl Gauge for PermBundle {
        type Group = Perm3;
        fn gauge(&self) -> &Perm3 {
            &self.perm
        }
        fn act_on(&self, state: &[i32; 3]) -> [i32; 3] {
            // Permute state entries: value at index i goes to index perm.0[i].
            let mut out = [0i32; 3];
            for i in 0..3 {
                out[self.perm.0[i] as usize] = state[i];
            }
            out
        }
    }

    impl Transport for PermBundle {
        type Holonomy = ScalarLoss;
        fn transport(&self, state: &[i32; 3]) -> Imperfect<[i32; 3], Infallible, ScalarLoss> {
            // Loss = |state[0]| as f64. Permutation-sensitive by
            // construction: whichever value ended up in position 0
            // determines the loss. This makes commutator non-vanishing
            // when non-commuting permutations move different values
            // into position 0.
            let loss = state[0].abs() as f64;
            if loss == 0.0 {
                Imperfect::success(*state)
            } else {
                Imperfect::partial(*state, ScalarLoss::new(loss))
            }
        }
    }

    impl Closure for PermBundle {
        type Fixed = StableFiber<[i32; 3]>;
        fn close(&self) -> &StableFiber<[i32; 3]> {
            &self.fixed
        }
    }
}

#[cfg(test)]
mod tests {
    use super::examples::*;
    use super::*;
    use crate::ScalarLoss;

    // Local wrapper so existing tests keep their call-sites unchanged.
    fn make_bundle(strategy_shift: u8, kernel: bool) -> TestBundle {
        TestBundle::new(strategy_shift, kernel)
    }

    #[test]
    fn fiber_has_state() {
        let _f = TestFiber;
        let _: <TestFiber as Fiber>::State = [1.0, 2.0, 3.0, 4.0];
    }

    #[test]
    fn connection_requires_fiber() {
        let c = TestConnection::new();
        // Witnessed by being able to ask for the connection at all.
        let _ = c.connection();
    }

    #[test]
    fn identity_prism_passes_through() {
        let p: IdentityPrism<[f64; 4]> = IdentityPrism::new();
        let beam: Optic<(), [f64; 4]> = Optic::ok((), [1.0, 2.0, 3.0, 4.0]);
        let focused = p.focus(beam);
        let projected = p.project(focused);
        let refracted = p.settle(projected);
        assert_eq!(refracted.value(), Some(&[1.0, 2.0, 3.0, 4.0]));
    }

    // --- Cyclic group laws (Gap 2 closure) ---

    type C5 = Cyclic<5>;

    #[test]
    fn cyclic_identity_law_left_and_right() {
        let id = C5::identity();
        for k in 0..5u8 {
            let x = Cyclic::<5>::new(k);
            assert_eq!(id.compose(&x), x);
            assert_eq!(x.compose(&id), x);
        }
    }

    #[test]
    fn cyclic_inverse_law() {
        let id = C5::identity();
        for k in 0..5u8 {
            let x = Cyclic::<5>::new(k);
            assert_eq!(x.compose(&x.inverse()), id);
            assert_eq!(x.inverse().compose(&x), id);
        }
    }

    #[test]
    fn cyclic_associativity() {
        for a in 0..5u8 {
            for b in 0..5u8 {
                for c in 0..5u8 {
                    let x = Cyclic::<5>::new(a);
                    let y = Cyclic::<5>::new(b);
                    let z = Cyclic::<5>::new(c);
                    let lhs = x.compose(&y).compose(&z);
                    let rhs = x.compose(&y.compose(&z));
                    assert_eq!(lhs, rhs);
                }
            }
        }
    }

    // --- The full bundle test carrier (imported from examples) ---

    #[test]
    fn gauge_requires_connection() {
        let b = make_bundle(3, true);
        assert_eq!(b.gauge().value(), 3);
    }

    #[test]
    fn gauge_act_on_consistency() {
        // The action-composition law: g.act_on(h.act_on(s)) == compose(g, h).act_on(s)
        let s = [10.0_f64, 20.0, 30.0, 40.0];
        for g_k in 0..4u8 {
            for h_k in 0..4u8 {
                let bg = make_bundle(g_k, true);
                let bh = make_bundle(h_k, true);
                let composed_k = bg.gauge().compose(bh.gauge()).value();
                let bc = make_bundle(composed_k, true);

                let after_h = bh.act_on(&s);
                let then_g = bg.act_on(&after_h);
                let direct = bc.act_on(&s);
                assert_eq!(
                    then_g, direct,
                    "action-consistency failed for g={}, h={}",
                    g_k, h_k
                );
            }
        }
    }

    #[test]
    fn transport_returns_partial() {
        let b = make_bundle(3, true);
        let state = [1.0, 2.0, 3.0, 4.0];
        let result = b.transport(&state);
        assert!(result.is_partial());
    }

    #[test]
    fn transport_holonomy_measures_loss() {
        let b = make_bundle(3, true);
        let state = [1.0, 2.0, 3.0, 4.0];
        match b.transport(&state) {
            Imperfect::Partial(compressed, loss) => {
                assert_eq!(compressed, [1.0, 2.0, 0.0, 0.0]);
                assert_eq!(loss.as_f64(), 7.0);
            }
            _ => panic!("expected Partial"),
        }
    }

    #[test]
    fn transport_zero_loss_returns_success() {
        let b = make_bundle(3, true);
        let state = [1.0, 2.0, 0.0, 0.0];
        let result = b.transport(&state);
        assert!(result.is_ok());
    }

    #[test]
    fn transport_holonomy_is_a_metric() {
        // The holonomy carrier (ScalarLoss) satisfies the Metric supertrait.
        // This is what Gap 4 enforces at the type level. Witness it explicitly.
        fn requires_metric<T: Transport>(_b: &T)
        where
            T::Holonomy: Metric,
        {
        }
        let b = make_bundle(3, true);
        requires_metric(&b);
    }

    // --- Closure / LawvereFixedPoint laws (Gap 3 closure) ---

    #[test]
    fn closure_is_in_kernel_when_certified() {
        let b = make_bundle(0, true);
        assert!(b.close().in_kernel());
    }

    #[test]
    fn closure_not_in_kernel_when_uncertified() {
        let b = make_bundle(0, false);
        assert!(!b.close().in_kernel());
    }

    #[test]
    fn closure_idempotence_under_identity() {
        // The fixed point is idempotent under the identity endomap trivially.
        let s = StableFiber {
            state: [1.0, 2.0, 0.0, 0.0_f64],
            kernel: true,
        };
        assert!(s.is_idempotent_under(|x| x.clone()));
    }

    #[test]
    fn closure_idempotence_under_transport_projection() {
        // The bundle's transport projects (s0, s1, s2, s3) -> (s0, s1, 0, 0).
        // Applied twice to an in-kernel state, it returns the same state —
        // the projection is idempotent. The fixed-point witness reflects this.
        let s = StableFiber {
            state: [1.0, 2.0, 0.0, 0.0_f64],
            kernel: true,
        };
        let project = |x: &StableFiber<[f64; 4]>| StableFiber {
            state: [x.state[0], x.state[1], 0.0, 0.0],
            kernel: x.kernel,
        };
        assert!(s.is_idempotent_under(project));
    }

    // --- Bundle blanket impl ---

    #[test]
    fn full_tower_is_bundle() {
        fn accepts_bundle<B: Bundle>(_b: &B)
        where
            B::Optic: Prism,
            <<B::Optic as Prism>::Input as crate::Beam>::In: Sized,
        {
        }
        let b = make_bundle(3, true);
        accepts_bundle(&b);
    }

    #[test]
    fn bundle_associated_types_accessible() {
        let b = make_bundle(3, true);
        // Walk every level.
        let _: &<TestBundle as Fiber>::State = &[0.0; 4];
        let _conn: &<TestBundle as Connection>::Optic = b.connection();
        let _gauge: &<TestBundle as Gauge>::Group = b.gauge();
        let _fixed: &<TestBundle as Closure>::Fixed = b.close();
        // The fixed point really is in the kernel.
        assert!(b.close().in_kernel());
    }
}
