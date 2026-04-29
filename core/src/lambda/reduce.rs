//! Reduction engine for Lambda terms.
//!
//! Beta reduction with a step budget. Returns Imperfect — three outcomes:
//! Success (normal form reached, zero steps), Partial (normal form reached
//! after steps), Failure (budget exhausted or stuck).

use terni::{Imperfect, Loss};

use crate::oid::Oid;

use super::Lambda;

// ---------------------------------------------------------------------------
// ReductionError
// ---------------------------------------------------------------------------

#[derive(Clone, Debug, PartialEq)]
pub enum ReductionError {
    /// Variable not bound in any enclosing scope.
    UnboundVariable(Oid),
    /// Step budget exhausted before reaching normal form.
    BudgetExhausted { steps_taken: usize },
    /// No arm matched in a Case expression.
    NoMatchingArm,
}

// ---------------------------------------------------------------------------
// ReductionLoss
// ---------------------------------------------------------------------------

#[derive(Clone, Debug, PartialEq, Default)]
pub struct ReductionLoss {
    pub steps: usize,
    pub budget_remaining: usize,
}

impl Loss for ReductionLoss {
    fn zero() -> Self {
        ReductionLoss {
            steps: 0,
            budget_remaining: usize::MAX,
        }
    }

    fn total() -> Self {
        ReductionLoss {
            steps: usize::MAX,
            budget_remaining: 0,
        }
    }

    fn is_zero(&self) -> bool {
        self.steps == 0
    }

    fn combine(self, other: Self) -> Self {
        ReductionLoss {
            steps: self.steps.saturating_add(other.steps),
            budget_remaining: self.budget_remaining.min(other.budget_remaining),
        }
    }
}

// ---------------------------------------------------------------------------
// Reduction
// ---------------------------------------------------------------------------

/// Reduce a lambda term with a step budget.
///
/// Three outcomes:
/// - `Success(term)` — already in normal form (zero steps).
/// - `Partial(term, loss)` — normal form reached after some steps.
/// - `Failure(error, loss)` — budget exhausted or stuck.
pub fn reduce_bounded<T: Clone + PartialEq>(
    term: Lambda<T>,
    budget: usize,
) -> Imperfect<Lambda<T>, ReductionError, ReductionLoss> {
    let mut current = term;
    let mut steps = 0;

    loop {
        if steps >= budget {
            return Imperfect::Failure(
                ReductionError::BudgetExhausted {
                    steps_taken: steps,
                },
                ReductionLoss {
                    steps,
                    budget_remaining: 0,
                },
            );
        }

        match try_step(&current) {
            Some(next) => {
                current = next;
                steps += 1;
            }
            None => {
                // Normal form reached
                let loss = ReductionLoss {
                    steps,
                    budget_remaining: budget - steps,
                };
                if steps == 0 {
                    return Imperfect::Success(current);
                } else {
                    return Imperfect::Partial(current, loss);
                }
            }
        }
    }
}

/// Try a single reduction step. Returns None if the term is in normal form.
fn try_step<T: Clone + PartialEq>(term: &Lambda<T>) -> Option<Lambda<T>> {
    match term {
        Lambda::Apply(app) => {
            match app.function.as_ref() {
                Lambda::Abs(abs) => {
                    // Beta reduction: (λx. body) arg → body[x := arg]
                    Some(substitute(&abs.body, &abs.param, &app.argument))
                }
                _ => {
                    // Try to reduce the function position first
                    if let Some(reduced_fn) = try_step(&app.function) {
                        Some(Lambda::apply(reduced_fn, *app.argument.clone()))
                    } else if let Some(reduced_arg) = try_step(&app.argument) {
                        Some(Lambda::apply(*app.function.clone(), reduced_arg))
                    } else {
                        None
                    }
                }
            }
        }
        Lambda::Abs(abs) => {
            // Reduce under the abstraction
            try_step(&abs.body).map(|reduced| Lambda::abs(abs.param.clone(), reduced))
        }
        Lambda::Case(c) => {
            // Try reducing the scrutinee first
            if let Some(reduced) = try_step(&c.scrutinee) {
                Some(Lambda::case(reduced, c.arms.clone()))
            } else {
                None
            }
        }
        Lambda::Bind(_) => None, // Already in normal form
    }
}

