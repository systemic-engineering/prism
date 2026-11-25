//! The measured half of [`room`](super) (`lapack`): the lab's float carrier
//! ([`F64`]), the one explicit step from exact entries onto it ([`Measure`],
//! [`Measuring`]), and the readings LAPACK gives of a measured map:
//! [`Modes`] (dsyev), the spectrum alone ([`Resonant`]), and the singular
//! values (dgesdd). Nothing exact depends on it (docs/spec/laws.md).

use num_bigint::BigInt;
use num_traits::ToPrimitive;

use prism_laws::field::Exact;
use prism_laws::room::{Coefficient, Entry, Kind, Map, Room, Symmetric, Z};

use crate::ffi::Fault;

/// A measured entry: the lab's float carrier (name provisional). A room's
/// map over it is a measurement, `Map<A, B, F64>`; it rounds, so it is no
/// [`Exactly`](prism_laws::room::Exactly) and gets no `solve`.
///
/// The floor admits no float entry (docs/spec/laws.md): this type is the
/// lab's, and so is every map over it. It is where a measurement's
/// provenance (machine and method, contract rule 7) will live; for now it
/// is a bare carrier.
#[derive(Clone, Copy, Debug, PartialEq, PartialOrd)]
pub struct F64(pub f64);

impl Coefficient for F64 {
    type Ring = ();
    fn ring(&self) -> &() {
        &()
    }
    fn zero(_: &()) -> Self {
        F64(0.0)
    }
    fn add(&self, other: &Self) -> Self {
        F64(self.0 + other.0)
    }
    fn neg(&self) -> Self {
        F64(-self.0)
    }
    fn times(&self, n: &BigInt) -> Self {
        F64(n.to_f64().expect("every BigInt has a float reading") * self.0)
    }
}

impl Entry for F64 {
    fn one(_: &()) -> Self {
        F64(1.0)
    }
    fn mul(&self, other: &Self) -> Self {
        F64(self.0 * other.0)
    }
}

/// Exact entries that can be measured: the one explicit step to a float.
pub trait Measure: Entry + Exact {
    /// This entry as a float.
    fn measure(&self) -> f64;
}

impl Measure for Z {
    /// Exact up to 2⁵³ in magnitude; beyond, the nearest float, and ±∞ past
    /// `f64`'s range.
    fn measure(&self) -> f64 {
        self.value()
            .to_f64()
            .expect("every BigInt has a float reading")
    }
}

/// The one explicit step from an exact map (or mirror) to a measured one:
/// every entry onto [`F64`]. A mirror stays a mirror.
pub trait Measuring {
    /// The measured map.
    type Measured;
    /// Every entry as a float.
    fn measure(&self) -> Self::Measured;
}

impl<A: Kind, B: Kind, R: Measure> Measuring for Map<A, B, R> {
    type Measured = Map<A, B, F64>;
    fn measure(&self) -> Map<A, B, F64> {
        self.entrywise((), |x| F64(x.measure()))
    }
}

impl<A: Kind, R: Measure> Measuring for Symmetric<A, R> {
    type Measured = Symmetric<A, F64>;
    fn measure(&self) -> Symmetric<A, F64> {
        self.entrywise((), |x| F64(x.measure()))
    }
}

/// The column-major entries of a measured map, as LAPACK takes them.
fn floats<A: Kind, B: Kind>(map: &Map<A, B, F64>) -> Vec<f64> {
    map.column_major().iter().map(|x| x.0).collect()
}

/// The norms the lab's commutator reads of a measured map.
pub(crate) trait Norms {
    /// The Frobenius norm: the square root of the sum of squared entries.
    fn frobenius(&self) -> f64;
    /// The singular values, descending, by LAPACK's dgesdd. The matrix goes
    /// over as stored (column-major). `Err` is dgesdd's `info`.
    fn singular_values(&self) -> Result<Vec<f64>, Fault>;
}

impl<A: Kind, B: Kind> Norms for Map<A, B, F64> {
    fn frobenius(&self) -> f64 {
        self.column_major()
            .iter()
            .map(|x| x.0 * x.0)
            .sum::<f64>()
            .sqrt()
    }

    fn singular_values(&self) -> Result<Vec<f64>, Fault> {
        crate::ffi::singular_values_columns(self.to().dim(), self.from().dim(), &floats(self))
    }
}

/// A measured mirror's resonances, by LAPACK. A reading fails only to
/// measure: its `Err` is what LAPACK, or prism's check before it, said.
pub trait Resonant<A: Kind> {
    /// The room's resonances: eigenvalues and eigenvectors, by dsyev.
    fn decompose(&self) -> Result<Modes<A>, Fault>;
    /// The room's resonant values alone, ascending: its spectrum without
    /// the modes, by dsyev('N'). Not [`decompose`](Self::decompose)'s
    /// spectrum to the bit: dsyev('V') solves the tridiagonal problem
    /// another way.
    fn spectrum(&self) -> Result<Vec<f64>, Fault>;
}

impl<A: Kind> Resonant<A> for Symmetric<A, F64> {
    fn decompose(&self) -> Result<Modes<A>, Fault> {
        let map = self.map();
        let (spectrum, vectors) = crate::ffi::eigensystem_columns(map.from().dim(), &floats(map))?;
        Ok(Modes {
            room: map.from().clone(),
            spectrum,
            vectors,
        })
    }

    fn spectrum(&self) -> Result<Vec<f64>, Fault> {
        let map = self.map();
        crate::ffi::eigenvalues_columns(map.from().dim(), &floats(map))
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
        let n = self.room.dim();
        (k < n).then(|| &self.vectors[k * n..(k + 1) * n])
    }

    /// Every eigenvector, in spectrum order.
    pub fn vectors(&self) -> impl Iterator<Item = &[f64]> {
        (0..self.room.dim()).filter_map(move |k| self.vector(k))
    }
}
