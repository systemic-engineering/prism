//! Monoid structure of optic composition.
//!
//! Classical optic composition forms a monoid: Iso then Iso = Iso,
//! Lens then Lens = Lens, etc. This module provides the identity
//! element and composition for the optics hierarchy.
//!
//! The `PrismMonoid` trait captures this: a type that has an identity
//! element and an associative binary operation (compose).

/// A monoid over optic composition.
///
/// Laws:
/// - Identity: `compose(identity(), p) ≡ p ≡ compose(p, identity())`
/// - Associativity: `compose(compose(a, b), c) ≡ compose(a, compose(b, c))`
pub trait PrismMonoid: Sized {
    /// The identity element.
    fn identity() -> Self;

    /// Monoid composition: run `self` then `other`.
    fn compose(self, other: Self) -> Self;
}

/// Test helper: a monoid carrying a `count` field. Composing two
/// CountMonoids sums their counts, giving a non-trivial monoid.
/// A monoid carrying a `count` field. Composing two CountMonoids sums
/// their counts, giving a non-trivial monoid for testing laws.
#[derive(Debug, Clone, PartialEq)]
pub struct CountMonoid {
    count: u64,
}

impl CountMonoid {
    pub fn new(count: u64) -> Self {
        CountMonoid { count }
    }
    pub fn count(&self) -> u64 {
        self.count
    }
}

impl PrismMonoid for CountMonoid {
    fn identity() -> Self {
        CountMonoid { count: 0 }
    }

    fn compose(self, other: Self) -> Self {
        CountMonoid {
            count: self.count + other.count,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn count_monoid_identity_left() {
        let p = CountMonoid::new(3);
        let id = CountMonoid::identity();
        let composed = id.compose(p.clone());
        assert_eq!(composed.count(), p.count());
    }

    #[test]
    fn count_monoid_identity_right() {
        let p = CountMonoid::new(3);
        let id = CountMonoid::identity();
        let composed = p.clone().compose(id);
        assert_eq!(composed.count(), p.count());
    }

    #[test]
    fn count_monoid_associativity() {
        let a = CountMonoid::new(1);
        let b = CountMonoid::new(2);
        let c = CountMonoid::new(3);
        let left = a.clone().compose(b.clone()).compose(c.clone());
        let right = a.compose(b.compose(c));
        assert_eq!(left.count(), right.count());
        assert_eq!(left.count(), 6);
    }

    #[test]
    fn count_monoid_identity_is_zero() {
        let id = CountMonoid::identity();
        assert_eq!(id.count(), 0);
    }
}
