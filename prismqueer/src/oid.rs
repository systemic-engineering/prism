//! Object identity. An [`Oid`] is the content address of a value inside a
//! [`Scope`]: two values with the same bytes in the same scope produce the
//! same Oid. Oids are the nodes in every graph this system builds:
//! content-addressed, comparable, hashable. The function is specified in
//! `docs/spec/identity.md`.

use std::sync::LazyLock;

/// The context string of the public scope. Names the scope for the whole
/// stack; `v1` versions the identity function.
pub const PUBLIC_CONTEXT: &str = "systemic.engineering://visibility/public/v1";

static PUBLIC_KEY: LazyLock<[u8; 32]> = LazyLock::new(|| blake3::derive_key(PUBLIC_CONTEXT, b""));

/// The scope an address is taken in: a 32-byte BLAKE3 key.
///
/// [`Scope::public`] derives its key from [`PUBLIC_CONTEXT`], so anyone can
/// recompute a public address. [`Scope::keyed`] takes a held key; where held
/// keys come from is outside prism. The key is never exposed, and `Debug`
/// doesn't print it.
#[derive(Clone)]
pub struct Scope {
    key: [u8; 32],
    public: bool,
}

impl Scope {
    /// The public scope: `K_public = BLAKE3-derive-key(PUBLIC_CONTEXT, "")`.
    pub fn public() -> Self {
        Scope {
            key: *PUBLIC_KEY,
            public: true,
        }
    }

    /// A held scope under a 32-byte key.
    pub fn keyed(key: [u8; 32]) -> Self {
        Scope { key, public: false }
    }
}

impl std::fmt::Debug for Scope {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(if self.public {
            "Scope(public)"
        } else {
            "Scope(held)"
        })
    }
}

/// Content address. The identity of a thing is its content, in a scope.
/// Two values with the same bytes in the same scope have the same Oid.
/// Oids are the nodes in every graph this system builds.
#[derive(Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
#[cfg_attr(feature = "pq", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "pq", serde(transparent))]
pub struct Oid(String);

impl Oid {
    /// Wrap an existing string as an `Oid` without hashing. Use
    /// [`Self::hash`] to produce an Oid from raw bytes.
    pub fn new(s: impl Into<String>) -> Self {
        Oid(s.into())
    }

    /// The address of `bytes` in `scope`: `BLAKE3-keyed(K_scope, bytes)`,
    /// 64 lowercase hex characters. Every caller names its scope.
    pub fn hash(scope: &Scope, bytes: &[u8]) -> Self {
        Oid(blake3::keyed_hash(&scope.key, bytes).to_hex().to_string())
    }

    /// The dark OID. The address of absence. Constant, and not the address
    /// of any content in any scope.
    pub fn dark() -> Self {
        Oid("0".repeat(64))
    }

    /// Borrow the underlying string form.
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

/// The thing has a content address. That's all.
pub trait Addressable {
    /// Return the content address — the [`Oid`] derived from this value.
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
        let a = Oid::hash(&Scope::public(), b"hello");
        let b = Oid::hash(&Scope::public(), b"hello");
        assert_eq!(a, b); // deterministic
    }

    #[test]
    fn oid_different_content_different_hash() {
        let a = Oid::hash(&Scope::public(), b"hello");
        let b = Oid::hash(&Scope::public(), b"world");
        assert_ne!(a, b);
    }

    #[test]
    fn oid_display_is_hex() {
        let oid = Oid::hash(&Scope::public(), b"test");
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
        assert_ne!(Oid::dark(), Oid::hash(&Scope::public(), b""));
        assert_ne!(Oid::dark(), Oid::hash(&Scope::public(), b"anything"));
    }

    struct TestValue(u32);

    impl Addressable for TestValue {
        fn oid(&self) -> Oid {
            Oid::hash(&Scope::public(), &self.0.to_le_bytes())
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

    // --- Scoped identity (docs/spec/identity-vectors.json) ---

    #[test]
    fn oid_hash_cross_version_stable() {
        // From identity-vectors.json, "public, prism" (computed with b3sum).
        assert_eq!(
            Oid::hash(&Scope::public(), b"prism").as_str(),
            "89514bceb709fad516e0273864a6f07b302d7142d7c6d53c4640e73b8d40b403"
        );
    }

    #[test]
    fn oid_hash_empty_bytes_is_not_dark() {
        assert_ne!(Oid::hash(&Scope::public(), b""), Oid::dark());
        assert_ne!(Oid::hash(&Scope::keyed([0; 32]), b""), Oid::dark());
    }

    #[test]
    fn same_content_two_scopes_two_addresses() {
        let public = Oid::hash(&Scope::public(), b"hello");
        let held = Oid::hash(&Scope::keyed([1; 32]), b"hello");
        assert_ne!(public, held);
    }

    #[test]
    fn scope_debug_does_not_print_the_key() {
        let key = [0xab; 32];
        let shown = format!("{:?} {:?}", Scope::keyed(key), Scope::public());
        assert_eq!(shown, "Scope(held) Scope(public)");
        assert!(!shown.contains("ab"));
        assert!(!shown.contains("171"));
    }

    #[test]
    fn scope_clone_keeps_the_key() {
        let scope = Scope::keyed([7; 32]);
        assert_eq!(Oid::hash(&scope.clone(), b"x"), Oid::hash(&scope, b"x"));
    }
}
