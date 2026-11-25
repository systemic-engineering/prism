//! `prism_laws::measured` — a precision matrix estimated from stream
//! windows, exactly over ℚ (prism-023 §1, `docs/spec/measured.md`). The
//! names are provisional.
//!
//! # Measured, not yet a `Metrix`
//!
//! A [`Measured`] is the precision matrix of `d` streams, fitted from `n`
//! samples of **integer counts** under a declared [`Support`]. Integer
//! counts make the sample covariance rational, and every inverse taken is
//! exact: there is no float on this path, and no float decides anything.
//!
//! - **The fit, never a mask.** The declared edges are the support of the
//!   precision matrix; a zero off it is declared conditional independence
//!   (D6). Full support gives S⁻¹. Chordal support gives the covariance
//!   selection MLE in closed form, K̂ = Σ_C [S_C⁻¹]⁰ − Σ_T [S_T⁻¹]⁰ over the
//!   cliques C and separators T (Lauritzen 1996 §5.3; Dempster 1972).
//! - **Non-chordal support is refused**, [`Refused::Unresolved`]: it has no
//!   closed form, and iteration isn't exact. Never approximated.
//! - **Chordality, exactly:** maximum cardinality search and a
//!   perfect-elimination check (Tarjan & Yannakakis 1984), in [`decompose`].
//! - **Addressed, unsigned.** The address is `"measured\0" ‖ bytes(window)
//!   ‖ bytes(frame) ‖ method ‖ bytes(samples)`, and a `Measured` holds no
//!   signer.
//!
//! A `Measured` is not a `Metrix`:
//!
//! ```compile_fail,E0308
//! use prism_laws::measured::Measured;
//! use prism_laws::metrix::Metrix;
//! fn wants(m: &Measured) -> &Metrix {
//!     m
//! }
//! let _ = wants;
//! ```
//!
//! The control: the road to one is a draft (the LDLᵀ certificate) and a
//! seal (a signature).
//!
//! ```
//! use prism_laws::measured::Measured;
//! use prism_laws::metrix::{Metrix, MetrixError};
//! use prism_laws::oid::Oid;
//! fn road(m: &Measured, by: Oid) -> Result<Metrix, MetrixError> {
//!     Ok(m.draft()?.seal(by))
//! }
//! let _ = road;
//! ```

use std::collections::BTreeSet;

use num_bigint::BigInt;
use num_rational::BigRational;
use num_traits::Zero;

use crate::algedonic::Ruler;
use crate::gl::rational;
use crate::hypothesis::Frame;
use crate::metrix::{Metrix, MetrixDraft, MetrixError};
use crate::oid::{Oid, Scope};
use crate::room::{Q, Z};

/// The denominator of the sample covariance.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Estimator {
    /// Divide by n: the maximum-likelihood estimate.
    Mle,
    /// Divide by n − 1.
    Unbiased,
}

impl Estimator {
    fn tag(self) -> u8 {
        match self {
            Estimator::Mle => 1,
            Estimator::Unbiased => 2,
        }
    }

    /// The fewest samples its denominator allows.
    fn needs(self) -> usize {
        match self {
            Estimator::Mle => 1,
            Estimator::Unbiased => 2,
        }
    }

    fn divisor(self, n: usize) -> usize {
        n + 1 - self.needs()
    }
}

/// The declared edges between streams: unordered pairs, the diagonal
/// implied. The support of the precision matrix.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Support {
    edges: Vec<(usize, usize)>,
}

impl Support {
    /// These edges, each written (lower, higher), ascending, once.
    pub fn new(edges: impl IntoIterator<Item = (usize, usize)>) -> Self {
        let set: BTreeSet<(usize, usize)> = edges
            .into_iter()
            .map(|(a, b)| (a.min(b), a.max(b)))
            .collect();
        Support {
            edges: set.into_iter().collect(),
        }
    }

    /// Every pair of `d` streams.
    pub fn full(d: usize) -> Self {
        Support::new((0..d).flat_map(|i| (i + 1..d).map(move |j| (i, j))))
    }

    /// The edges, (lower, higher), ascending.
    pub fn edges(&self) -> &[(usize, usize)] {
        &self.edges
    }
}

/// How an estimate was made: signed data, part of its address.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Method {
    /// The covariance's denominator.
    pub estimator: Estimator,
    /// The declared edges.
    pub support: Support,
}

/// What is the wrong shape.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Shape {
    /// A sample whose length isn't the frame's dimension.
    Sample {
        /// Which sample, from 0.
        index: usize,
        /// Its length.
        len: usize,
        /// The frame's dimension.
        streams: usize,
    },
    /// An edge naming a stream past the frame, or a stream and itself.
    Edge {
        /// The lower end.
        from: usize,
        /// The higher end.
        to: usize,
    },
}

