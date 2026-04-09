//! Prism — focus | project | split | zoom | refract.
//!
//! A `Prism` is fully self-contained: it describes its own input and output
//! types through associated types, not type parameters. The prism IS the
//! complete description of the transformation. This is the only reasonable
//! design for an autopoietic system.
//!
//! Beam-generic prisms carry a phantom type parameter so the beam type is
//! baked in at construction rather than at every call site:
//!
//! ```rust,ignore
//! struct MyPrism<B>(PhantomData<B>);
//! impl<B: Beam<Out = String>> Prism for MyPrism<B> {
//!     type Input = B;
//!     type Focused = B::Next<Vec<Token>>;
//!     // ...
//! }
//! // Beam type chosen once, at construction:
//! MyPrism::<PureBeam<_, _>>::default()
//! ```
//!
//! `Operation` is a trait in `beam`. Five implementations — one per pipeline
//! stage. Each wraps the prism (and a closure for split/zoom). The beam is
//! NOT stored in the operation — it arrives via `apply`.
//!
//! The pipeline DSL:
//!
//! ```rust,ignore
//! beam.apply(Focus(&prism))
//!     .apply(Project(&prism))
//!     .apply(Refract(&prism))
//! ```

pub mod beam;
pub mod connection;
pub mod content;
pub mod loss;
pub mod metal;
pub mod oid;
pub mod precision;
pub mod spectral_oid;
pub mod trace;

#[cfg(feature = "optics")]
pub mod optics;

pub use beam::{Beam, Luminosity, Operation, PureBeam};
pub use connection::{Connection, ScalarConnection};
pub use content::ContentAddressed;
pub use loss::ShannonLoss;
pub use oid::Oid;
pub use precision::{Precision, Pressure};
pub use spectral_oid::SpectralOid;
pub use trace::{Op, Step, StepOutput, Trace, Traced};

// ---------------------------------------------------------------------------
// Prism trait
// ---------------------------------------------------------------------------

/// Five optic operations. A prism is fully self-contained: all beam types
/// are associated types, not parameters.
///
/// # Type chain
///
/// The compiler enforces the chain: each stage's output type becomes the
/// next stage's input type. Invalid pipelines are rejected at compile time.
pub trait Prism {
    type Input:     Beam;
    type Focused:   Beam<In = <Self::Input     as Beam>::Out>;
    type Projected: Beam<In = <Self::Focused   as Beam>::Out>;
    type Refracted: Beam<In = <Self::Projected as Beam>::Out>;

    fn focus(&self, beam: Self::Input) -> Self::Focused;
    fn project(&self, beam: Self::Focused) -> Self::Projected;

    /// Split: apply `f` to the projected value and fan out.
    /// Default impl delegates to the closure. Override for custom behaviour.
    /// Contract: `beam` must be non-Dark. Panics otherwise.
    fn split<S, SE>(
        &self,
        beam: Self::Projected,
        f: &dyn Fn(&<Self::Projected as Beam>::Out) -> Result<Vec<S>, SE>,
    ) -> <Self::Projected as Beam>::NextWithError<Vec<S>, SE> {
        let result = match beam.result() {
            Ok(v)  => f(v),
            Err(_) => panic!("split called on Dark beam"),
        };
        match result {
            Ok(parts) => beam.advance(Luminosity::Radiant(parts)),
            Err(e)    => beam.advance(Luminosity::Dark(e)),
        }
    }

    /// Zoom: apply `f` to the projected value.
    /// Default impl delegates to the closure. Override for custom behaviour.
    /// Contract: `beam` must be non-Dark. Panics otherwise.
    fn zoom<Z, ZE>(
        &self,
        beam: Self::Projected,
        f: &dyn Fn(&<Self::Projected as Beam>::Out) -> Result<Z, ZE>,
    ) -> <Self::Projected as Beam>::NextWithError<Z, ZE> {
        let result = match beam.result() {
            Ok(v)  => f(v),
            Err(_) => panic!("zoom called on Dark beam"),
        };
        match result {
            Ok(z)  => beam.advance(Luminosity::Radiant(z)),
            Err(e) => beam.advance(Luminosity::Dark(e)),
        }
    }

    fn refract(&self, beam: Self::Projected) -> Self::Refracted;
}

