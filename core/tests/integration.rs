use prism_core::{Beam, Focus, Prism, Project, PureBeam, Refract};
use std::convert::Infallible;
use terni::{Imperfect, ShannonLoss};

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
    let b: PureBeam<(), String, Infallible, ShannonLoss> =
        PureBeam::partial((), "hello world".to_string(), ShannonLoss::new(0.5));

    let focused = TokenPrism.focus(b);
    assert!(focused.is_partial());

    let projected = TokenPrism.project(focused);
    assert!(projected.is_partial());
}

#[test]
fn imperfect_result_interop() {
    let ok_result: Result<u32, String> = Ok(42);
    let imp: Imperfect<u32, String> = ok_result.into();
    assert!(imp.is_ok());

    let back: Result<u32, String> = imp.into();
    assert_eq!(back, Ok(42));
}

#[test]
fn shannon_loss_methods_covered_in_integration() {
    use terni::Loss;

    // is_lossless: delegates to is_zero
    let zero = ShannonLoss::zero();
    assert!(zero.is_lossless());

    // Loss::total via trait method
    let total = ShannonLoss::total();
    assert!(!total.is_zero());

    // Add operator
    let a = ShannonLoss::new(1.0);
    let b = ShannonLoss::new(2.0);
    let sum = a + b;
    assert_eq!(sum.as_f64(), 3.0);

    // AddAssign operator
    let mut c = ShannonLoss::new(1.0);
    c += ShannonLoss::new(0.5);
    assert_eq!(c.as_f64(), 1.5);

    // Display
    let d = ShannonLoss::new(2.0);
    assert_eq!(format!("{}", d), "2.000000 bits");

    // From<f64>
    let e: ShannonLoss = 3.14f64.into();
    assert_eq!(e.as_f64(), 3.14);
}
