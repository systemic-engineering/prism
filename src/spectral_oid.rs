use crate::oid::Oid;
use crate::precision::Precision;
use std::fmt;
use std::hash::{Hash, Hasher};

#[derive(Clone, Debug)]
pub struct SpectralOid {
    raw: String,
    precision: Precision,
    truncated: String,
}

impl SpectralOid {
    pub fn new(_raw: impl Into<String>, _precision: Precision) -> Self {
        todo!()
    }

    pub fn raw(&self) -> &str {
        todo!()
    }

    pub fn precision(&self) -> &Precision {
        todo!()
    }

    pub fn as_str(&self) -> &str {
        todo!()
    }

    pub fn to_oid(&self) -> Oid {
        todo!()
    }
}

fn truncation_len(_total: usize, _precision: &Precision) -> usize {
    todo!()
}

impl PartialEq for SpectralOid {
    fn eq(&self, _other: &Self) -> bool {
        todo!()
    }
}

impl Eq for SpectralOid {}

impl Hash for SpectralOid {
    fn hash<H: Hasher>(&self, _state: &mut H) {
        todo!()
    }
}

impl fmt::Display for SpectralOid {
    fn fmt(&self, _f: &mut fmt::Formatter<'_>) -> fmt::Result {
        todo!()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    #[test]
    fn same_raw_same_precision_are_equal() {
        let a = SpectralOid::new("abcdef", Precision::new(1.0));
        let b = SpectralOid::new("abcdef", Precision::new(1.0));
        assert_eq!(a, b);
    }

    #[test]
    fn different_raw_same_precision_differ() {
        let a = SpectralOid::new("abcdef", Precision::new(1.0));
        let b = SpectralOid::new("xyzqrs", Precision::new(1.0));
        assert_ne!(a, b);
    }

    #[test]
    fn coarse_precision_makes_similar_strings_equal() {
        let a = SpectralOid::new("abcd000000000000", Precision::new(0.25));
        let b = SpectralOid::new("abcd999999999999", Precision::new(0.25));
        assert_eq!(a.as_str(), "abcd");
        assert_eq!(b.as_str(), "abcd");
        assert_eq!(a, b);
    }

    #[test]
    fn fine_precision_distinguishes() {
        let a = SpectralOid::new("abcd000000000000", Precision::new(1.0));
        let b = SpectralOid::new("abcd999999999999", Precision::new(1.0));
        assert_ne!(a, b);
    }

    #[test]
    fn hash_set_dedup_at_coarse_precision() {
        let a = SpectralOid::new("abcd000000000000", Precision::new(0.25));
        let b = SpectralOid::new("abcd999999999999", Precision::new(0.25));
        let mut set = HashSet::new();
        set.insert(a);
        set.insert(b);
        assert_eq!(set.len(), 1);
    }

    #[test]
    fn hash_set_keeps_distinct_at_fine_precision() {
        let a = SpectralOid::new("abcd000000000000", Precision::new(1.0));
        let b = SpectralOid::new("abcd999999999999", Precision::new(1.0));
        let mut set = HashSet::new();
        set.insert(a);
        set.insert(b);
        assert_eq!(set.len(), 2);
    }

    #[test]
    fn raw_returns_full_string() {
        let oid = SpectralOid::new("full-string", Precision::new(0.5));
        assert_eq!(oid.raw(), "full-string");
    }

    #[test]
    fn to_oid_uses_truncated() {
        let soid = SpectralOid::new("abcdefgh", Precision::new(0.5));
        let oid = soid.to_oid();
        assert_eq!(oid.as_str(), "abcd");
        assert_eq!(oid.as_str(), soid.as_str());
    }

    #[test]
    fn display_shows_truncated_at_precision() {
        let soid = SpectralOid::new("abcdefgh", Precision::new(0.5));
        let displayed = format!("{}", soid);
        assert!(displayed.contains('@'), "display must contain '@'");
        assert!(
            displayed.starts_with("abcd"),
            "display must start with truncated prefix, got: {}",
            displayed
        );
    }

    #[test]
    fn minimum_one_char_kept() {
        let soid = SpectralOid::new("hello", Precision::new(0.0));
        assert_eq!(soid.as_str().len(), 1);
    }

    #[test]
    fn full_precision_keeps_all() {
        let input = "hello world";
        let soid = SpectralOid::new(input, Precision::new(1.0));
        assert_eq!(soid.as_str(), input);
    }

    #[test]
    fn truncation_len_edge_cases() {
        assert_eq!(truncation_len(0, &Precision::new(0.5)), 0);
        assert_eq!(truncation_len(10, &Precision::new(0.0)), 1);
        assert_eq!(truncation_len(10, &Precision::new(1.0)), 10);
        assert_eq!(truncation_len(10, &Precision::new(0.5)), 5);
    }
}