/// Blanket impl so `&P` is a prism wherever `P` is.
/// Enables `beam.apply(Focus(&prism))` without consuming the prism.
impl<P: Prism> Prism for &P {
    type Input     = P::Input;
    type Focused   = P::Focused;
    type Projected = P::Projected;
    type Refracted = P::Refracted;

    fn focus(&self, beam: P::Input) -> P::Focused         { P::focus(self, beam) }
    fn project(&self, beam: P::Focused) -> P::Projected   { P::project(self, beam) }
    fn refract(&self, beam: P::Projected) -> P::Refracted { P::refract(self, beam) }
}

/// Run a prism end-to-end: focus → project → refract.
pub fn apply<P: Prism>(prism: &P, beam: P::Input) -> P::Refracted {
    beam.apply(Focus(prism))
        .apply(Project(prism))
        .apply(Refract(prism))
}

// ---------------------------------------------------------------------------
// Operation structs — five pipeline stages
// ---------------------------------------------------------------------------
//
// Each wraps the prism (and a closure for split/zoom).
// The beam is NOT stored here. It arrives via Operation::apply or Beam::apply.

/// focus: Input → Focused.
pub struct Focus<P>(pub P);

/// project: Focused → Projected.
pub struct Project<P>(pub P);

/// split: Projected → NextWithError<Vec<S>, SE> via closure.
pub struct Split<P, F>(pub P, pub F);

/// zoom: Projected → NextWithError<Z, ZE> via closure.
pub struct Zoom<P, F>(pub P, pub F);

/// refract: Projected → Refracted.
pub struct Refract<P>(pub P);

impl<P: Prism> Operation<P::Input> for Focus<P> {
    type Output = P::Focused;
    fn op(&self) -> Op { Op::Focus }
    fn apply(self, beam: P::Input) -> P::Focused { self.0.focus(beam) }
}

impl<P: Prism> Operation<P::Focused> for Project<P> {
    type Output = P::Projected;
    fn op(&self) -> Op { Op::Project }
    fn apply(self, beam: P::Focused) -> P::Projected { self.0.project(beam) }
}

impl<P, F, S, SE> Operation<P::Projected> for Split<P, F>
where
    P: Prism,
    F: Fn(&<P::Projected as Beam>::Out) -> Result<Vec<S>, SE>,
{
    type Output = <P::Projected as Beam>::NextWithError<Vec<S>, SE>;
    fn op(&self) -> Op { Op::Split }
    fn apply(self, beam: P::Projected) -> Self::Output { self.0.split(beam, &self.1) }
}

impl<P, F, Z, ZE> Operation<P::Projected> for Zoom<P, F>
where
    P: Prism,
    F: Fn(&<P::Projected as Beam>::Out) -> Result<Z, ZE>,
{
    type Output = <P::Projected as Beam>::NextWithError<Z, ZE>;
    fn op(&self) -> Op { Op::Zoom }
    fn apply(self, beam: P::Projected) -> Self::Output { self.0.zoom(beam, &self.1) }
}

impl<P: Prism> Operation<P::Projected> for Refract<P> {
    type Output = P::Refracted;
    fn op(&self) -> Op { Op::Refract }
    fn apply(self, beam: P::Projected) -> P::Refracted { self.0.refract(beam) }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use std::convert::Infallible;
    use super::*;

    /// A prism that counts characters. Fixed to PureBeam for simplicity.
    ///
    /// For beam-generic prisms, carry a phantom: `struct CountPrism<B>(PhantomData<B>)`.
    ///
    /// focus:   String    → Vec<char>
    /// project: Vec<char> → usize
    /// refract: usize     → CountPrism
    struct CountPrism;

    impl Prism for CountPrism {
        type Input     = PureBeam<(), String>;
        type Focused   = PureBeam<String, Vec<char>>;
        type Projected = PureBeam<Vec<char>, usize>;
        type Refracted = PureBeam<usize, CountPrism>;

        fn focus(&self, beam: Self::Input) -> Self::Focused {
            let chars: Vec<char> = beam.result()
                .expect("focus: Dark beam")
                .chars()
                .collect();
            beam.next(chars)
        }

        fn project(&self, beam: Self::Focused) -> Self::Projected {
            let n = beam.result().expect("project: Dark beam").len();
            beam.next(n)
        }

        fn refract(&self, beam: Self::Projected) -> Self::Refracted {
            beam.next(CountPrism)
        }
    }

