//! Prism — focus | project | refract.
//!
//! A [`Beam`] carries three things through a pipeline: a value, the input
//! that produced it, and the accumulated loss ([`Imperfect`]). A [`Prism`]
//! defines three operations over beams:
//!
//! - **focus** — select what matters from the input.
//! - **project** — transform the focused value (precision cut, eigenvalue
//!   threshold, the lossy step where information may not survive).
//! - **refract** — produce the output from what survived projection.
//!
//! ```ignore
//! let result = seed("hello")
//!     .apply(Focus(&my_prism))
//!     .apply(Project(&my_prism))
//!     .apply(Refract(&my_prism));
//! ```
//!
//! Algebraically: a Beam is a **semifunctor** — you can map over the carried
//! value (`smap`), but the identity law may not hold because Failure beams
//! break it (mapping over a Failure panics rather than returning the same
//! Failure). A Prism is a **monoid** lifted into that semifunctor: prisms
//! compose associatively (`focus | project | refract` chains), and an identity
//! prism exists (pass-through on all three stages). This means pipelines are
//! type-safe by construction — the compiler enforces that each stage's output
//! type matches the next stage's input type.

pub mod beam;
pub mod scalar_loss;
pub mod trace;

pub mod connection;
pub mod content;
pub mod kernel;
pub mod metal;
pub mod oid;
pub mod precision;
pub mod spectral_oid;

#[cfg(feature = "optics")]
pub mod optics;

#[cfg(feature = "bundle")]
pub mod bundle;

#[cfg(feature = "lapack")]
pub mod ffi;

#[cfg(feature = "bundle")]
pub use bundle::{Bundle, Closure, Connection, Fiber, Gauge, Transport};

pub use beam::{Beam, Operation, PureBeam};
pub use scalar_loss::ScalarLoss;
pub use terni::{Imperfect, Loss};
pub use trace::{Op, Step, StepOutput, Trace, Traced};

pub use connection::{Carrier, ScalarConnection};
pub use content::ContentAddressed;
pub use kernel::{Decomposition, KernelSpec};
pub use oid::Oid;
pub use precision::{Precision, Pressure};
pub use spectral_oid::SpectralOid;

// ---------------------------------------------------------------------------
// Prism trait
// ---------------------------------------------------------------------------

/// Three optic operations over beams: focus, project, refract.
///
/// The associated types form a chain: `Input` feeds `focus`, whose output
/// beam (`Focused`) feeds `project`, whose output beam (`Projected`) feeds
/// `refract`, producing `Refracted`. The chain is enforced by the `Beam::In`
/// constraints — each stage's `In` must equal the previous stage's `Out`.
/// This makes type mismatches between pipeline stages a compile error.
pub trait Prism {
    type Input: Beam;
    type Focused: Beam<In = <Self::Input as Beam>::Out>;
    type Projected: Beam<In = <Self::Focused as Beam>::Out>;
    type Refracted: Beam<In = <Self::Projected as Beam>::Out>;

    fn focus(&self, beam: Self::Input) -> Self::Focused;
    fn project(&self, beam: Self::Focused) -> Self::Projected;
    fn refract(&self, beam: Self::Projected) -> Self::Refracted;
}

/// Blanket impl: `&P` is a Prism wherever `P` is.
impl<P: Prism> Prism for &P {
    type Input = P::Input;
    type Focused = P::Focused;
    type Projected = P::Projected;
    type Refracted = P::Refracted;

    fn focus(&self, beam: P::Input) -> P::Focused {
        P::focus(self, beam)
    }
    fn project(&self, beam: P::Focused) -> P::Projected {
        P::project(self, beam)
    }
    fn refract(&self, beam: P::Projected) -> P::Refracted {
        P::refract(self, beam)
    }
}

/// Run a prism end-to-end: focus, then project, then refract.
///
/// Equivalent to the DSL pattern `beam.apply(Focus(p)).apply(Project(p)).apply(Refract(p))`
/// but without requiring the caller to spell out each stage.
pub fn apply<P: Prism>(prism: &P, beam: P::Input) -> P::Refracted {
    beam.apply(Focus(prism))
        .apply(Project(prism))
        .apply(Refract(prism))
}

// ---------------------------------------------------------------------------
// Operation structs — three pipeline stages
// ---------------------------------------------------------------------------

/// focus: Input → Focused.
pub struct Focus<P>(pub P);

/// project: Focused → Projected.
pub struct Project<P>(pub P);

/// refract: Projected → Refracted.
pub struct Refract<P>(pub P);

impl<P: Prism> Operation<P::Input> for Focus<P> {
    type Output = P::Focused;
    fn op(&self) -> Op {
        Op::Focus
    }
    fn apply(self, beam: P::Input) -> P::Focused {
        self.0.focus(beam)
    }
}

impl<P: Prism> Operation<P::Focused> for Project<P> {
    type Output = P::Projected;
    fn op(&self) -> Op {
        Op::Project
    }
    fn apply(self, beam: P::Focused) -> P::Projected {
        self.0.project(beam)
    }
}

