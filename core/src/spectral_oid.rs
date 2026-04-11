//! Spectral object identity. A [`SpectralOid`] is an [`Oid`](crate::oid::Oid)
//! viewed at a specific [`Precision`](crate::precision::Precision). Truncating
//! the raw content address to fewer characters makes coarser identities —
//! distinct values become "equal" when their truncated representations match.
//! This is how the project operation controls resolution: lower precision
//! merges more things into the same identity.

use crate::oid::Oid;
use crate::precision::Precision;
use std::fmt;
use std::hash::{Hash, Hasher};

/// A content address at a specific precision.
///
/// Two SpectralOids are equal if their truncated eigenvalue
/// representations match. The precision determines the truncation
/// depth: fewer significant digits = coarser = more things are "equal".
///
/// Equality is on the truncated form only. Two SpectralOids at
/// different precisions may be equal if they happen to truncate
/// identically.
///
/// Implements PartialEq, Eq, Hash — standard Rust equality semantics
/// where equality IS spectral closeness.
#[derive(Clone, Debug)]
pub struct SpectralOid {
    raw: String,
    precision: Precision,
    truncated: String,
}

impl SpectralOid {
    pub fn new(raw: impl Into<String>, precision: Precision) -> Self {
        let raw = raw.into();
        let len = truncation_len(raw.chars().count(), &precision);
        let truncated: String = raw.chars().take(len).collect();
        SpectralOid {
            raw,
            precision,
            truncated,
        }
    }

    pub fn raw(&self) -> &str {
        &self.raw
    }

    pub fn precision(&self) -> &Precision {
        &self.precision
    }

    /// Returns the truncated form — the identity at this precision.
    pub fn as_str(&self) -> &str {
        &self.truncated
    }

    /// Wraps the truncated form in an Oid.
    pub fn to_oid(&self) -> Oid {
        Oid::new(self.truncated.clone())
    }
}

/// Maps a precision to the number of characters to keep from a string of
/// `total` characters. Always keeps at least 1 character when total > 0.
fn truncation_len(total: usize, precision: &Precision) -> usize {
    if total == 0 {
        return 0;
    }
    let p = precision.as_f64();
    debug_assert!(
        p.is_finite() && p >= 0.0,
        "precision must be finite and non-negative, got {p}"
    );
    let p_clamped = p.clamp(0.0, 1.0);
    let keep = (total as f64 * p_clamped).ceil() as usize;
    keep.clamp(1, total)
}

// ---------------------------------------------------------------------------
// Equality is on the truncated form
// ---------------------------------------------------------------------------

impl PartialEq for SpectralOid {
    fn eq(&self, other: &Self) -> bool {
        self.truncated == other.truncated
    }
}

impl Eq for SpectralOid {}

impl Hash for SpectralOid {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.truncated.hash(state);
    }
}

impl fmt::Display for SpectralOid {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}@{}", self.truncated, self.precision)
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

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
        // 16 chars, precision 0.25 → ceil(16 * 0.25) = 4 chars kept
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
        // 8 chars, precision 0.5 → ceil(8 * 0.5) = 4 chars kept
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
        // precision 0.0 → ceil(total * 0.0) = 0, clamped to 1
        let soid = SpectralOid::new("hello", Precision::new(0.0));
        assert_eq!(soid.as_str().chars().count(), 1);
    }

    #[test]
    fn full_precision_keeps_all() {
        let input = "hello world";
        let soid = SpectralOid::new(input, Precision::new(1.0));
        assert_eq!(soid.as_str(), input);
    }

    #[test]
    fn unicode_truncation_by_chars_not_bytes() {
        // 4 CJK characters, each 3 bytes in UTF-8
        let input = "\u{4e00}\u{4e8c}\u{4e09}\u{56db}"; // 一二三四
        let soid = SpectralOid::new(input, Precision::new(0.5));
        assert_eq!(soid.as_str().chars().count(), 2);
        assert_eq!(soid.as_str(), "\u{4e00}\u{4e8c}"); // 一二
    }

    #[test]
    fn empty_string_produces_valid_oid() {
        let soid = SpectralOid::new("", Precision::new(0.5));
        assert_eq!(soid.as_str(), "");
        assert_eq!(soid.raw(), "");
        assert_eq!(soid.to_oid(), Oid::new(""));
    }

    #[test]
    fn truncation_len_edge_cases() {
        // 0 → 0
        assert_eq!(truncation_len(0, &Precision::new(0.5)), 0);
        // 10 at 0.0 → ceil(0) = 0, clamped to 1
        assert_eq!(truncation_len(10, &Precision::new(0.0)), 1);
        // 10 at 1.0 → ceil(10) = 10
        assert_eq!(truncation_len(10, &Precision::new(1.0)), 10);
        // 10 at 0.5 → ceil(5.0) = 5
        assert_eq!(truncation_len(10, &Precision::new(0.5)), 5);
    }
}
