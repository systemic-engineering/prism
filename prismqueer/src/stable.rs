//! `Stable`: the Prisms whose light only adds and never twists.
//!
//! prism-012, Ask 1, decided by Alex (2026-11-01): the stable/`Unstable`
//! kinds are a **marker trait**, a proof-carrying refinement of [`Prism`].
//! `Unstable` is the absence of the proof, not a type.
//!
//! [`Compose`] is sequential composition, the combinator that preserves
//! `Stable`. Together with `&P` it is every Prism-to-Prism combinator prism
//! has.

use crate::beam::{Beam, Optic};
use crate::{apply, Prism};
use terni::{Imperfect, Loss};

/// A Prism whose light only adds and never twists.
///
/// This is Mara's exact illuminating class, **no `not` and no declaration**,
/// carried as a type. An `impl Stable for P {}` is a proof obligation. It
/// promises three things about `P`'s own steps:
///
/// 1. **Light only adds.** Nothing `P` does negates. On a growing input,
///    nothing visible retracts: each output contains the one before, so a
///    lums-like count over the outputs never falls.
/// 2. **Light never twists.** `P` checks no declaration, so it never
///    produces `undefined` (a horizontal twist, an odd `not` loop) or a
///    paradox (a vertical twist, a violated declaration).
/// 3. **Darkness is received, never made.** Holes, unreadable input and
///    budget arrive from outside. A stable Prism's own result never carries
///    darkness it made: any loss or failure in its output was in its input,
///    and passes through. Where the result type allows it, the dark arm is
///    uninhabited (`Infallible`, as
///    `spectral::instrument::Instrument::kernel_dimension` already does).
///    This is not forced on existing types.
///
/// # Sound, not complete
///
/// `Stable` is **sound but not complete, by construction**. Every Prism
/// marked `Stable` keeps the three promises. A Prism that isn't marked is
/// **unknown**, not twisted: an underivable `not`, or a declaration that can
/// never be violated, is semantically stable and still unmarked until
/// someone writes the proof. "Unstable" names that absence. There is no
/// `Unstable` type to implement.
///
/// So stability is never inferred from shape. There is no blanket impl, and
/// a Prism built from a user-supplied function (a `Lens` view, a `Fold`)
/// isn't marked, because the function may negate.
///
/// # Placement
///
/// `Stable` is a law, earned by a proof: the impl is the proof obligation,
/// and every impl in prism states its argument in one line beside it.
/// Everything else is unknown.
///
/// # Composition
///
/// Stable composes with stable to stable:
/// - `&P` is stable wherever `P` is;
/// - [`Compose<A, B>`] is stable when `A` and `B` both are, because its seam
///   adds nothing (see its docs).
///
/// A combinator that can twist by itself, for example one that negates or
/// settles under a ruler, must never be marked, even from stable parts.
///
/// The compile-time laws (composing with an unmarked Prism, and a negating
/// combinator over stable parts, each with a compiling control) are the
/// doctests of the private `laws` module below, written over prism's own
/// marked `IdentityPrism`. They need the `bundle`
/// feature, as `IdentityPrism` does.
pub trait Stable: Prism {}

// `&P` only forwards to `P`: it adds no step of its own.
impl<P: Stable> Stable for &P {}

/// Sequential composition: run `A`, then run `B` on what `A` settled.
///
/// The stages of `Compose<A, B>` are:
/// - **focus** runs all of `A` (`X → Y`);
/// - **project** is the seam, passing `Y` through unchanged;
/// - **settle** runs all of `B` (`Y → V`).
///
/// Each part runs whole, so its own stages keep their sources. The loss is
/// the combination of both parts' losses; a failure in `A` carries through
/// `B` untouched, and so does received darkness.
///
/// `Compose<A, B>` is [`Stable`] when `A` and `B` both are: the seam
/// negates nothing, checks nothing, and adds no loss or error.
pub struct Compose<A, B>(pub A, pub B);

impl<A, B, X, W, Y, U, V, E, L> Prism for Compose<A, B>
where
    X: Clone,
    Y: Clone,
    L: Loss,
    A: Prism<Input = Optic<(), X, E, L>, Refracted = Optic<W, Y, E, L>>,
    B: Prism<Input = Optic<(), Y, E, L>, Refracted = Optic<U, V, E, L>>,
{
    type Input = Optic<(), X, E, L>;
    type Focused = Optic<X, Y, E, L>;
    type Projected = Optic<Y, Y, E, L>;
    type Refracted = Optic<Y, V, E, L>;

    fn focus(&self, beam: Self::Input) -> Self::Focused {
        let source = beam.value().cloned();
        Optic::from_parts(source, apply(&self.0, beam).into_focus())
    }

    fn project(&self, beam: Self::Focused) -> Self::Projected {
        beam.smap(|y| Imperfect::Success(y.clone()))
    }

    fn settle(&self, beam: Self::Projected) -> Self::Refracted {
        let source = beam.value().cloned();
        let seed = beam.into_focus();
        let lit = (!seed.is_err()).then_some(());
        let out = apply(&self.1, Optic::from_parts(lit, seed)).into_focus();
        Optic::from_parts(source, out)
    }
}

// The seam only passes `Y` through (`smap` with `Success`), and each part
// runs unchanged: no negation, no check, no loss of its own.
impl<A, B, X, W, Y, U, V, E, L> Stable for Compose<A, B>
where
    X: Clone,
    Y: Clone,
    L: Loss,
    A: Stable<Input = Optic<(), X, E, L>, Refracted = Optic<W, Y, E, L>>,
    B: Stable<Input = Optic<(), Y, E, L>, Refracted = Optic<U, V, E, L>>,
{
}

