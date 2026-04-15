//! Typed lambda — `LambdaFn` trait with compile-time checked composition.
//!
//! `LambdaFn` is the typed surface of lambda. Each implementation declares
//! its Input, Output, Error, and Loss types. Composition via `.then()` is
//! checked at compile time: `A.then(B)` only compiles when `A::Output == B::Input`.
//!
//! Loss accumulates through the chain. Partial + Partial = Partial with
//! combined loss. Failure short-circuits.

use terni::{Imperfect, Loss};

// ---------------------------------------------------------------------------
// LambdaFn — the typed lambda trait
// ---------------------------------------------------------------------------

/// A typed lambda with compile-time checked Input/Output.
///
/// Each implementation declares what it consumes and what it produces.
/// Composition via `.then()` is a compile-time proof that the types align.
/// Loss accumulates through the chain via `Loss::combine`.
pub trait LambdaFn: Sized {
    /// The type this lambda consumes.
    type Input;
    /// The type this lambda produces.
    type Output;
    /// The error type on failure.
    type Error;
    /// The loss type — what didn't survive the transformation.
    type Loss: Loss;

    /// Reduce the input to an output, potentially with loss.
    fn reduce(self, input: Self::Input) -> Imperfect<Self::Output, Self::Error, Self::Loss>;

    /// Chain: apply self, then apply next. Compile-time type check.
    ///
    /// `A.then(B)` requires `B::Input == A::Output`. The resulting
    /// `Composed<Self, B>` has `Input = A::Input`, `Output = B::Output`.
    fn then<B>(self, next: B) -> Composed<Self, B>
    where
        B: LambdaFn<Input = Self::Output, Error = Self::Error, Loss = Self::Loss>,
    {
        Composed(self, next)
    }
}

// ---------------------------------------------------------------------------
// Composed — two typed lambdas chained with loss accumulation
// ---------------------------------------------------------------------------

/// Two typed lambdas composed sequentially. Loss accumulates.
///
/// Created by `LambdaFn::then()`. The type parameters enforce that
/// `A::Output == B::Input` at compile time.
pub struct Composed<A, B>(pub A, pub B);

