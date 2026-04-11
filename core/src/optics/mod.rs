//! Optics — classical functional optics expressed through the Beam/Prism system.
//!
//! The optics hierarchy (Iso → Lens → OpticPrism → Traversal → Fold/Setter)
//! is preserved. Each optic implements the `Prism` trait using `PureBeam` as
//! the concrete beam type.
//!
//! Key design:
//! - Each optic has inherent methods (forward/backward, view/set, etc.)
//! - Each optic implements `Prism` with `PureBeam` beams
//! - Composition uses `then_*` methods (optic-to-optic) and `smap` (user-space)
//! - Gather strategies collapse `Vec<T>` into `T` (used by Traversal/Fold)
//! - `PrismMonoid` captures the monoid structure of the optics
//!
//! Enabled via `features = ["optics"]` on the `prism-core` dependency.

pub mod monoid;
pub mod gather;
pub mod iso;
pub mod lens;
pub mod traversal;
pub mod optic_prism;
pub mod setter;
pub mod fold;