/// The compile-time laws of [`Stable`], over prism's own marked Prism.
///
/// They use [`crate::IdentityPrism`] rather than a Prism marked here, so
/// that the mutation "every Prism is `Stable`" (a blanket impl) makes each
/// `compile_fail` below compile, instead of failing for an unrelated reason.
///
/// Composing a stable Prism with one that isn't marked gives a Prism that
/// isn't marked:
///
/// ```compile_fail,E0277
/// # use prismqueer::{Beam, Compose, IdentityPrism, Optic, Prism, Stable};
/// # use std::collections::BTreeSet;
/// # type Set = BTreeSet<u32>;
/// /// Passes its set through, like `IdentityPrism`, but nobody wrote the
/// /// proof.
/// struct Unproved;
/// # impl Prism for Unproved {
/// #     type Input = Optic<(), Set>;
/// #     type Focused = Optic<Set, Set>;
/// #     type Projected = Optic<Set, Set>;
/// #     type Refracted = Optic<Set, Set>;
/// #     fn focus(&self, b: Self::Input) -> Self::Focused { let s = b.value().cloned().unwrap(); b.next(s) }
/// #     fn project(&self, b: Self::Focused) -> Self::Projected { let s = b.value().cloned().unwrap(); b.next(s) }
/// #     fn settle(&self, b: Self::Projected) -> Self::Refracted { let s = b.value().cloned().unwrap(); b.next(s) }
/// # }
///
/// fn stable<P: Stable>(_: &P) {}
/// stable(&Compose(IdentityPrism::<Set>::new(), Unproved));
/// ```
///
/// The control compiles: stable parts, composed and nested, and a reference.
///
/// ```
/// # use prismqueer::{Compose, IdentityPrism, Stable};
/// # use std::collections::BTreeSet;
/// # type Set = BTreeSet<u32>;
/// let id = IdentityPrism::<Set>::new;
/// fn stable<P: Stable>(_: &P) {}
/// stable(&Compose(id(), id()));
/// let inner = id();
/// stable(&Compose(Compose(id(), id()), &inner));
/// ```
///
/// A combinator that negates is not stable, even from stable parts. prism
/// ships no such combinator, so this one is written here: `Not<P>`
/// complements `P`'s set. It is a Prism, and nothing makes it `Stable`:
///
/// ```compile_fail,E0277
/// # use prismqueer::{apply, Beam, IdentityPrism, Optic, Prism, Stable};
/// # use std::collections::BTreeSet;
/// # type Set = BTreeSet<u32>;
/// /// The complement of `P`'s output within 0..16.
/// struct Not<P>(P);
///
/// impl<P> Prism for Not<P>
/// where
///     P: Prism<Input = Optic<(), Set>, Refracted = Optic<Set, Set>>,
/// {
///     type Input = Optic<(), Set>;
///     type Focused = Optic<Set, Set>;
///     type Projected = Optic<Set, Set>;
///     type Refracted = Optic<Set, Set>;
///     fn focus(&self, b: Self::Input) -> Self::Focused {
///         let s = b.value().cloned().unwrap();
///         b.next(s)
///     }
///     fn project(&self, b: Self::Focused) -> Self::Projected {
///         let lit = apply(&self.0, Optic::ok((), b.value().cloned().unwrap()));
///         let out: Set = (0..16).filter(|n| !lit.value().unwrap().contains(n)).collect();
///         b.next(out)
///     }
///     fn settle(&self, b: Self::Projected) -> Self::Refracted {
///         let s = b.value().cloned().unwrap();
///         b.next(s)
///     }
/// }
///
/// fn stable<P: Stable>(_: &P) {}
/// stable(&Not(IdentityPrism::<Set>::new()));
/// ```
///
/// The control compiles: the same `Not` is still a Prism, it negates, and
/// its part alone is stable.
///
/// ```
/// # use prismqueer::{apply, Beam, IdentityPrism, Optic, Prism, Stable};
/// # use std::collections::BTreeSet;
/// # type Set = BTreeSet<u32>;
/// # struct Not<P>(P);
/// # impl<P> Prism for Not<P>
/// # where
/// #     P: Prism<Input = Optic<(), Set>, Refracted = Optic<Set, Set>>,
/// # {
/// #     type Input = Optic<(), Set>;
/// #     type Focused = Optic<Set, Set>;
/// #     type Projected = Optic<Set, Set>;
/// #     type Refracted = Optic<Set, Set>;
/// #     fn focus(&self, b: Self::Input) -> Self::Focused { let s = b.value().cloned().unwrap(); b.next(s) }
/// #     fn project(&self, b: Self::Focused) -> Self::Projected {
/// #         let lit = apply(&self.0, Optic::ok((), b.value().cloned().unwrap()));
/// #         let out: Set = (0..16).filter(|n| !lit.value().unwrap().contains(n)).collect();
/// #         b.next(out)
/// #     }
/// #     fn settle(&self, b: Self::Projected) -> Self::Refracted { let s = b.value().cloned().unwrap(); b.next(s) }
/// # }
/// fn prism<P: Prism>(_: &P) {}
/// fn stable<P: Stable>(_: &P) {}
/// let id = IdentityPrism::<Set>::new();
/// prism(&Not(IdentityPrism::<Set>::new()));
/// stable(&id);
/// let all: Set = (0..16).collect();
/// let none = apply(&Not(id), Optic::ok((), all)).value().cloned().unwrap();
/// assert!(none.is_empty());
/// ```
#[cfg(feature = "bundle")]
mod laws {}