impl<A, B> LambdaFn for Composed<A, B>
where
    A: LambdaFn,
    B: LambdaFn<Input = A::Output, Error = A::Error, Loss = A::Loss>,
{
    type Input = A::Input;
    type Output = B::Output;
    type Error = A::Error;
    type Loss = A::Loss;

    fn reduce(self, input: A::Input) -> Imperfect<B::Output, A::Error, A::Loss> {
        match self.0.reduce(input) {
            Imperfect::Success(mid) => self.1.reduce(mid),
            Imperfect::Partial(mid, loss) => {
                match self.1.reduce(mid) {
                    Imperfect::Success(out) => Imperfect::Partial(out, loss),
                    Imperfect::Partial(out, loss2) => Imperfect::Partial(out, loss.combine(loss2)),
                    Imperfect::Failure(err, loss2) => {
                        Imperfect::Failure(err, loss.combine(loss2))
                    }
                }
            }
            Imperfect::Failure(err, loss) => Imperfect::Failure(err, loss),
        }
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lambda::ReductionLoss;

    // -- Test types: a three-phase pipeline with typed stages --

    #[derive(Debug)]
    struct SourceText(String);
    #[derive(Debug)]
    struct ParsedAst(String);
    #[derive(Debug)]
    struct ResolvedAst(String);

    struct TestParse;
    struct TestResolve;
    struct TestFail;
    struct TestPartialParse;

    impl LambdaFn for TestParse {
        type Input = SourceText;
        type Output = ParsedAst;
        type Error = String;
        type Loss = ReductionLoss;

        fn reduce(self, input: SourceText) -> Imperfect<ParsedAst, String, ReductionLoss> {
            Imperfect::Success(ParsedAst(format!("parsed({})", input.0)))
        }
    }

    impl LambdaFn for TestResolve {
        type Input = ParsedAst;
        type Output = ResolvedAst;
        type Error = String;
        type Loss = ReductionLoss;

        fn reduce(self, input: ParsedAst) -> Imperfect<ResolvedAst, String, ReductionLoss> {
            Imperfect::Success(ResolvedAst(format!("resolved({})", input.0)))
        }
    }

    impl LambdaFn for TestFail {
        type Input = ParsedAst;
        type Output = ResolvedAst;
        type Error = String;
        type Loss = ReductionLoss;

        fn reduce(self, _input: ParsedAst) -> Imperfect<ResolvedAst, String, ReductionLoss> {
            Imperfect::Failure(
                "resolve failed".into(),
                ReductionLoss {
                    steps: 1,
                    budget_remaining: 0,
                },
            )
        }
    }

    impl LambdaFn for TestPartialParse {
        type Input = SourceText;
        type Output = ParsedAst;
        type Error = String;
        type Loss = ReductionLoss;

        fn reduce(self, input: SourceText) -> Imperfect<ParsedAst, String, ReductionLoss> {
            Imperfect::Partial(
                ParsedAst(format!("partial({})", input.0)),
                ReductionLoss {
                    steps: 2,
                    budget_remaining: 8,
                },
            )
        }
    }

    // -- Tests --

    #[test]
    fn single_phase_reduces() {
        let result = TestParse.reduce(SourceText("hello".into()));
        assert!(result.is_ok());
        assert_eq!(result.ok().unwrap().0, "parsed(hello)");
    }

    #[test]
    fn two_phases_compose() {
        let pipeline = TestParse.then(TestResolve);
        let result = pipeline.reduce(SourceText("hello".into()));
        assert!(result.is_ok());
        assert_eq!(result.ok().unwrap().0, "resolved(parsed(hello))");
    }

    #[test]
    fn failure_short_circuits() {
        let pipeline = TestParse.then(TestFail);
        let result = pipeline.reduce(SourceText("hello".into()));
        assert!(result.is_err());
    }

    #[test]
    fn partial_then_success_stays_partial() {
        let pipeline = TestPartialParse.then(TestResolve);
        let result = pipeline.reduce(SourceText("hello".into()));
        assert!(result.is_partial());
        assert_eq!(
            result.ok().unwrap().0,
            "resolved(partial(hello))"
        );
    }

    #[test]
    fn loss_accumulates_partial_then_partial() {
        // Build a resolve that also returns Partial
        struct PartialResolve;
        impl LambdaFn for PartialResolve {
            type Input = ParsedAst;
            type Output = ResolvedAst;
            type Error = String;
            type Loss = ReductionLoss;

            fn reduce(
                self,
                input: ParsedAst,
            ) -> Imperfect<ResolvedAst, String, ReductionLoss> {
                Imperfect::Partial(
                    ResolvedAst(format!("resolved({})", input.0)),
                    ReductionLoss {
                        steps: 3,
                        budget_remaining: 7,
                    },
                )
            }
        }

        let pipeline = TestPartialParse.then(PartialResolve);
        let result = pipeline.reduce(SourceText("x".into()));
        assert!(result.is_partial());
        // Loss should be combined: steps 2 + 3 = 5
        match result {
            Imperfect::Partial(_, loss) => {
                assert_eq!(loss.steps, 5);
                assert_eq!(loss.budget_remaining, 7); // min(8, 7)
            }
            other => panic!("expected Partial, got {:?}", other),
        }
    }

    #[test]
    fn partial_then_failure_combines_loss() {
        let pipeline = TestPartialParse.then(TestFail);
        let result = pipeline.reduce(SourceText("x".into()));
        assert!(result.is_err());
        match result {
            Imperfect::Failure(msg, loss) => {
                assert_eq!(msg, "resolve failed");
                assert_eq!(loss.steps, 3); // 2 + 1
                assert_eq!(loss.budget_remaining, 0); // min(8, 0)
            }
            other => panic!("expected Failure, got {:?}", other),
        }
    }

    #[test]
    fn three_phase_composition() {
        struct TestEmit;
        impl LambdaFn for TestEmit {
            type Input = ResolvedAst;
            type Output = String;
            type Error = String;
            type Loss = ReductionLoss;

            fn reduce(self, input: ResolvedAst) -> Imperfect<String, String, ReductionLoss> {
                Imperfect::Success(format!("emitted({})", input.0))
            }
        }

        let pipeline = TestParse.then(TestResolve).then(TestEmit);
        let result = pipeline.reduce(SourceText("src".into()));
        assert!(result.is_ok());
        assert_eq!(
            result.ok().unwrap(),
            "emitted(resolved(parsed(src)))"
        );
    }

    // compile-time check: this test just verifies the types align
    #[test]
    fn type_safety_proven_by_compilation() {
        // If these lines compile, the type system has proven correctness.
        let _pipeline = TestParse.then(TestResolve);
        // TestParse.then(TestEmit) would not compile because
        // TestParse::Output = ParsedAst but TestEmit::Input = ResolvedAst.
    }
}
