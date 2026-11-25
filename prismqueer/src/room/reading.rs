//! The measured half of [`room`](super) (`lapack`): the one explicit step
//! from exact entries to `f64` ([`Measure`]), and the readings LAPACK gives
//! of a measured map: [`Modes`] (dsyev), the spectrum alone, and the singular
//! values (dgesdd). Nothing exact depends on it (docs/spec/features.md).

use num_bigint::BigInt;
use num_traits::ToPrimitive;

use super::{Coefficient, Entry, Kind, Map, Room, RoomError, Symmetric, Z};
use crate::field::Exact;

/// Exact entries that can be measured: the one explicit step to `f64`.
pub trait Measure: Entry + Exact {
    /// This entry as a float.
    fn measure(&self) -> f64;
}

impl Measure for Z {
    /// Exact up to 2⁵³ in magnitude; beyond, the nearest float, and ±∞ past
    /// `f64`'s range.
    fn measure(&self) -> f64 {
        self.0.to_f64().expect("every BigInt has a float reading")
    }
}

impl Coefficient for f64 {
    type Ring = ();
    fn ring(&self) -> &() {
        &()
    }
    fn zero(_: &()) -> Self {
        0.0
    }
    fn add(&self, other: &Self) -> Self {
        self + other
    }
    fn neg(&self) -> Self {
        -self
    }
    fn times(&self, n: &BigInt) -> Self {
        n.to_f64().expect("every BigInt has a float reading") * self
    }
}

impl Entry for f64 {
    fn one(_: &()) -> Self {
        1.0
    }
    fn mul(&self, other: &Self) -> Self {
        self * other
    }
}

impl<A: Kind, B: Kind, R: Measure> Map<A, B, R> {
    /// The one explicit step from exact to measured: every entry as `f64`.
    pub fn measure(&self) -> Map<A, B, f64> {
        self.with(self.entries.iter().map(Measure::measure).collect(), ())
    }
}

impl<A: Kind, B: Kind> Map<A, B, f64> {
    /// The Frobenius norm: the square root of the sum of squared entries.
    pub(crate) fn frobenius(&self) -> f64 {
        self.entries.iter().map(|x| x * x).sum::<f64>().sqrt()
    }

    /// The singular values, descending, by LAPACK's dgesdd. The matrix goes
    /// over as stored (column-major). `Err` is dgesdd's `info`.
    pub(crate) fn singular_values(&self) -> Result<Vec<f64>, crate::ffi::Fault> {
        crate::ffi::singular_values_columns(self.to.dim, self.from.dim, &self.entries)
    }
}

impl<A: Kind, R: Measure> Symmetric<A, R> {
    /// Measured: every entry as `f64`. A mirror stays a mirror.
    pub fn measure(&self) -> Symmetric<A, f64> {
        Symmetric(self.0.measure())
    }
}

impl<A: Kind> Symmetric<A, f64> {
    /// The room's resonances: eigenvalues and eigenvectors, by LAPACK's dsyev.
    /// The matrix goes over as stored (column-major), without a copy.
    pub fn decompose(&self) -> Result<Modes<A>, RoomError> {
        let map = &self.0;
        let (spectrum, vectors) = crate::ffi::eigensystem_columns(map.from.dim, &map.entries)
            .map_err(RoomError::Unmeasured)?;
        Ok(Modes {
            room: map.from.clone(),
            spectrum,
            vectors,
        })
    }

    /// The room's resonant values alone, ascending: its spectrum without the
    /// modes, by dsyev('N'). Not [`decompose`](Self::decompose)'s spectrum to
    /// the bit: dsyev('V') solves the tridiagonal problem another way.
    pub fn spectrum(&self) -> Result<Vec<f64>, RoomError> {
        let map = &self.0;
        crate::ffi::eigenvalues_columns(map.from.dim, &map.entries).map_err(RoomError::Unmeasured)
    }
}

/// The modes of a room: how a self-adjoint map on it resonates.
///
/// The spectrum is sorted ascending; mode `k` is eigenvalue `k` with
/// eigenvector `k`. Eigenvectors are read only whole, as columns.
#[derive(Clone, Debug, PartialEq)]
pub struct Modes<A: Kind> {
    room: Room<A>,
    spectrum: Vec<f64>,
    /// Column-major: eigenvector `k` is `vectors[k * n..(k + 1) * n]`.
    vectors: Vec<f64>,
}

impl<A: Kind> Modes<A> {
    /// The room that resonates.
    pub fn room(&self) -> &Room<A> {
        &self.room
    }

    /// The eigenvalues, ascending.
    pub fn spectrum(&self) -> &[f64] {
        &self.spectrum
    }

    /// Eigenvector `k`, belonging to `spectrum()[k]`: a vector of the room.
    pub fn vector(&self, k: usize) -> Option<&[f64]> {
        let n = self.room.dim;
        (k < n).then(|| &self.vectors[k * n..(k + 1) * n])
    }

    /// Every eigenvector, in spectrum order.
    pub fn vectors(&self) -> impl Iterator<Item = &[f64]> {
        (0..self.room.dim).filter_map(move |k| self.vector(k))
    }
}
