//! The measured half of [`poly`](super) (`lapack`): a measured value, read
//! as the exact dyadic rational it is, matched against the exact roots.

use num_rational::BigRational;
use num_traits::Signed;

use super::{Algebraic, Factorization, Identified, Located, Roots};

impl Roots {
    /// Which root a measured value is: the roots in the closed window
    /// [measured − radius, measured + radius], counted exactly (each float
    /// is the dyadic rational it is). One root: [`Located::Root`] with its
    /// index. Several: [`Located::Ambiguous`], a refusal naming them. None:
    /// [`Located::Absent`].
    pub fn locate(&self, measured: f64, radius: f64) -> Located {
        let (Some(m), Some(r)) = (
            BigRational::from_float(measured),
            BigRational::from_float(radius),
        ) else {
            return Located::Unmeasured;
        };
        if r.is_negative() {
            return Located::Unmeasured;
        }
        let (lo, hi) = (&m - &r, &m + &r);
        let (below, on_edge) = self.below(&lo);
        let count = self.count_in(&lo, &hi) + usize::from(on_edge);
        match count {
            0 => Located::Absent,
            1 => Located::Root(below),
            _ => Located::Ambiguous(below..below + count),
        }
    }
}

impl Factorization {
    /// Which exact algebraic number a measured value is: the roots of every
    /// factor in the closed window [measured − radius, measured + radius],
    /// counted exactly ([`Roots::locate`] on each). One root: its tag, the
    /// factor as minimal polynomial and the root's index among that
    /// factor's real roots. Several: a refusal naming them.
    pub fn identify(&self, measured: f64, radius: f64) -> Identified {
        let mut hits = Vec::new();
        for (f, roots) in self.factors.iter().zip(&self.roots) {
            let tag = |index| Algebraic::new(f.clone(), index);
            match roots.locate(measured, radius) {
                Located::Unmeasured => return Identified::Unmeasured,
                Located::Absent => {}
                Located::Root(i) => hits.push(tag(i)),
                Located::Ambiguous(range) => hits.extend(range.map(tag)),
            }
        }
        match hits.len() {
            0 => Identified::Absent,
            1 => Identified::Algebraic(hits.pop().expect("one")),
            _ => Identified::Ambiguous(hits),
        }
    }
}
