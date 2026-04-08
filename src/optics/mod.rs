//! Optics — the composition layer for the Prism trait.
//!
//! A Prism (for the homogeneous case `Crystal = Self`) is fundamentally
//! an endofunction `Beam<T> → Beam<T>`. Endofunctions under composition
//! form a monoid. This module makes that structure first-class:
//!
//! - `monoid` — the `PrismMonoid` trait, `IdPrism`, `Compose`
//! - `gather` — strategies for collapsing `Vec<Beam<T>>` into `Beam<T>`
//! - `meta`   — `MetaPrism`, which operates on populations of beams
//! - Classical optics: `Iso`, `Lens`, `Traversal`, `OpticPrism`, `Setter`, `Fold`
//!
//! Each classical optic is a specific shape of monoid homomorphism between
//! beam levels. `split` is the operation that leaves the single-beam monoid
//! (produces a `Vec<Beam<T>>`); meta-prisms gather it back.
//!
//! Enabled via `features = ["optics"]` on the `prism` dependency.

pub mod monoid;
pub mod gather;
pub mod meta;
pub mod iso;
pub mod lens;
pub mod traversal;
pub mod optic_prism;
pub mod setter;
pub mod fold;
