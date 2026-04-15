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

    // -----------------------------------------------------------------------
    // Optic field annotation tests
    // -----------------------------------------------------------------------

    #[test]
    fn lens_on_plain_field() {
        #[derive(prism_derive::Prism)]
        #[oid("@test/lens")]
        struct Foo {
            #[lens]
            x: u32,
        }
        assert_eq!(Foo::optic_fields()[0].kind, crate::OpticKind::Lens);
        let mut foo = Foo { x: 42 };
        assert_eq!(*XLens::view(&foo), 42);
        XLens::set(&mut foo, 99);
        assert_eq!(foo.x, 99);
    }

    #[test]
    fn prism_on_option_field() {
        #[derive(prism_derive::Prism)]
        #[oid("@test/prism")]
        struct Bar {
            #[prism]
            maybe: Option<String>,
        }
        assert_eq!(Bar::optic_fields()[0].kind, crate::OpticKind::Prism);
        let bar = Bar { maybe: None };
        assert!(MaybePrism::extract(&bar).is_none());

        let mut bar2 = Bar { maybe: None };
        MaybePrism::review(&mut bar2, "hello".to_string());
        assert_eq!(MaybePrism::extract(&bar2), Some(&"hello".to_string()));
    }

    #[test]
    fn traversal_on_vec_field() {
        #[derive(prism_derive::Prism)]
        #[oid("@test/traversal")]
        struct Baz {
            #[traversal]
            items: Vec<i32>,
        }
        assert_eq!(Baz::optic_fields()[0].kind, crate::OpticKind::Traversal);
        let baz = Baz { items: vec![1, 2, 3] };
        assert_eq!(ItemsTraversal::traverse(&baz).len(), 3);
    }

    #[test]
    fn traversal_mut_access() {
        #[derive(prism_derive::Prism)]
        #[oid("@test/traversal-mut")]
        struct Quux {
            #[traversal]
            vals: Vec<u32>,
        }
        let mut q = Quux { vals: vec![10, 20] };
        ValsTraversal::traverse_mut(&mut q).push(30);
        ValsTraversal::traverse_mut(&mut q).push(40);
        assert_eq!(q.vals.len(), 4);
    }

    #[test]
    fn iso_on_field() {
        #[derive(prism_derive::Prism)]
        #[oid("@test/iso")]
        struct IsoTest {
            #[iso]
            value: f64,
        }
        assert_eq!(IsoTest::optic_fields()[0].kind, crate::OpticKind::Iso);
        let mut t = IsoTest { value: 3.14 };
        assert_eq!(*ValueIso::forward(&t), 3.14);
        ValueIso::backward(&mut t, 2.72);
        assert_eq!(t.value, 2.72);
    }

    #[test]
    fn composition_table() {
        use crate::OpticKind::*;
        assert_eq!(Lens.compose(Lens), Lens);
        assert_eq!(Lens.compose(Prism), Prism);
        assert_eq!(Prism.compose(Traversal), Traversal);
        assert_eq!(Iso.compose(Lens), Lens);
        assert_eq!(Fold.compose(Lens), Fold);
    }

    #[test]
    fn optic_fields_metadata() {
        #[derive(prism_derive::Prism)]
        #[oid("@multi")]
        struct Multi {
            #[lens]
            a: u32,
            #[prism]
            b: Option<u32>,
            #[traversal]
            c: Vec<u32>,
        }
        let fields = Multi::optic_fields();
        assert_eq!(fields.len(), 3);
        assert_eq!(fields[0].name, "a");
        assert_eq!(fields[0].kind, crate::OpticKind::Lens);
        assert_eq!(fields[1].name, "b");
        assert_eq!(fields[1].kind, crate::OpticKind::Prism);
        assert_eq!(fields[2].name, "c");
        assert_eq!(fields[2].kind, crate::OpticKind::Traversal);
    }

    #[test]
    fn mixed_annotated_and_unannotated() {
        #[derive(prism_derive::Prism)]
        #[oid("@test/mixed")]
        struct Mixed {
            #[lens]
            visible: u32,
            _hidden: String,
        }
        let fields = Mixed::optic_fields();
        assert_eq!(fields.len(), 1, "only annotated fields appear");
        assert_eq!(fields[0].name, "visible");
    }

    #[test]
    fn prism_inner_still_works() {
        // #[prism(inner)] should still be accepted without generating optic accessors
        #[derive(prism_derive::Prism)]
        #[oid("@test/inner")]
        struct WithInner {
            #[prism(inner)]
            _inner: crate::Crystal<u32>,
        }
        // No optic_fields generated (no bare #[prism] annotation)
        let w = WithInner {
            _inner: crate::Crystal(42, crate::Luminosity::Light),
        };
        assert_eq!(format!("{}", w), "@test/inner");
    }

    #[test]
    fn lens_view_returns_reference() {
        #[derive(prism_derive::Prism)]
        #[oid("@test/ref")]
        struct RefTest {
            #[lens]
            name: String,
        }
        let t = RefTest { name: "hello".to_string() };
        let r: &String = NameLens::view(&t);
        assert_eq!(r, "hello");
    }
}
