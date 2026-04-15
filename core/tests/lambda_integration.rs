//! Integration tests for #[derive(Lambda)] and Composable.

#[cfg(feature = "lambda")]
mod tests {
    extern crate prism_core;

    use prism_core::lambda::{Composable, Lambda};
    use prism_core::oid::Addressable;
    use prism_core::DeriveLambda;
    use prism_core::Oid;

    #[derive(DeriveLambda)]
    #[oid("@parse")]
    struct Parse;

    #[derive(DeriveLambda)]
    #[oid("@resolve")]
    struct Resolve;

    #[derive(DeriveLambda)]
    #[oid("@emit")]
    struct Emit;

    #[derive(DeriveLambda)]
    #[oid("@properties")]
    struct Properties;

    #[test]
    fn parse_is_composable() {
        let lambda: Lambda<String> = Parse.into();
        assert!(matches!(lambda, Lambda::Abs(_)));
    }

    #[test]
    fn parse_oid_from_derive() {
        assert_eq!(Parse.oid(), Oid::hash(b"@parse"));
    }

    #[test]
    fn parse_display() {
        assert_eq!(format!("{}", Parse), "@parse");
    }

    #[test]
    fn resolve_display() {
        assert_eq!(format!("{}", Resolve), "@resolve");
    }

    #[test]
    fn parse_then_resolve() {
        let pipeline: Lambda<String> = Parse.then(Resolve);
        assert!(matches!(pipeline, Lambda::Abs(_)));
        assert!(!pipeline.oid().is_dark());
    }

    #[test]
    fn craft_pipeline_composes_four_phases() {
        let craft: Lambda<String> = Parse.then(Resolve).then(Properties).then(Emit);
        assert!(!craft.oid().is_dark());
    }

    #[test]
    fn same_composition_same_oid() {
        let a: Lambda<String> = Parse.then(Resolve);
        let b: Lambda<String> = Parse.then(Resolve);
        assert_eq!(a.oid(), b.oid());
    }

    #[test]
    fn different_composition_different_oid() {
        let a: Lambda<String> = Parse.then(Resolve);
        let b: Lambda<String> = Parse.then(Emit);
        assert_ne!(a.oid(), b.oid());
    }

    #[test]
    fn named_lambda_display() {
        assert_eq!(format!("{}", Parse), "@parse");
        assert_eq!(format!("{}", Resolve), "@resolve");
    }

    #[test]
    fn order_matters() {
        let ab: Lambda<String> = Parse.then(Resolve);
        let ba: Lambda<String> = Resolve.then(Parse);
        assert_ne!(ab.oid(), ba.oid());
    }

    #[test]
    fn apply_to_wraps_in_apply() {
        let input = Lambda::<String>::bind(Oid::hash(b"source"));
        let applied = Parse.apply_to(input);
        assert!(matches!(applied, Lambda::Apply(_)));
    }

    #[test]
    fn into_lambda_is_named_identity() {
        let lambda: Lambda<String> = Parse.into();
        // Should be Abs(@parse, Bind(@parse))
        if let Lambda::Abs(a) = &lambda {
            assert_eq!(a.param, Oid::hash(b"@parse"));
            if let Lambda::Bind(b) = a.body.as_ref() {
                assert_eq!(b.name, Oid::hash(b"@parse"));
            } else {
                panic!("expected Bind body");
            }
        } else {
            panic!("expected Abs");
        }
    }
}