    fn seed(s: &str) -> PureBeam<(), String> {
        PureBeam::radiant((), s.to_string())
    }

    // --- Prism method tests ---

    #[test]
    fn focus_yields_chars() {
        let b: PureBeam<String, Vec<char>> = CountPrism.focus(seed("hello"));
        assert_eq!(b.result(), Ok(&vec!['h', 'e', 'l', 'l', 'o']));
        assert_eq!(b.input(), &"hello".to_string());
    }

    #[test]
    fn project_yields_count() {
        let f = CountPrism.focus(seed("hello"));
        let p: PureBeam<Vec<char>, usize> = CountPrism.project(f);
        assert_eq!(p.result(), Ok(&5));
    }

    #[test]
    fn refract_produces_crystal() {
        let f = CountPrism.focus(seed("hi"));
        let p = CountPrism.project(f);
        let r: PureBeam<usize, CountPrism> = CountPrism.refract(p);
        let inner = r.result().unwrap();
        let inner_f: PureBeam<String, Vec<char>> = inner.focus(seed("abc"));
        assert_eq!(inner_f.result(), Ok(&vec!['a', 'b', 'c']));
    }

    // --- DSL pipeline tests ---

    #[test]
    fn apply_end_to_end() {
        let r: PureBeam<usize, CountPrism> = apply(&CountPrism, seed("hi"));
        assert!(r.is_light());
    }

    #[test]
    fn apply_dsl() {
        let r: PureBeam<usize, CountPrism> = seed("hi")
            .apply(Focus(&CountPrism))
            .apply(Project(&CountPrism))
            .apply(Refract(&CountPrism));
        assert!(r.is_light());
    }

    // --- Operation tests ---

    #[test]
    fn operation_focus() {
        let b: PureBeam<String, Vec<char>> =
            Focus(&CountPrism).apply(seed("hello"));
        assert_eq!(b.result(), Ok(&vec!['h', 'e', 'l', 'l', 'o']));
    }

    #[test]
    fn operation_project() {
        let focused = CountPrism.focus(seed("hello"));
        let p: PureBeam<Vec<char>, usize> =
            Project(&CountPrism).apply(focused);
        assert_eq!(p.result(), Ok(&5));
    }

    #[test]
    fn operation_split() {
        let projected = seed("abc")
            .apply(Focus(&CountPrism))
            .apply(Project(&CountPrism));
        let r: PureBeam<usize, Vec<u32>, Infallible> =
            projected.apply(Split(&CountPrism, |&n: &usize| {
                Ok::<Vec<u32>, Infallible>((0..n as u32).collect())
            }));
        assert_eq!(r.result(), Ok(&vec![0, 1, 2]));
    }

    #[test]
    fn operation_zoom() {
        let projected = seed("hello")
            .apply(Focus(&CountPrism))
            .apply(Project(&CountPrism));
        let r: PureBeam<usize, usize, Infallible> =
            projected.apply(Zoom(&CountPrism, |&n: &usize| {
                Ok::<usize, Infallible>(n * 2)
            }));
        assert_eq!(r.result(), Ok(&10));
    }

    #[test]
    fn operation_zoom_dark() {
        let projected = seed("hello")
            .apply(Focus(&CountPrism))
            .apply(Project(&CountPrism));
        let r: PureBeam<usize, usize, &str> =
            projected.apply(Zoom(&CountPrism, |_: &usize| Err("nope")));
        assert!(r.is_dark());
    }

    #[test]
    fn operation_refract() {
        let projected = seed("hi")
            .apply(Focus(&CountPrism))
            .apply(Project(&CountPrism));
        let r: PureBeam<usize, CountPrism> =
            Refract(&CountPrism).apply(projected);
        assert!(r.is_light());
    }

    // --- op() labels ---

    #[test]
    fn operation_op_labels() {
        assert_eq!(Focus(&CountPrism).op(),   Op::Focus);
        assert_eq!(Project(&CountPrism).op(), Op::Project);
        assert_eq!(Refract(&CountPrism).op(), Op::Refract);
        assert_eq!(
            Split(&CountPrism, |_: &usize| Ok::<Vec<()>, ()>(vec![])).op(),
            Op::Split
        );
        assert_eq!(
            Zoom(&CountPrism, |_: &usize| Ok::<(), ()>(())).op(),
            Op::Zoom
        );
    }
}
