use prism_core::{Beam, Focus, Prism, Project, PureBeam, Refract};
use std::convert::Infallible;
use prism_core::ScalarLoss;
use terni::Imperfect;

/// A prism that tokenizes → counts → formats.
struct TokenPrism;

impl Prism for TokenPrism {
    type Input = PureBeam<(), String>;
    type Focused = PureBeam<String, Vec<String>>;
    type Projected = PureBeam<Vec<String>, usize>;
    type Refracted = PureBeam<usize, String>;

    fn focus(&self, beam: Self::Input) -> Self::Focused {
        let tokens: Vec<String> = beam
            .result()
            .ok()
            .expect("focus: Err beam")
            .split_whitespace()
            .map(String::from)
            .collect();
        beam.next(tokens)
    }

    fn project(&self, beam: Self::Focused) -> Self::Projected {
        let count = beam.result().ok().expect("project: Err beam").len();
        beam.next(count)
    }

    fn refract(&self, beam: Self::Projected) -> Self::Refracted {
        let n = *beam.result().ok().expect("refract: Err beam");
        beam.next(format!("{} tokens", n))
    }
}

#[test]
fn full_pipeline_dsl() {
    let result = PureBeam::ok((), "hello world foo".to_string())
        .apply(Focus(&TokenPrism))
        .apply(Project(&TokenPrism))
        .apply(Refract(&TokenPrism));

    assert!(result.is_ok());
    assert_eq!(result.result().ok(), Some(&"3 tokens".to_string()));
}

#[test]
fn full_pipeline_apply_fn() {
    let result = prism_core::apply(&TokenPrism, PureBeam::ok((), "a b c d".to_string()));
    assert_eq!(result.result().ok(), Some(&"4 tokens".to_string()));
}

#[test]
fn smap_as_zoom_in_pipeline() {
    let projected = PureBeam::ok((), "hello world".to_string())
        .apply(Focus(&TokenPrism))
        .apply(Project(&TokenPrism));

    let doubled = projected.smap(|&n| Imperfect::Success(n * 2));
    assert_eq!(doubled.result().ok(), Some(&4));
}

#[test]
fn smap_as_split_in_pipeline() {
    let focused = PureBeam::ok((), "hello world".to_string()).apply(Focus(&TokenPrism));

    let chars: PureBeam<Vec<String>, Vec<char>> = focused.smap(|tokens| {
        let all_chars: Vec<char> = tokens.iter().flat_map(|t| t.chars()).collect();
        Imperfect::Success(all_chars)
    });
    assert_eq!(
        chars.result().ok(),
        Some(&vec!['h', 'e', 'l', 'l', 'o', 'w', 'o', 'r', 'l', 'd'])
    );
}

#[test]
fn partial_beam_propagates_loss() {
    let b: PureBeam<(), String, Infallible, ScalarLoss> =
        PureBeam::partial((), "hello world".to_string(), ScalarLoss::new(0.5));

    let focused = TokenPrism.focus(b);
    assert!(focused.is_partial());

    let projected = TokenPrism.project(focused);
    assert!(projected.is_partial());
}

#[test]
fn imperfect_result_interop() {
    let ok_result: Result<u32, String> = Ok(42);
    let imp: Imperfect<u32, String, ScalarLoss> = ok_result.into();
    assert!(imp.is_ok());

    let back: Result<u32, String> = imp.into();
    assert_eq!(back, Ok(42));
}

#[test]
fn scalar_loss_methods_covered_in_integration() {
    use terni::Loss;

    let zero = ScalarLoss::zero();
    assert!(zero.is_zero());
    assert_eq!(zero.as_f64(), 0.0);

    let total = ScalarLoss::total();
    assert!(!total.is_zero());
    assert!(total.as_f64().is_infinite());

    let a = ScalarLoss::new(1.0);
    let b = ScalarLoss::new(2.0);
    let combined = a.combine(b);
    assert_eq!(combined.as_f64(), 3.0);

    let d = ScalarLoss::default();
    assert!(d.is_zero());
}
