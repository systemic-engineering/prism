//! `kleinos`: a law of prism's floor, re-exported at its old path, with the
//! lab's measured half on top. The pushout along spans lives in
//! `prism_laws::kleinos` (docs/spec/laws.md).

pub use prism_laws::kleinos::*;

// The measured half (`lapack`): the Fiedler value, a reading beside the
// address.
#[cfg(feature = "lapack")]
mod reading;
#[cfg(feature = "lapack")]
pub use reading::{Fiedler, FIEDLER_METHOD};
