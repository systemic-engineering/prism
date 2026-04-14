//! Content addressing. A value's identity is derived from its content,
//! not from where it lives or when it was created. Types that implement
//! [`ContentAddressed`] produce an [`Oid`] that is their content-derived identity.

use crate::oid::Oid;

/// A type that has a content address.
/// Alias for [`Addressable`](crate::oid::Addressable) — same trait, legacy name.
pub trait ContentAddressed: crate::oid::Addressable {}

/// Blanket: every Addressable is ContentAddressed.
impl<T: crate::oid::Addressable> ContentAddressed for T {}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::oid::Addressable;

    struct Thing {
        id: String,
    }

    impl Addressable for Thing {
        fn oid(&self) -> Oid {
            Oid::new(format!("thing:{}", self.id))
        }
    }

    #[test]
    fn content_addressed_impl() {
        let t = Thing {
            id: "abc".to_owned(),
        };
        assert_eq!(t.oid().as_str(), "thing:abc");
    }

    #[test]
    fn content_addressed_is_addressable() {
        let t = Thing {
            id: "test".to_owned(),
        };
        fn takes_ca(x: &impl ContentAddressed) -> Oid {
            x.oid()
        }
        assert_eq!(takes_ca(&t).as_str(), "thing:test");
    }
}
