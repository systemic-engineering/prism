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

    // --- #[derive(Prism)] tests ---

    #[derive(prism_derive::Prism)]
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

    #[derive(prism_derive::Prism)]
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

    // --- Cascade tests: three-level derive(Prism) ---

    #[derive(prism_derive::Prism)]
    #[oid("@test/simple")]
    struct Simple;

    #[derive(Clone, prism_derive::Prism)]
    #[oid("@test/with-inner")]
    struct WithInner {
        #[prism(inner)]
        _inner: crate::Crystal<u32>,
        _extra: String,
    }

    #[derive(prism_derive::Prism)]
    #[oid("@test/nested")]
    struct Nested {
        #[prism(inner)]
        _inner: WithInner,
    }

    #[test]
    fn derive_cascade_oids_differ() {
        let s = Simple;
        let w = WithInner {
            _inner: crate::Crystal(42, crate::Luminosity::Light),
            _extra: "x".into(),
        };
        let n = Nested { _inner: w.clone() };

        // Different @oid strings → different Oids
        assert_ne!(s.oid(), w.oid());
        assert_ne!(w.oid(), n.oid());
    }

    #[test]
    fn derive_cascade_display() {
        assert_eq!(format!("{}", Simple), "@test/simple");
        assert_eq!(
            format!(
                "{}",
                WithInner {
                    _inner: crate::Crystal(42, crate::Luminosity::Light),
                    _extra: "x".into(),
                }
            ),
            "@test/with-inner"
        );
        assert_eq!(
            format!(
                "{}",
                Nested {
                    _inner: WithInner {
                        _inner: crate::Crystal(42, crate::Luminosity::Light),
                        _extra: "x".into(),
                    },
                }
            ),
            "@test/nested"
        );
    }

    #[test]
    fn derive_cascade_three_levels() {
        // Three nested derives. Each one is a Prism.
        // The Oid at each level is derived from the @oid string, not the inner value.
        let n = Nested {
            _inner: WithInner {
                _inner: crate::Crystal(42, crate::Luminosity::Light),
                _extra: "x".into(),
            },
        };

        // The Oid is from "@test/nested", not from the inner Crystal
        let oid = n.oid();
        assert!(!oid.is_dark());

        // Same @oid string → same Oid regardless of inner value
        let n2 = Nested {
            _inner: WithInner {
                _inner: crate::Crystal(99, crate::Luminosity::Dark),
                _extra: "y".into(),
            },
        };
        assert_eq!(n.oid(), n2.oid()); // same @oid string
    }

    #[test]
    fn derive_vs_hand_written_same_oid() {
        // Hand-written impl
        struct HandWritten;
        impl Addressable for HandWritten {
            fn oid(&self) -> Oid {
                Oid::hash("@test/simple".as_bytes())
            }
        }
        // Derive-generated
        let derived = Simple;
        let hand = HandWritten;
        assert_eq!(derived.oid(), hand.oid());
    }

    #[test]
    fn derive_vs_hand_written_same_display() {
        assert_eq!(format!("{}", Simple), "@test/simple");
    }

    #[test]
    fn benchmark_derive_prism_hash() {
        // Warm up
        let _ = Simple.oid();

        let start = std::time::Instant::now();
        for _ in 0..1_000 {
            let _ = Simple.oid();
        }
        let elapsed = start.elapsed();
        eprintln!("--- derive(Prism) oid: 1k calls in {:?} ---", elapsed);
        // Each call runs CoincidenceHash<3> (eigenvalue-based).
        // No LazyLock in the derive — each call recomputes the hash.
        // ~1ms per call is expected for the full coincidence detector pipeline.
    }
}
