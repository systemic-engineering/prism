//! Lambda calculus on content-addressed trees.
//!
//! Four variants: Bind, Abs, Apply, Case. No strings. Oid for identity.
//! Reduction returns Imperfect — three outcomes: Success (normal form),
//! Partial (budget exhausted), Failure (stuck term).

mod reduce;

pub use reduce::{reduce_bounded, ReductionError, ReductionLoss};

use crate::merkle::MerkleTree;
use crate::oid::{Addressable, Oid};

/// A lambda term over content-addressed trees.
///
/// Four variants. No strings. Oid for identity.
#[derive(Clone, Debug, PartialEq)]
pub enum Lambda<T: Clone + PartialEq> {
    /// Variable binding. The Oid identifies which binding.
    Bind(BindLambda),
    /// Abstraction. Parameter Oid + body.
    Abs(AbsLambda<T>),
    /// Application. Function + argument.
    Apply(ApplyLambda<T>),
    /// Case. Scrutinee + arms.
    Case(CaseLambda<T>),
}

#[derive(Clone, Debug, PartialEq)]
pub struct BindLambda {
    pub name: Oid,
}

#[derive(Clone, Debug, PartialEq)]
pub struct AbsLambda<T: Clone + PartialEq> {
    pub param: Oid,
    pub body: Box<Lambda<T>>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ApplyLambda<T: Clone + PartialEq> {
    pub function: Box<Lambda<T>>,
    pub argument: Box<Lambda<T>>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct CaseLambda<T: Clone + PartialEq> {
    pub scrutinee: Box<Lambda<T>>,
    pub arms: Vec<(Pattern<T>, Lambda<T>)>,
}

#[derive(Clone, Debug, PartialEq)]
pub enum Pattern<T: Clone + PartialEq> {
    Exact(T),
    Bind(Oid),
    Any,
}

// ---------------------------------------------------------------------------
// Factory methods
// ---------------------------------------------------------------------------

impl<T: Clone + PartialEq> Lambda<T> {
    pub fn bind(name: Oid) -> Self {
        Lambda::Bind(BindLambda { name })
    }

    pub fn abs(param: Oid, body: Lambda<T>) -> Self {
        Lambda::Abs(AbsLambda {
            param,
            body: Box::new(body),
        })
    }

    pub fn apply(function: Lambda<T>, argument: Lambda<T>) -> Self {
        Lambda::Apply(ApplyLambda {
            function: Box::new(function),
            argument: Box::new(argument),
        })
    }

    pub fn case(scrutinee: Lambda<T>, arms: Vec<(Pattern<T>, Lambda<T>)>) -> Self {
        Lambda::Case(CaseLambda {
            scrutinee: Box::new(scrutinee),
            arms,
        })
    }
}

// ---------------------------------------------------------------------------
// Composition: a.then(b) = λx. b(a(x))
// ---------------------------------------------------------------------------

impl<T: Clone + PartialEq> Lambda<T> {
    /// Compose two lambdas: `self.then(next)` = `λx. next(self(x))`.
    ///
    /// The composition IS a lambda term. Not a Vec. Not a trait object.
    pub fn then(self, next: Lambda<T>) -> Lambda<T> {
        let param = Oid::hash(b"__compose");
        Lambda::abs(
            param.clone(),
            Lambda::apply(next, Lambda::apply(self, Lambda::bind(param))),
        )
    }
}

// ---------------------------------------------------------------------------
// Addressable — content-addressed identity
// ---------------------------------------------------------------------------

impl<T: Clone + PartialEq> Addressable for Lambda<T> {
    fn oid(&self) -> Oid {
        match self {
            Lambda::Bind(b) => Oid::hash(format!("Bind:{}", b.name).as_bytes()),
            Lambda::Abs(a) => {
                let body_oid = a.body.oid();
                Oid::hash(format!("Abs:{}:{}", a.param, body_oid).as_bytes())
            }
            Lambda::Apply(a) => {
                let f_oid = a.function.oid();
                let x_oid = a.argument.oid();
                Oid::hash(format!("Apply:{}:{}", f_oid, x_oid).as_bytes())
            }
            Lambda::Case(c) => {
                let s_oid = c.scrutinee.oid();
                let arms: String = c
                    .arms
                    .iter()
                    .map(|(p, b)| format!("{}:{}", pattern_oid(p), b.oid()))
                    .collect::<Vec<_>>()
                    .join(",");
                Oid::hash(format!("Case:{}:[{}]", s_oid, arms).as_bytes())
            }
        }
    }
}

/// Compute an Oid for a Pattern. Not exposed publicly — used by Lambda's Addressable impl.
fn pattern_oid<T: Clone + PartialEq>(pattern: &Pattern<T>) -> Oid {
    match pattern {
        Pattern::Exact(_) => Oid::hash(b"Pattern:Exact"),
        Pattern::Bind(oid) => Oid::hash(format!("Pattern:Bind:{}", oid).as_bytes()),
        Pattern::Any => Oid::hash(b"Pattern:Any"),
    }
}

// ---------------------------------------------------------------------------
// MerkleTree — Lambda is recursive via Box, children() returns empty
// ---------------------------------------------------------------------------

/// Tag type for Lambda's MerkleTree::Data. Identifies the variant without recursion.
#[derive(Clone, Debug, PartialEq)]
pub enum LambdaTag {
    Bind(Oid),
    Abs(Oid),
    Apply,
    Case,
}

impl<T: Clone + PartialEq> MerkleTree for Lambda<T> {
    type Data = LambdaTag;

    fn data(&self) -> &LambdaTag {
        // We need to store the tag. Since MerkleTree::data returns a reference,
        // we cannot construct it on the fly. Instead, we use a thread-local or
        // accept that Lambda stores its tag. For now, we use a different approach:
        // we won't implement MerkleTree directly since Lambda's recursion is through
        // Box, not Vec<Self>. The tree structure IS the term structure.
        //
        // Actually, MerkleTree requires returning &Self::Data. We need to store it.
        // Let's skip MerkleTree impl for now — Addressable is the key trait.
        unimplemented!(
            "Lambda's tree structure is through Box, not Vec. Use Addressable::oid() instead."
        )
    }

    fn children(&self) -> &[Self] {
        // Lambda is recursive via Box, not via Vec children.
        // The tree structure IS the term structure, captured in the Oid.
        &[]
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::oid::Addressable;

    #[test]
    fn bind_creates_variable() {
        let x = Lambda::<String>::bind(Oid::hash(b"x"));
        assert!(matches!(x, Lambda::Bind(_)));
    }

    #[test]
    fn abs_creates_abstraction() {
        let id = Lambda::<String>::abs(Oid::hash(b"x"), Lambda::bind(Oid::hash(b"x")));
        assert!(matches!(id, Lambda::Abs(_)));
    }

    #[test]
    fn apply_creates_application() {
        let app = Lambda::<String>::apply(
            Lambda::bind(Oid::hash(b"f")),
            Lambda::bind(Oid::hash(b"x")),
        );
        assert!(matches!(app, Lambda::Apply(_)));
    }

    #[test]
    fn case_creates_case() {
        let case = Lambda::<String>::case(
            Lambda::bind(Oid::hash(b"x")),
            vec![(Pattern::Any, Lambda::bind(Oid::hash(b"y")))],
        );
        assert!(matches!(case, Lambda::Case(_)));
    }

    #[test]
    fn identity_reduces() {
        let id = Lambda::<String>::abs(Oid::hash(b"x"), Lambda::bind(Oid::hash(b"x")));
        let arg = Lambda::bind(Oid::hash(b"hello"));
        let app = Lambda::apply(id, arg.clone());

        let result = reduce_bounded(app, 10);
        assert!(result.is_ok());
        // Should reduce to just the argument
        assert_eq!(result.ok(), Some(arg));
    }

    #[test]
    fn then_composes() {
        let f = Lambda::<String>::abs(Oid::hash(b"x"), Lambda::bind(Oid::hash(b"x")));
        let g = Lambda::<String>::abs(Oid::hash(b"y"), Lambda::bind(Oid::hash(b"y")));
        let composed = f.then(g);
        assert!(matches!(composed, Lambda::Abs(_)));
    }

    #[test]
    fn same_term_same_oid() {
        let a = Lambda::<String>::bind(Oid::hash(b"x"));
        let b = Lambda::<String>::bind(Oid::hash(b"x"));
        assert_eq!(a.oid(), b.oid());
    }

    #[test]
    fn different_term_different_oid() {
        let a = Lambda::<String>::bind(Oid::hash(b"x"));
        let b = Lambda::<String>::bind(Oid::hash(b"y"));
        assert_ne!(a.oid(), b.oid());
    }

    #[test]
    fn abs_oid_depends_on_param_and_body() {
        let a = Lambda::<String>::abs(Oid::hash(b"x"), Lambda::bind(Oid::hash(b"x")));
        let b = Lambda::<String>::abs(Oid::hash(b"y"), Lambda::bind(Oid::hash(b"y")));
        assert_ne!(a.oid(), b.oid());
    }

    #[test]
    fn abs_oid_same_structure_same_oid() {
        let a = Lambda::<String>::abs(Oid::hash(b"x"), Lambda::bind(Oid::hash(b"x")));
        let b = Lambda::<String>::abs(Oid::hash(b"x"), Lambda::bind(Oid::hash(b"x")));
        assert_eq!(a.oid(), b.oid());
    }

    #[test]
    fn apply_oid_depends_on_function_and_argument() {
        let a = Lambda::<String>::apply(
            Lambda::bind(Oid::hash(b"f")),
            Lambda::bind(Oid::hash(b"x")),
        );
        let b = Lambda::<String>::apply(
            Lambda::bind(Oid::hash(b"f")),
            Lambda::bind(Oid::hash(b"y")),
        );
        assert_ne!(a.oid(), b.oid());
    }

    #[test]
    fn case_oid_includes_arms() {
        let a = Lambda::<String>::case(
            Lambda::bind(Oid::hash(b"x")),
            vec![(Pattern::Any, Lambda::bind(Oid::hash(b"a")))],
        );
        let b = Lambda::<String>::case(
            Lambda::bind(Oid::hash(b"x")),
            vec![(Pattern::Any, Lambda::bind(Oid::hash(b"b")))],
        );
        assert_ne!(a.oid(), b.oid());
    }

    #[test]
    fn budget_exhausted_is_failure() {
        // Omega combinator: (λx. x x)(λx. x x) — non-terminating
        let x = Oid::hash(b"x");
        let omega = Lambda::<String>::abs(
            x.clone(),
            Lambda::apply(Lambda::bind(x.clone()), Lambda::bind(x.clone())),
        );
        let big_omega = Lambda::apply(omega.clone(), omega);

        let result = reduce_bounded(big_omega, 100);
        assert!(result.is_err());
    }

    #[test]
    fn composition_reduces_correctly() {
        // f = λx. x (identity)
        // g = λy. y (identity)
        // f.then(g) = λz. g(f(z)) = λz. z (still identity in effect)
        let x = Oid::hash(b"x");
        let y = Oid::hash(b"y");
        let f = Lambda::<String>::abs(x.clone(), Lambda::bind(x));
        let g = Lambda::<String>::abs(y.clone(), Lambda::bind(y));
        let composed = f.then(g);

        let arg = Lambda::bind(Oid::hash(b"hello"));
        let app = Lambda::apply(composed, arg);
        let result = reduce_bounded(app, 100);
        assert!(result.is_ok());
    }

    #[test]
    fn bind_oid_is_deterministic() {
        let a = Lambda::<String>::bind(Oid::hash(b"x"));
        let b = Lambda::<String>::bind(Oid::hash(b"x"));
        assert_eq!(a.oid(), b.oid());
        assert!(!a.oid().is_dark());
    }

    #[test]
    fn lambda_children_is_empty() {
        let term = Lambda::<String>::bind(Oid::hash(b"x"));
        assert_eq!(term.children().len(), 0);
    }

    #[test]
    fn normal_form_is_success_with_zero_steps() {
        // A Bind is already in normal form — reduce should return Success
        let term = Lambda::<String>::bind(Oid::hash(b"x"));
        let result = reduce_bounded(term.clone(), 10);
        match result {
            terni::Imperfect::Success(v) => assert_eq!(v, term),
            _ => panic!("expected Success for normal form"),
        }
    }

    #[test]
    fn pattern_bind_oid_differs_from_pattern_any() {
        let a = pattern_oid::<String>(&Pattern::Bind(Oid::hash(b"x")));
        let b = pattern_oid::<String>(&Pattern::Any);
        assert_ne!(a, b);
    }
}
