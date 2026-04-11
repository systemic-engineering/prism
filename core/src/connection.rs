//! Carrier — the relational substrate flowing through a Beam.
//!
//! ShannonLoss captures scalar information loss.
//! Carrier captures the relational/geometric structure of how
//! pipeline stages are connected.
//!
//! For most pipelines ScalarConnection is sufficient.
//! Non-abelian carriers are for geometric contexts where
//! order matters and parallel transport is non-trivial.

use imperfect::{Loss, ShannonLoss};

/// The relational structure carried by a Beam through a pipeline.
///
/// Laws:
/// - Identity: `C::identity().compose(c) == c` and `c.compose(C::identity()) == c`
/// - Associativity: `(a.compose(b)).compose(c) == a.compose(b.compose(c))`
/// - Norm: `identity().norm()` is zero loss
pub trait Carrier: Clone + Default {
    /// Compose: self followed by other. Non-abelian in general.
    fn compose(self, other: Self) -> Self;
    /// The scalar projection — how much information this connection consumed.
    fn norm(&self) -> ShannonLoss;
}

/// The default connection: pure scalar, no relational structure.
/// compose = add losses, norm = the loss itself.
#[derive(Clone, Default, Debug, PartialEq)]
pub struct ScalarConnection {
    pub loss: ShannonLoss,
}

impl ScalarConnection {
    pub fn zero() -> Self {
        ScalarConnection { loss: ShannonLoss::zero() }
    }
    pub fn new(loss: f64) -> Self {
        ScalarConnection { loss: ShannonLoss::new(loss) }
    }
}

impl Carrier for ScalarConnection {
    fn compose(self, other: Self) -> Self {
        ScalarConnection { loss: ShannonLoss::new(self.loss.as_f64() + other.loss.as_f64()) }
    }
    fn norm(&self) -> ShannonLoss {
        self.loss.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scalar_connection_zero() {
        let c = ScalarConnection::zero();
        assert!(c.norm().is_zero());
    }

    #[test]
    fn scalar_connection_new() {
        let c = ScalarConnection::new(2.5);
        assert_eq!(c.norm().as_f64(), 2.5);
    }

    #[test]
    fn scalar_connection_compose() {
        let a = ScalarConnection::new(1.0);
        let b = ScalarConnection::new(2.0);
        let c = a.compose(b);
        assert_eq!(c.norm().as_f64(), 3.0);
    }

    #[test]
    fn scalar_connection_identity_left() {
        let id = ScalarConnection::default();
        let c = ScalarConnection::new(1.5);
        let result = id.compose(c.clone());
        assert_eq!(result.norm().as_f64(), c.norm().as_f64());
    }

    #[test]
    fn scalar_connection_identity_right() {
        let id = ScalarConnection::default();
        let c = ScalarConnection::new(1.5);
        let result = c.clone().compose(id);
        assert_eq!(result.norm().as_f64(), c.norm().as_f64());
    }
}