/// Why nothing was measured, or why a measurement wasn't sealed. Every
/// refusal is a value; none is "fixed".
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Refused {
    /// The samples or the support don't fit the frame.
    WrongShape(Shape),
    /// The support isn't chordal: no exact closed form.
    Unresolved(Support),
    /// n is at most the largest clique (or below the estimator's floor).
    TooFewSamples {
        /// n.
        samples: usize,
        /// The fewest that would do.
        needed: usize,
    },
    /// The covariance on these streams is singular.
    NotInvertible {
        /// The streams, ascending.
        clique: Vec<usize>,
    },
    /// Sealed by a ruler that isn't the measurement's window.
    OtherWindow {
        /// The window measured under.
        expected: Oid,
        /// The ruler offered.
        found: Oid,
    },
    /// The draft refused the estimate.
    Draft(MetrixError),
}

impl From<MetrixError> for Refused {
    fn from(e: MetrixError) -> Self {
        Refused::Draft(e)
    }
}

/// A chordal support, decomposed: the maximum cardinality search order,
/// the maximal cliques in order of discovery, and each later clique's
/// separator from those before it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Decomposition {
    order: Vec<usize>,
    cliques: Vec<Vec<usize>>,
    separators: Vec<Vec<usize>>,
}

impl Decomposition {
    /// The streams in the order the search visited them.
    pub fn order(&self) -> &[usize] {
        &self.order
    }

    /// The maximal cliques, each ascending, in order of discovery.
    pub fn cliques(&self) -> &[Vec<usize>] {
        &self.cliques
    }

    /// For the k-th clique (k ≥ 1), Cₖ ∩ (C₀ ∪ … ∪ Cₖ₋₁); empty between
    /// components.
    pub fn separators(&self) -> &[Vec<usize>] {
        &self.separators
    }
}

/// Decompose `support` on `d` streams, if it is chordal.
///
/// Maximum cardinality search visits the unvisited stream with the most
/// visited neighbours (lowest index on a tie). The support is chordal iff
/// every stream's earlier-visited neighbours are pairwise adjacent; the
/// first that aren't is [`Refused::Unresolved`].
pub fn decompose(d: usize, support: &Support) -> Result<Decomposition, Refused> {
    if let Some(&(from, to)) = support.edges.iter().find(|&&(a, b)| a == b || b >= d) {
        return Err(Refused::WrongShape(Shape::Edge { from, to }));
    }
    let mut adjacent = vec![vec![false; d]; d];
    for &(a, b) in &support.edges {
        adjacent[a][b] = true;
        adjacent[b][a] = true;
    }
    let mut visited = vec![false; d];
    let mut weight = vec![0usize; d];
    let mut order: Vec<usize> = Vec::with_capacity(d);
    let mut candidates: Vec<Vec<usize>> = Vec::with_capacity(d);
    for _ in 0..d {
        let v = (0..d)
            .filter(|&v| !visited[v])
            .fold(None, |best: Option<usize>, v| match best {
                Some(b) if weight[b] >= weight[v] => Some(b),
                _ => Some(v),
            })
            .unwrap_or_default();
        let earlier: Vec<usize> = order.iter().copied().filter(|&u| adjacent[u][v]).collect();
        let clique = earlier
            .iter()
            .all(|&a| earlier.iter().all(|&b| a == b || adjacent[a][b]));
        if !clique {
            return Err(Refused::Unresolved(support.clone()));
        }
        let mut candidate = earlier;
        candidate.push(v);
        candidate.sort_unstable();
        candidates.push(candidate);
        visited[v] = true;
        order.push(v);
        for u in 0..d {
            if adjacent[v][u] && !visited[u] {
                weight[u] += 1;
            }
        }
    }
    let inside = |a: &[usize], b: &[usize]| a.len() < b.len() && a.iter().all(|x| b.contains(x));
    let cliques: Vec<Vec<usize>> = candidates
        .iter()
        .filter(|c| !candidates.iter().any(|other| inside(c, other)))
        .cloned()
        .collect();
    let separators = (1..cliques.len())
        .map(|k| {
            cliques[k]
                .iter()
                .copied()
                .filter(|x| cliques[..k].iter().any(|c| c.contains(x)))
                .collect()
        })
        .collect();
    Ok(Decomposition {
        order,
        cliques,
        separators,
    })
}

/// The sample covariance of `counts` (n samples × the frame's d streams):
/// Sᵢⱼ = Σₛ (xₛᵢ − x̄ᵢ)(xₛⱼ − x̄ⱼ) / m, m = n or n − 1, exactly.
pub fn covariance(
    frame: &Frame,
    counts: &[Vec<Z>],
    estimator: Estimator,
) -> Result<Vec<Vec<Q>>, Refused> {
    shaped(frame.dim(), counts)?;
    let needed = estimator.needs();
    if counts.len() < needed {
        return Err(Refused::TooFewSamples {
            samples: counts.len(),
            needed,
        });
    }
    Ok(in_q(sample_covariance(frame.dim(), counts, estimator)))
}

