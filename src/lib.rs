//! Prism — the DATA DIVISION + the OPERATION DIVISION.
//!
//! Five operations: fold, prism, traversal, lens, iso.
//! The optic hierarchy as a trait. The parser's vocabulary.
//!
//! Beam<T>, Oid, ShannonLoss, Precision, Pressure, Recovery, ContentAddressed.
//! Zero dependencies. The types that outlive everything around them.
//!
//! A prism splits light into beams.

pub mod beam;
pub mod content;
pub mod loss;
pub mod oid;
pub mod optic;
pub mod precision;

pub use beam::{Beam, Recovery};
pub use content::ContentAddressed;
pub use loss::ShannonLoss;
pub use oid::Oid;
pub use optic::{compile, Prism};
pub use precision::{Precision, Pressure};
