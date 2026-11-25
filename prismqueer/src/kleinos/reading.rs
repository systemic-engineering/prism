//! The measured half of [`kleinos`](super) (`lapack`): λ₂ of the glued
//! party's room multigraph, by LAPACK, a reading beside the address.

use super::Pushout;
use crate::band::Measurement;
use crate::gl::Scalar;
use crate::room::RoomError;

/// How [`Pushout::fiedler`] is measured.
pub const FIEDLER_METHOD: &str =
    "L₀ = δ*δ on the span multigraph (room), dsyev('N'), second eigenvalue; counts messages";

impl<R: Scalar> Pushout<R> {
    /// λ₂ of the room multigraph, measured: a reading beside the address,
    /// never a law. Between two parties joined by k spans it is 2k.
    pub fn fiedler(&self) -> Result<Measurement, RoomError> {
        let spectrum = crate::room::laplacian(self.composite.graph())
            .measure()
            .spectrum()?;
        Ok(Measurement::here(
            spectrum.get(1).copied().unwrap_or(0.0),
            FIEDLER_METHOD,
        ))
    }
}
