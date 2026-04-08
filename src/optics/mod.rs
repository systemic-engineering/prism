//! Optics — the composition layer for the Prism trait.
//!
//! A Prism with `type Crystal = Self` is fundamentally an endofunction
//! `Beam<T> → Beam<T>`. Endofunctions under composition form a monoid.
//! This module makes that structure first-class and builds a kit of
//! classical functional optics as specific shapes of monoid
//! homomorphisms between beam levels.
//!
//! Four layers:
//!
//! 1. **Monoid** (`monoid`): the `PrismMonoid` trait witnessing the
//!    `Crystal = Self` closure, `IdPrism<T>` as identity, `Compose<P1, P2>`
//!    for sequential composition, and the `CountPrism` test helper
//!    proving the monoid laws on a non-trivial element.
//!
//! 2. **Gather** (`gather`): strategies for collapsing `Vec<Beam<T>>`
//!    into `Beam<T>`. `SumGather` concatenates, `MaxGather` picks the
//!    highest-precision beam, `FirstGather` takes the first.
//!
//! 3. **Meta-prism** (`meta`): `MetaPrism<T, G>` operates on populations
//!    of beams. Its `Input` is `Vec<Beam<T>>`; its `project` collapses
//!    the population via the Gather strategy. This is the morphism
//!    between level 0 (`Beam<T>`) and level 1 (`Vec<Beam<T>>`).
//!
//! 4. **Classical optics**: six named optic kinds, each implementing
//!    the base `Prism` trait:
//!    - `iso::Iso<A, B>` — total invertible
//!    - `lens::Lens<S, A>` — total bidirectional single-focus
//!    - `traversal::Traversal<A, B>` — multi-focus lift
//!    - `optic_prism::OpticPrism<S, A>` — semidet bidirectional
//!    - `setter::Setter<S, A>` — write-only
//!    - `fold::Fold<S, A>` — multi read-only
//!
//! `split` is the single operation that leaves the single-beam monoid:
//! it produces `Vec<Beam<T>>`, which is handled at level 1 via meta-prisms.
//! Gathering the population back into one beam is a meta-prism's `project`,
//! not a method on the base trait.
//!
//! Enabled via `features = ["optics"]` on the `prism` dependency.

pub mod monoid;
pub mod gather;
pub mod meta;
pub mod phantom_crystal;
pub mod iso;
pub mod lens;
pub mod traversal;
pub mod optic_prism;
pub mod setter;
pub mod fold;