/// Estimate the precision matrix of `counts` under `method`, in `frame`'s
/// basis, measured under the window ruler `window`.
///
/// Refused, in this order: a sample of the wrong length or an edge outside
/// the frame ([`Refused::WrongShape`]); a non-chordal support
/// ([`Refused::Unresolved`]); n ≤ the largest clique
/// ([`Refused::TooFewSamples`]); a clique whose covariance is singular
/// ([`Refused::NotInvertible`]).
pub fn measure(
    window: &Ruler,
    frame: &Frame,
    counts: &[Vec<Z>],
    method: Method,
) -> Result<Measured, Refused> {
    measure_under(window.oid(), frame, counts, method)
}

/// [`measure`], with the window named by its address alone: what a caller
/// that holds the address but not the ruler (prism-mcp's `measure` tool)
/// can give. The window is a claim the caller vouches for, as a signer's
/// identity is; nothing here reads more of the ruler than its address, and
/// [`Measured::seal`] still takes the ruler itself and refuses another
/// ([`Refused::OtherWindow`]).
pub fn measure_under(
    window: Oid,
    frame: &Frame,
    counts: &[Vec<Z>],
    method: Method,
) -> Result<Measured, Refused> {
    let d = frame.dim();
    shaped(d, counts)?;
    let parts = decompose(d, &method.support)?;
    let largest = parts.cliques.iter().map(Vec::len).max().unwrap_or(0);
    let needed = (largest + 1).max(method.estimator.needs());
    let n = counts.len();
    if n < needed {
        return Err(Refused::TooFewSamples { samples: n, needed });
    }
    let s = sample_covariance(d, counts, method.estimator);
    let mut k = vec![vec![BigRational::zero(); d]; d];
    for clique in &parts.cliques {
        add_padded(&mut k, clique, &inverse_on(&s, clique)?, false);
    }
    for separator in &parts.separators {
        add_padded(&mut k, separator, &inverse_on(&s, separator)?, true);
    }
    Ok(Measured {
        frame: frame.clone(),
        window,
        samples_oid: samples_oid(d, counts),
        method,
        samples: n,
        estimate: in_q(k),
    })
}

/// A precision matrix measured from a window: its frame, the window ruler's
/// address, the method, the sample count and address, and the estimate.
/// No signer. Not a [`Metrix`]: [`Measured::draft`] certifies it, and a seal
/// signs it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Measured {
    frame: Frame,
    window: Oid,
    method: Method,
    samples: usize,
    samples_oid: Oid,
    estimate: Vec<Vec<Q>>,
}

impl Measured {
    /// The frame whose basis the streams are.
    pub fn frame(&self) -> &Frame {
        &self.frame
    }

    /// The window ruler's address.
    pub fn window(&self) -> &Oid {
        &self.window
    }

    /// How it was estimated.
    pub fn method(&self) -> &Method {
        &self.method
    }

    /// n, the number of samples.
    pub fn samples(&self) -> usize {
        self.samples
    }

    /// The samples' address: `"samples\0" ‖ u64le(n) ‖ u64le(d) ‖
    /// rational(xₛⱼ)` row by row, in the public scope.
    pub fn samples_oid(&self) -> &Oid {
        &self.samples_oid
    }

    /// K̂, row by row.
    pub fn estimate(&self) -> &[Vec<Q>] {
        &self.estimate
    }

    /// The canonical preimage: `"measured\0" ‖ bytes(window) ‖ bytes(frame)
    /// ‖ method ‖ bytes(samples)`, with method = `estimator ‖ u64le(m) ‖
    /// (u64le(i) ‖ u64le(j))…`. No signer, and not the estimate (a function
    /// of the rest).
    pub fn preimage(&self) -> Vec<u8> {
        let mut out = b"measured\0".to_vec();
        delimited(&mut out, self.window.as_str().as_bytes());
        delimited(&mut out, self.frame.oid().as_str().as_bytes());
        out.push(self.method.estimator.tag());
        let edges = self.method.support.edges();
        out.extend_from_slice(&(edges.len() as u64).to_le_bytes());
        for &(i, j) in edges {
            out.extend_from_slice(&(i as u64).to_le_bytes());
            out.extend_from_slice(&(j as u64).to_le_bytes());
        }
        delimited(&mut out, self.samples_oid.as_str().as_bytes());
        out
    }

    /// Its address: the preimage, in the public scope.
    pub fn oid(&self) -> Oid {
        Oid::hash(&Scope::public(), &self.preimage())
    }

