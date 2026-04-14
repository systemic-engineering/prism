//! Object identity. An [`Oid`] is the content address of a value — two values
//! with the same bytes produce the same Oid. Oids are the nodes in every
//! graph this system builds: content-addressed, comparable, hashable.

/// Content address. The identity of a thing is its content.
/// Two values with the same bytes have the same Oid.
/// Oids are the nodes in every graph this system builds.
#[derive(Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Oid(String);

impl Oid {
    pub fn new(s: impl Into<String>) -> Self {
        Oid(s.into())
    }

    /// Hash bytes to produce an Oid. Deterministic content addressing.
    ///
    /// Uses CoincidenceHash<3> — three independent projection observers
    /// in a 16-dimensional space. The shared eigenvalue becomes the content
    /// address, compressed through SHA-256 to a fixed 64-char hex string.
    /// Falls back to SHA-256 with domain separation for degenerate input.
    ///
    /// Deterministic across Rust versions (no SipHash/DefaultHasher).
    pub fn hash(bytes: &[u8]) -> Self {
        let hex_str = crate::coincidence::canonical_hash(bytes);
        Oid(hex_str)
    }

    /// The dark OID. The address of absence. Constant.
    pub fn dark() -> Self {
        Oid("0".repeat(64))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Is this the dark OID?
    pub fn is_dark(&self) -> bool {
        self.0 == "0".repeat(64)
    }
}

impl std::fmt::Display for Oid {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl std::fmt::Debug for Oid {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.0.len() >= 12 {
            write!(f, "Oid({})", &self.0[..12])
        } else {
            write!(f, "Oid({})", &self.0)
        }
    }
}

impl AsRef<str> for Oid {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

impl From<&str> for Oid {
    fn from(s: &str) -> Self {
        Oid(s.to_owned())
    }
}

impl From<String> for Oid {
    fn from(s: String) -> Self {
        Oid(s)
    }
}

/// The thing has an address. That's all.
pub trait Addressable {
    fn oid(&self) -> Oid;
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    #[test]
    fn new_and_as_str() {
        let oid = Oid::new("abc123");
        assert_eq!(oid.as_str(), "abc123");
    }

    #[test]
    fn display() {
        let oid = Oid::new("hello");
        assert_eq!(format!("{}", oid), "hello");
    }

    #[test]
    fn equality() {
        let a = Oid::new("foo");
        let b = Oid::new("foo");
        let c = Oid::new("bar");
        assert_eq!(a, b);
        assert_ne!(a, c);
    }

    #[test]
    fn ordering() {
        let a = Oid::new("apple");
        let b = Oid::new("banana");
        assert!(a < b);
        assert!(b > a);
    }

    #[test]
    fn from_str() {
        let oid: Oid = "test".into();
        assert_eq!(oid.as_str(), "test");
    }

    #[test]
    fn from_string() {
        let oid: Oid = String::from("owned").into();
        assert_eq!(oid.as_str(), "owned");
    }

    #[test]
    fn as_ref() {
        let oid = Oid::new("reftest");
        let s: &str = oid.as_ref();
        assert_eq!(s, "reftest");
    }

    #[test]
    fn hash_set_dedup() {
        let mut set = HashSet::new();
        set.insert(Oid::new("x"));
        set.insert(Oid::new("x"));
        set.insert(Oid::new("y"));
        assert_eq!(set.len(), 2);
    }

    #[test]
    fn clone() {
        let a = Oid::new("original");
        let b = a.clone();
        assert_eq!(a, b);
    }

    // --- Foundation tests: hash, dark, Addressable ---

    #[test]
    fn oid_from_bytes() {
        let a = Oid::hash(b"hello");
        let b = Oid::hash(b"hello");
        assert_eq!(a, b); // deterministic
    }

    #[test]
    fn oid_different_content_different_hash() {
        let a = Oid::hash(b"hello");
        let b = Oid::hash(b"world");
        assert_ne!(a, b);
    }

    #[test]
    fn oid_display_is_hex() {
        let oid = Oid::hash(b"test");
        let s = format!("{}", oid);
        assert_eq!(s.len(), 64); // 32 bytes = 64 hex chars
        assert!(s.chars().all(|c| c.is_ascii_hexdigit()));
    }

    #[test]
    fn oid_dark_is_constant() {
        assert_eq!(Oid::dark(), Oid::dark());
    }

    #[test]
    fn oid_dark_differs_from_any_content() {
        assert_ne!(Oid::dark(), Oid::hash(b""));
        assert_ne!(Oid::dark(), Oid::hash(b"anything"));
    }

    struct TestValue(u32);

    impl Addressable for TestValue {
        fn oid(&self) -> Oid {
            Oid::hash(&self.0.to_le_bytes())
        }
    }

    #[test]
    fn addressable_impl() {
        let a = TestValue(42);
        let b = TestValue(42);
        let c = TestValue(99);
        assert_eq!(a.oid(), b.oid());
        assert_ne!(a.oid(), c.oid());
    }

    // --- Coincidence hash integration tests ---

    #[test]
    fn oid_hash_is_coincidence_detector() {
        // Oid::hash must use CoincidenceHash<3> (eigenvalue-based content addressing).
        // The canonical detector with N=3, dim=16, space="content" produces a
        // deterministic eigenvalue, then SHA-256 compresses it to 64 hex chars.
        let oid = Oid::hash(b"hello");
        let s = oid.as_str();
        // Fixed size: SHA-256 of eigenvalue = 32 bytes = 64 hex chars
        assert_eq!(s.len(), 64, "must be 64 hex chars");
        assert!(s.chars().all(|c| c.is_ascii_hexdigit()), "must be hex");
        // Deterministic
        assert_eq!(oid, Oid::hash(b"hello"));
        // Different input produces different hash
        assert_ne!(oid, Oid::hash(b"world"));
    }

    #[test]
    fn oid_hash_empty_bytes_fallback() {
        // Empty input may produce a zero state vector, triggering the SHA-256 fallback.
        // The fallback must still be deterministic and produce valid hex.
        let oid = Oid::hash(b"");
        let s = oid.as_str();
        assert_eq!(s.len(), 64, "fallback must produce 64 hex chars");
        assert!(s.chars().all(|c| c.is_ascii_hexdigit()));
        assert_eq!(oid, Oid::hash(b""), "fallback must be deterministic");
        assert_ne!(oid, Oid::dark(), "fallback must differ from dark");
    }

    #[test]
    fn oid_hash_cross_version_stable() {
        // Pin a known hash value to detect if the hash function changes accidentally.
        // This value is computed by CoincidenceHash<3> then SHA-256 compressed.
        // If this test fails, either the hash function changed or the pin needs updating.
        let oid = Oid::hash(b"prism");
        assert_eq!(
            oid.as_str(),
            "08f8e91d230c49a5072202e4e82db8306e226d83f77aa6f57d05dc87b56efc1e",
            "hash of b\"prism\" must match pinned CoincidenceHash<3> value"
        );
    }
}
