//! Prism — focus | project | refract.
//!
//! A Beam is a semifunctor. A Prism is the monoid lifted into it.
//! Three operations. Three dimensions of space.

pub mod beam;
pub mod trace;

pub mod connection;
pub mod content;
pub mod metal;
pub mod oid;
pub mod precision;
pub mod spectral_oid;

#[cfg(feature = "optics")]
pub mod optics;

pub use beam::{Beam, Operation, PureBeam};
pub use imperfect::{Imperfect, Loss, ShannonLoss};
pub use trace::{Op, Step, StepOutput, Trace, Traced};

pub use connection::{Connection, ScalarConnection};
pub use content::ContentAddressed;
pub use oid::Oid;
pub use precision::{Precision, Pressure};
pub use spectral_oid::SpectralOid;

// ---------------------------------------------------------------------------
// Prism trait
// ---------------------------------------------------------------------------

/// Three optic operations. A prism is the monoid lifted into the Beam
/// semifunctor. All beam types are associated types, not parameters.
pub trait Prism {
    type Input:     Beam;
    type Focused:   Beam<In = <Self::Input     as Beam>::Out>;
    type Projected: Beam<In = <Self::Focused   as Beam>::Out>;
    type Refracted: Beam<In = <Self::Projected as Beam>::Out>;

    fn focus(&self, beam: Self::Input) -> Self::Focused;
    fn project(&self, beam: Self::Focused) -> Self::Projected;
    fn refract(&self, beam: Self::Projected) -> Self::Refracted;
}

/// Blanket impl: `&P` is a Prism wherever `P` is.
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
    fn op(&self) -> Op { Op::Focus }
    fn apply(self, beam: P::Input) -> P::Focused { self.0.focus(beam) }
}

impl<P: Prism> Operation<P::Focused> for Project<P> {
    type Output = P::Projected;
    fn op(&self) -> Op { Op::Project }
    fn apply(self, beam: P::Focused) -> P::Projected { self.0.project(beam) }
}

impl<P: Prism> Operation<P::Projected> for Refract<P> {
    type Output = P::Refracted;
    fn op(&self) -> Op { Op::Refract }
    fn apply(self, beam: P::Projected) -> P::Refracted { self.0.refract(beam) }
}

#[cfg(test)]
mod tests {
    use super::*;
    use imperfect::{Imperfect, ShannonLoss};

    /// A prism that counts characters.
    /// focus: String → Vec<char>, project: Vec<char> → usize, refract: usize → String
    struct CountPrism;

    impl Prism for CountPrism {
        type Input     = PureBeam<(), String>;
        type Focused   = PureBeam<String, Vec<char>>;
        type Projected = PureBeam<Vec<char>, usize>;
        type Refracted = PureBeam<usize, String>;

        fn focus(&self, beam: Self::Input) -> Self::Focused {
            let chars: Vec<char> = beam.result()
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