    /// The estimate as a draft metric: the LDLᵀ certificate, or the
    /// refusal at the failing pivot.
    pub fn draft(&self) -> Result<MetrixDraft, MetrixError> {
        MetrixDraft::new(&self.frame, self.estimate.clone())
    }

    /// Sealed by the window ruler's signer: refused if `window` isn't the
    /// ruler this was measured under. (Any signer may still seal
    /// [`Measured::draft`]; the `Metrix` names who did.)
    pub fn seal(self, window: &Ruler) -> Result<Metrix, Refused> {
        if window.oid() != self.window {
            return Err(Refused::OtherWindow {
                expected: self.window,
                found: window.oid(),
            });
        }
        Ok(self.draft()?.seal(window.signer().by().clone()))
    }
}

// --- Exact arithmetic ----------------------------------------------------------

fn shaped(d: usize, counts: &[Vec<Z>]) -> Result<(), Refused> {
    match counts.iter().position(|r| r.len() != d) {
        Some(index) => Err(Refused::WrongShape(Shape::Sample {
            index,
            len: counts[index].len(),
            streams: d,
        })),
        None => Ok(()),
    }
}

fn sample_covariance(d: usize, counts: &[Vec<Z>], estimator: Estimator) -> Vec<Vec<BigRational>> {
    let n = counts.len();
    let int = |k: usize| BigRational::from_integer(BigInt::from(k));
    let mean: Vec<BigRational> = (0..d)
        .map(|j| {
            let total: BigInt = counts.iter().map(|r| r[j].value()).sum();
            BigRational::new(total, BigInt::from(n))
        })
        .collect();
    let centred: Vec<Vec<BigRational>> = counts
        .iter()
        .map(|r| {
            r.iter()
                .zip(&mean)
                .map(|(x, m)| BigRational::from_integer(x.value().clone()) - m)
                .collect()
        })
        .collect();
    let m = int(estimator.divisor(n));
    (0..d)
        .map(|i| {
            (0..d)
                .map(|j| {
                    let sum: BigRational = centred.iter().map(|r| &r[i] * &r[j]).sum();
                    sum / &m
                })
                .collect()
        })
        .collect()
}

/// (S restricted to `set`)⁻¹, by Gauss–Jordan over ℚ; singular is refused.
fn inverse_on(s: &[Vec<BigRational>], set: &[usize]) -> Result<Vec<Vec<BigRational>>, Refused> {
    let k = set.len();
    let singular = || Refused::NotInvertible {
        clique: set.to_vec(),
    };
    let mut a: Vec<Vec<BigRational>> = set
        .iter()
        .enumerate()
        .map(|(r, &i)| {
            let mut row: Vec<BigRational> = set.iter().map(|&j| s[i][j].clone()).collect();
            row.extend((0..k).map(|c| BigRational::from_integer(BigInt::from(u8::from(r == c)))));
            row
        })
        .collect();
    for c in 0..k {
        let p = (c..k).find(|&r| !a[r][c].is_zero()).ok_or_else(singular)?;
        a.swap(c, p);
        let pivot = a[c][c].clone();
        a[c] = a[c].iter().map(|x| x / &pivot).collect();
        let row = a[c].clone();
        for (r, other) in a.iter_mut().enumerate() {
            if r != c && !other[c].is_zero() {
                let f = other[c].clone();
                for (x, y) in other.iter_mut().zip(&row) {
                    *x -= &f * y;
                }
            }
        }
    }
    Ok(a.into_iter().map(|r| r[k..].to_vec()).collect())
}

/// k += [m]⁰ on `set` (or −= when `subtract`).
fn add_padded(k: &mut [Vec<BigRational>], set: &[usize], m: &[Vec<BigRational>], subtract: bool) {
    for (r, &i) in set.iter().enumerate() {
        for (c, &j) in set.iter().enumerate() {
            if subtract {
                k[i][j] -= &m[r][c];
            } else {
                k[i][j] += &m[r][c];
            }
        }
    }
}

fn in_q(m: Vec<Vec<BigRational>>) -> Vec<Vec<Q>> {
    m.into_iter()
        .map(|r| r.into_iter().map(Q::from).collect())
        .collect()
}

fn samples_oid(d: usize, counts: &[Vec<Z>]) -> Oid {
    let mut out = b"samples\0".to_vec();
    out.extend_from_slice(&(counts.len() as u64).to_le_bytes());
    out.extend_from_slice(&(d as u64).to_le_bytes());
    for x in counts.iter().flatten() {
        rational(&mut out, &BigRational::from_integer(x.value().clone()));
    }
    Oid::hash(&Scope::public(), &out)
}

/// `bytes(x)` = `u64le(len) ‖ x`.
fn delimited(out: &mut Vec<u8>, x: &[u8]) {
    out.extend_from_slice(&(x.len() as u64).to_le_bytes());
    out.extend_from_slice(x);
}