impl<P: Prism> Operation<P::Projected> for Refract<P> {
    type Output = P::Refracted;
    fn op(&self) -> Op {
        Op::Refract
    }
    fn apply(self, beam: P::Projected) -> P::Refracted {
        self.0.refract(beam)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use terni::Imperfect;

    /// A prism that counts characters.
    /// focus: String → Vec<char>, project: Vec<char> → usize, refract: usize → String
    struct CountPrism;

    impl Prism for CountPrism {
        type Input = PureBeam<(), String>;
        type Focused = PureBeam<String, Vec<char>>;
        type Projected = PureBeam<Vec<char>, usize>;
        type Refracted = PureBeam<usize, String>;

        fn focus(&self, beam: Self::Input) -> Self::Focused {
            let chars: Vec<char> = beam
                .result()
                .ok()
                .expect("focus: Err beam")
                .chars()
                .collect();
            beam.next(chars)
        }

        fn project(&self, beam: Self::Focused) -> Self::Projected {
            let n = beam.result().ok().expect("project: Err beam").len();
            beam.next(n)
        }

        fn refract(&self, beam: Self::Projected) -> Self::Refracted {
            let n = *beam.result().ok().expect("refract: Err beam");
            beam.next(format!("{} chars", n))
        }
    }

    fn seed(s: &str) -> PureBeam<(), String> {
        PureBeam::ok((), s.to_string())
    }

    // --- Prism method tests ---

    #[test]
    fn focus_yields_chars() {
        let b = CountPrism.focus(seed("hello"));
        assert_eq!(b.result().ok(), Some(&vec!['h', 'e', 'l', 'l', 'o']));
        assert_eq!(b.input(), &"hello".to_string());
    }

    #[test]
    fn project_yields_count() {
        let f = CountPrism.focus(seed("hello"));
        let p = CountPrism.project(f);
        assert_eq!(p.result().ok(), Some(&5));
    }

    #[test]
    fn refract_produces_string() {
        let f = CountPrism.focus(seed("hi"));
        let p = CountPrism.project(f);
        let r = CountPrism.refract(p);
        assert_eq!(r.result().ok(), Some(&"2 chars".to_string()));
    }

    // --- Operation tests ---

    #[test]
    fn operation_focus() {
        let b = Focus(&CountPrism).apply(seed("hello"));
        assert_eq!(b.result().ok(), Some(&vec!['h', 'e', 'l', 'l', 'o']));
    }

    #[test]
    fn operation_project() {
        let focused = CountPrism.focus(seed("hello"));
        let p = Project(&CountPrism).apply(focused);
        assert_eq!(p.result().ok(), Some(&5));
    }

    #[test]
    fn operation_refract() {
        let projected = seed("hi")
            .apply(Focus(&CountPrism))
            .apply(Project(&CountPrism));
        let r = Refract(&CountPrism).apply(projected);
        assert_eq!(r.result().ok(), Some(&"2 chars".to_string()));
    }

    // --- DSL pipeline ---

    #[test]
    fn dsl_pipeline() {
        let r = seed("hi")
            .apply(Focus(&CountPrism))
            .apply(Project(&CountPrism))
            .apply(Refract(&CountPrism));
        assert!(r.is_ok());
        assert_eq!(r.result().ok(), Some(&"2 chars".to_string()));
    }

    #[test]
    fn apply_fn_end_to_end() {
        let r = apply(&CountPrism, seed("hi"));
        assert!(r.is_ok());
        assert_eq!(r.result().ok(), Some(&"2 chars".to_string()));
    }

    // --- Blanket impl ---

    #[test]
    fn ref_prism_works() {
        let prism = CountPrism;
        let r = apply(&prism, seed("abc"));
        assert_eq!(r.result().ok(), Some(&"3 chars".to_string()));
    }

    // --- Op labels ---

    #[test]
    fn operation_op_labels() {
        assert_eq!(Focus(&CountPrism).op(), Op::Focus);
        assert_eq!(Project(&CountPrism).op(), Op::Project);
        assert_eq!(Refract(&CountPrism).op(), Op::Refract);
    }

    // --- smap in user space (split/zoom equivalent) ---

    #[test]
    fn smap_as_zoom() {
        let projected = seed("hello")
            .apply(Focus(&CountPrism))
            .apply(Project(&CountPrism));
        let zoomed = projected.smap(|&n| Imperfect::Success(n * 2));
        assert_eq!(zoomed.result().ok(), Some(&10));
    }

    #[test]
    fn smap_as_split() {
        let projected = seed("abc")
            .apply(Focus(&CountPrism))
            .apply(Project(&CountPrism));
        let split = projected.smap(|&n| Imperfect::Success((0..n as u32).collect::<Vec<_>>()));
        assert_eq!(split.result().ok(), Some(&vec![0, 1, 2]));
    }
}