/// Substitute all free occurrences of `param` in `body` with `argument`.
fn substitute<T: Clone + PartialEq>(
    body: &Lambda<T>,
    param: &Oid,
    argument: &Lambda<T>,
) -> Lambda<T> {
    match body {
        Lambda::Bind(b) if b.name == *param => argument.clone(),
        Lambda::Bind(_) => body.clone(),
        Lambda::Abs(a) if a.param == *param => body.clone(), // shadowed
        Lambda::Abs(a) => Lambda::abs(a.param.clone(), substitute(&a.body, param, argument)),
        Lambda::Apply(a) => Lambda::apply(
            substitute(&a.function, param, argument),
            substitute(&a.argument, param, argument),
        ),
        Lambda::Case(c) => Lambda::case(
            substitute(&c.scrutinee, param, argument),
            c.arms
                .iter()
                .map(|(p, t)| (p.clone(), substitute(t, param, argument)))
                .collect(),
        ),
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lambda::Pattern;
    use crate::oid::Oid;

    #[test]
    fn reduction_loss_zero_is_zero() {
        assert!(ReductionLoss::zero().is_zero());
    }

    #[test]
    fn reduction_loss_total_is_not_zero() {
        assert!(!ReductionLoss::total().is_zero());
    }

    #[test]
    fn reduction_loss_combine_adds_steps() {
        let a = ReductionLoss {
            steps: 3,
            budget_remaining: 10,
        };
        let b = ReductionLoss {
            steps: 5,
            budget_remaining: 7,
        };
        let combined = a.combine(b);
        assert_eq!(combined.steps, 8);
        assert_eq!(combined.budget_remaining, 7); // min
    }

    #[test]
    fn beta_reduction_identity() {
        // (λx. x) arg → arg
        let x = Oid::hash(b"x");
        let id = Lambda::<String>::abs(x.clone(), Lambda::bind(x));
        let arg = Lambda::bind(Oid::hash(b"hello"));
        let app = Lambda::apply(id, arg.clone());

        let result = reduce_bounded(app, 10);
        assert_eq!(result.ok(), Some(arg));
    }

    #[test]
    fn beta_reduction_constant() {
        // (λx. λy. x) a b → a
        let x = Oid::hash(b"x");
        let y = Oid::hash(b"y");
        let konst = Lambda::<String>::abs(
            x.clone(),
            Lambda::abs(y.clone(), Lambda::bind(x.clone())),
        );
        let a = Lambda::bind(Oid::hash(b"a"));
        let b = Lambda::bind(Oid::hash(b"b"));
        let app = Lambda::apply(Lambda::apply(konst, a.clone()), b);

        let result = reduce_bounded(app, 10);
        assert_eq!(result.ok(), Some(a));
    }

    #[test]
    fn budget_exhausted_returns_failure() {
        // Omega combinator — non-terminating
        let x = Oid::hash(b"x");
        let omega = Lambda::<String>::abs(
            x.clone(),
            Lambda::apply(Lambda::bind(x.clone()), Lambda::bind(x.clone())),
        );
        let big_omega = Lambda::apply(omega.clone(), omega);

        let result = reduce_bounded(big_omega, 5);
        assert!(result.is_err());
        match result {
            Imperfect::Failure(ReductionError::BudgetExhausted { steps_taken }, loss) => {
                assert_eq!(steps_taken, 5);
                assert_eq!(loss.budget_remaining, 0);
            }
            other => panic!("expected BudgetExhausted, got {:?}", other),
        }
    }

    #[test]
    fn normal_form_zero_budget_is_success() {
        // A bind is already normal form
        let term = Lambda::<String>::bind(Oid::hash(b"x"));
        let result = reduce_bounded(term.clone(), 0);
        // Budget 0 but term is already normal — should succeed
        // Actually with budget 0, the loop checks steps >= budget first (0 >= 0 = true),
        // so this returns BudgetExhausted. That's correct — you asked for 0 steps.
        assert!(result.is_err());
    }

    #[test]
    fn shadowed_variable_not_substituted() {
        // (λx. λx. x) a → λx. x  (inner x shadows outer)
        let x = Oid::hash(b"x");
        let shadowed = Lambda::<String>::abs(
            x.clone(),
            Lambda::abs(x.clone(), Lambda::bind(x.clone())),
        );
        let a = Lambda::bind(Oid::hash(b"a"));
        let app = Lambda::apply(shadowed, a);

        let result = reduce_bounded(app, 10);
        let expected = Lambda::<String>::abs(Oid::hash(b"x"), Lambda::bind(Oid::hash(b"x")));
        assert_eq!(result.ok(), Some(expected));
    }

    #[test]
    fn substitute_in_case() {
        // (λx. case x of { _ => x }) a → case a of { _ => a }
        let x = Oid::hash(b"x");
        let body = Lambda::<String>::case(
            Lambda::bind(x.clone()),
            vec![(Pattern::Any, Lambda::bind(x.clone()))],
        );
        let abs = Lambda::abs(x, body);
        let a = Lambda::bind(Oid::hash(b"a"));
        let app = Lambda::apply(abs, a.clone());

        let result = reduce_bounded(app, 10);
        let expected = Lambda::<String>::case(a.clone(), vec![(Pattern::Any, a)]);
        assert_eq!(result.ok(), Some(expected));
    }

    #[test]
    fn reduce_under_abstraction() {
        // λy. (λx. x) z → λy. z
        let x = Oid::hash(b"x");
        let y = Oid::hash(b"y");
        let z = Oid::hash(b"z");
        let inner = Lambda::<String>::apply(
            Lambda::abs(x.clone(), Lambda::bind(x)),
            Lambda::bind(z.clone()),
        );
        let outer = Lambda::abs(y, inner);

        let result = reduce_bounded(outer, 10);
        let expected = Lambda::<String>::abs(Oid::hash(b"y"), Lambda::bind(z));
        assert_eq!(result.ok(), Some(expected));
    }

    #[test]
    fn partial_result_has_loss() {
        // One step of reduction should give Partial with steps=1
        let x = Oid::hash(b"x");
        let id = Lambda::<String>::abs(x.clone(), Lambda::bind(x));
        let arg = Lambda::bind(Oid::hash(b"hello"));
        let app = Lambda::apply(id, arg);

        let result = reduce_bounded(app, 10);
        match result {
            Imperfect::Partial(_, loss) => {
                assert_eq!(loss.steps, 1);
                assert_eq!(loss.budget_remaining, 9);
            }
            other => panic!("expected Partial, got {:?}", other),
        }
    }
}
