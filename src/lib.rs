//! Prism — focus | project | split | zoom | refract.
//!
//! A `Prism` is fully self-contained: it describes its own input and output
//! types through associated types, not type parameters. The prism IS the
//! complete description of the transformation. This is the only reasonable
//! design for an autopoietic system.
//!
//! `Beam` is a trait. Two implementations:
//! - `PureBeam<In, Out, E>` — prod. No trace overhead.
//! - `TraceBeam<In, Out, E>` — debug. Full execution trace. (forthcoming)
//!
//! `Operation` is a trait. Five implementations — one per pipeline stage.
//! Each carries the prism and input beam. `apply()` is the single execution
//! point. Tracing wraps `apply`.
//!
//! Three methods MUST be implemented on `Prism`: `focus`, `project`, `refract`.
//! `split` and `zoom` have default implementations via their closure arguments.

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

pub use beam::{Beam, PureBeam};
pub use connection::{Connection, ScalarConnection};
pub use content::ContentAddressed;
pub use loss::ShannonLoss;
pub use oid::Oid;
pub use precision::{Precision, Pressure};
pub use spectral_oid::SpectralOid;
pub use trace::{Step, StepOutput, Trace, Traced};

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
///
/// # Beam choice
///
/// Prisms that want to work with multiple beam types (PureBeam vs TraceBeam)
/// carry a phantom type parameter:
///
/// ```rust,ignore
/// struct MyPrism<B>(PhantomData<B>);
/// impl<B: Beam<Out = String>> Prism for MyPrism<B> {
///     type Input = B;
///     type Focused = B::Next<Vec<Token>>;
///     // ...
/// }
/// ```
///
/// The beam choice is at the call site — `MyPrism::<PureBeam<_,_>>::new()`
/// or `MyPrism::<TraceBeam<_,_>>::new()`.
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
        let result = {
            let out = match beam.result() {
                Ok(v)  => v,
                Err(_) => panic!("split called on Dark beam"),
            };
            f(out)
        };
        match result {
            Ok(parts) => beam.advance_err(parts),
            Err(e)    => beam.fail_err(e),
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
        let result = {
            let out = match beam.result() {
                Ok(v)  => v,
                Err(_) => panic!("zoom called on Dark beam"),
            };
            f(out)
        };
        match result {
            Ok(z)  => beam.advance_err(z),
            Err(e) => beam.fail_err(e),
        }
    }

    fn refract(&self, beam: Self::Projected) -> Self::Refracted;
}

/// Run a prism end-to-end: focus → project → refract.
pub fn apply<P: Prism>(prism: &P, beam: P::Input) -> P::Refracted {
    let focused   = prism.focus(beam);
    let projected = prism.project(focused);
    prism.refract(projected)
}

// ---------------------------------------------------------------------------
// Operation trait + five structs
// ---------------------------------------------------------------------------

/// An operation is a self-contained unit of pipeline work.
/// It carries the prism and the input beam. `apply` executes it.
///
/// Tracing wraps `apply` — one recording point per operation, no
/// instrumentation inside prism methods.
pub trait Operation {
    type Output: Beam;
    fn apply(self) -> Self::Output;
}

/// focus: Input → Focused.
pub struct Focus<P: Prism>(pub P, pub P::Input);

/// project: Focused → Projected.
pub struct Project<P: Prism>(pub P, pub P::Focused);

/// split: Projected → NextWithError<Vec<S>, SE> via closure.
pub struct Split<P: Prism, F>(pub P, pub P::Projected, pub F);

/// zoom: Projected → NextWithError<Z, ZE> via closure.
pub struct Zoom<P: Prism, F>(pub P, pub P::Projected, pub F);

/// refract: Projected → Refracted.
pub struct Refract<P: Prism>(pub P, pub P::Projected);

impl<P: Prism> Operation for Focus<P> {
    type Output = P::Focused;
    fn apply(self) -> P::Focused {
        self.0.focus(self.1)
    }
}

impl<P: Prism> Operation for Project<P> {
    type Output = P::Projected;
    fn apply(self) -> P::Projected {
        self.0.project(self.1)
    }
}

impl<P, F, S, SE> Operation for Split<P, F>
where
    P: Prism,
    F: Fn(&<P::Projected as Beam>::Out) -> Result<Vec<S>, SE>,
{
    type Output = <P::Projected as Beam>::NextWithError<Vec<S>, SE>;
    fn apply(self) -> Self::Output {
        self.0.split(self.1, &self.2)
    }
}

impl<P, F, Z, ZE> Operation for Zoom<P, F>
where
    P: Prism,
    F: Fn(&<P::Projected as Beam>::Out) -> Result<Z, ZE>,
{
    type Output = <P::Projected as Beam>::NextWithError<Z, ZE>;
    fn apply(self) -> Self::Output {
        self.0.zoom(self.1, &self.2)
    }
}

impl<P: Prism> Operation for Refract<P> {
    type Output = P::Refracted;
    fn apply(self) -> P::Refracted {
        self.0.refract(self.1)
    }
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
            beam.advance(chars)
        }

        fn project(&self, beam: Self::Focused) -> Self::Projected {
            let n = beam.result().expect("project: Dark beam").len();
            beam.advance(n)
        }

        fn refract(&self, beam: Self::Projected) -> Self::Refracted {
            beam.advance(CountPrism)
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

    #[test]
    fn apply_end_to_end() {
        let r: PureBeam<usize, CountPrism> = apply(&CountPrism, seed("hi"));
        assert!(r.is_ok());
    }

    // --- Operation tests ---

    #[test]
    fn operation_focus() {
        let b: PureBeam<String, Vec<char>> =
            Focus(CountPrism, seed("hello")).apply();
        assert_eq!(b.result(), Ok(&vec!['h', 'e', 'l', 'l', 'o']));
    }

    #[test]
    fn operation_project() {
        let focused = CountPrism.focus(seed("hello"));
        let p: PureBeam<Vec<char>, usize> =
            Project(CountPrism, focused).apply();
        assert_eq!(p.result(), Ok(&5));
    }

    #[test]
    fn operation_split() {
        let projected = CountPrism.project(CountPrism.focus(seed("abc")));
        let r: PureBeam<usize, Vec<u32>, Infallible> =
            Split(CountPrism, projected, |&n: &usize| {
                Ok::<Vec<u32>, Infallible>((0..n as u32).collect())
            }).apply();
        assert_eq!(r.result(), Ok(&vec![0, 1, 2]));
    }

    #[test]
    fn operation_zoom() {
        let projected = CountPrism.project(CountPrism.focus(seed("hello")));
        let r: PureBeam<usize, usize, Infallible> =
            Zoom(CountPrism, projected, |&n: &usize| {
                Ok::<usize, Infallible>(n * 2)
            }).apply();
        assert_eq!(r.result(), Ok(&10));
    }

    #[test]
    fn operation_zoom_dark() {
        let projected = CountPrism.project(CountPrism.focus(seed("hello")));
        let r: PureBeam<usize, usize, &str> =
            Zoom(CountPrism, projected, |_: &usize| Err("nope")).apply();
        assert!(r.is_dark());
    }

    #[test]
    fn operation_refract() {
        let projected = CountPrism.project(CountPrism.focus(seed("hi")));
        let r: PureBeam<usize, CountPrism> = Refract(CountPrism, projected).apply();
        assert!(r.is_ok());
    }
}
