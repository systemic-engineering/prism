//! Named — a labeled Prism.

use crate::oid::{Oid, Addressable};

/// A labeled Prism. The name is for humans. The OID is for the graph.
///
/// Named("focus", optic) — the optic with a name.
/// The OID is derived from both the name and the inner optic.
#[derive(Debug, Clone, PartialEq)]
pub struct Named<P>(pub &'static str, pub P);

impl<P> Named<P> {
    pub fn name(&self) -> &'static str {
        self.0
    }

    pub fn inner(&self) -> &P {
        &self.1
    }

    pub fn into_inner(self) -> P {
        self.1
    }
}

impl<P: Addressable> Addressable for Named<P> {
    fn oid(&self) -> Oid {
        let inner_oid = self.1.oid();
        let combined = format!("named:{}:{}", self.0, inner_oid);
        Oid::hash(combined.as_bytes())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Debug, Clone, PartialEq)]
    struct FocusOptic(u32);

    impl Addressable for FocusOptic {
        fn oid(&self) -> Oid {
            Oid::hash(&self.0.to_le_bytes())
        }
    }

    #[test]
    fn named_wraps_optic() {
        let named = Named("focus", FocusOptic(1));
        assert_eq!(named.name(), "focus");
        assert_eq!(named.inner(), &FocusOptic(1));
    }

    #[test]
    fn named_oid_includes_name() {
        let a = Named("focus", FocusOptic(1));
        let b = Named("project", FocusOptic(1));
        // Same inner optic, different name → different OID
        assert_ne!(a.oid(), b.oid());
    }

    #[test]
    fn named_same_name_same_optic_same_oid() {
        let a = Named("focus", FocusOptic(1));
        let b = Named("focus", FocusOptic(1));
        assert_eq!(a.oid(), b.oid());
    }

    #[test]
    fn named_different_optic_different_oid() {
        let a = Named("focus", FocusOptic(1));
        let b = Named("focus", FocusOptic(2));
        assert_ne!(a.oid(), b.oid());
    }

    // --- #[derive(Named)] tests ---

    #[derive(prism_derive::Named)]
    #[oid("@test")]
    struct TestNamed {
        _value: u32,
    }

    #[test]
    fn derived_named_has_oid() {
        let t = TestNamed { _value: 42 };
        let oid = t.oid();
        assert!(!oid.is_dark());
    }

    #[test]
    fn derived_named_display() {
        let t = TestNamed { _value: 42 };
        assert_eq!(format!("{}", t), "@test");
    }

    #[test]
    fn derived_named_oid_deterministic() {
        let a = TestNamed { _value: 1 };
        let b = TestNamed { _value: 2 };
        // Same @oid string → same Oid (the oid comes from the name, not the value)
        assert_eq!(a.oid(), b.oid());
    }

    #[derive(prism_derive::Named)]
    #[oid("@wrapper")]
    struct TestWrapper {
        #[prism(inner)]
        _inner: crate::Crystal<u32>,
        _extra: String,
    }

    #[test]
    fn derived_named_with_prism_inner_display() {
        let w = TestWrapper {
            _inner: crate::Crystal(42, crate::Luminosity::Light),
            _extra: "test".into(),
        };
        assert_eq!(format!("{}", w), "@wrapper");
    }

    #[test]
    fn derived_named_with_prism_inner_oid() {
        let w = TestWrapper {
            _inner: crate::Crystal(42, crate::Luminosity::Light),
            _extra: "test".into(),
        };
        assert!(!w.oid().is_dark());
    }
}
