//! Named — a labeled Prism.

#[cfg(test)]
mod tests {
    use super::*;
    use crate::oid::{Oid, Addressable};

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
}
