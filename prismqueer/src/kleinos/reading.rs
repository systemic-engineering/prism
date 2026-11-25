//! The measured half of [`kleinos`](super) (`lapack`): λ₂ of the glued
//! party's room multigraph, by LAPACK, a reading beside the address.

use prism_laws::gl::Scalar;
use prism_laws::kleinos::Pushout;

use crate::band::Measurement;
use crate::ffi::Fault;
use crate::room::{Measuring, Resonant};

/// How [`Fiedler::fiedler`] is measured.
pub const FIEDLER_METHOD: &str =
    "L₀ = δ*δ on the span multigraph (room), dsyev('N'), second eigenvalue; counts messages";

/// λ₂ of a pushout, measured.
pub trait Fiedler {
    /// λ₂ of the room multigraph, measured: a reading beside the address,
    /// never a law. Between two parties joined by k spans it is 2k.
    fn fiedler(&self) -> Result<Measurement, Fault>;
}

impl<R: Scalar> Fiedler for Pushout<R> {
    fn fiedler(&self) -> Result<Measurement, Fault> {
        let spectrum = crate::room::laplacian(self.composite().graph())
            .measure()
            .spectrum()?;
        Ok(Measurement::here(
            spectrum.get(1).copied().unwrap_or(0.0),
            FIEDLER_METHOD,
        ))
    }
}
