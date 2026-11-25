//! The judges of [`poly`](super) (docs/spec/laws.md, "Judges"): a measured
//! value, taken as an argument beside the law, read once as the exact dyadic
//! rational it is, and matched against the exact roots. Nothing here stores,
//! returns or computes in a float: each float parameter is read exactly once,
//! through `BigRational::from_float`, and everything after is exact.

use num_rational::BigRational;
use num_traits::Signed;

use super::{Algebraic, Factorization, Identified, Located, Roots};

/// The closed window [m − r, m + r], exactly: `None` when the radius is
/// negative.
fn window(m: BigRational, r: BigRational) -> Option<(BigRational, BigRational)> {
    if r.is_negative() {
        return None;
    }
    Some((&m - &r, &m + &r))
}

impl Roots {
    /// Which root a measured value is: the roots in the closed window
    /// [measured − radius, measured + radius], counted exactly (each float
    /// is the dyadic rational it is). One root: [`Located::Root`] with its
    /// index. Several: [`Located::Ambiguous`], a refusal naming them. None:
    /// [`Located::Absent`]. A NaN, an infinity or a negative radius:
    /// [`Located::Unmeasured`].
    pub fn locate(&self, measured: f64, radius: f64) -> Located {
        let (Some(m), Some(r)) = (
            BigRational::from_float(measured),
            BigRational::from_float(radius),
        ) else {
            return Located::Unmeasured;
        };
        match window(m, r) {
            Some((lo, hi)) => self.located(&lo, &hi),
            None => Located::Unmeasured,
        }
    }

    /// The roots in the closed window [lo, hi], exactly.
    fn located(&self, lo: &BigRational, hi: &BigRational) -> Located {
        let (below, on_edge) = self.below(lo);
        let count = self.count_in(lo, hi) + usize::from(on_edge);
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
    /// counted exactly ([`Roots::locate`]'s window on each). One root: its
    /// tag, the factor as minimal polynomial and the root's index among that
    /// factor's real roots. Several: a refusal naming them.
    pub fn identify(&self, measured: f64, radius: f64) -> Identified {
        // No factor has a root to find: absent, whatever was measured.
        if self.factors.is_empty() {
            return Identified::Absent;
        }
        let (Some(m), Some(r)) = (
            BigRational::from_float(measured),
            BigRational::from_float(radius),
        ) else {
            return Identified::Unmeasured;
        };
        let Some((lo, hi)) = window(m, r) else {
            return Identified::Unmeasured;
        };
        let mut hits = Vec::new();
        for (f, roots) in self.factors.iter().zip(&self.roots) {
            let tag = |index| Algebraic::new(f.clone(), index);
            match roots.located(&lo, &hi) {
                Located::Unmeasured | Located::Absent => {}
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
