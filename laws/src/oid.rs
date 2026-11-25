//! Object identity. An [`Oid`] is the content address of a value inside a
//! [`Scope`]: two values with the same bytes in the same scope produce the
//! same Oid. Oids are the nodes in every graph this system builds:
//! content-addressed, comparable, hashable. The function is specified in
//! `docs/spec/identity.md`.

use std::sync::LazyLock;

use zeroize::Zeroize;

/// The context string of the public scope. Names the scope for the whole
/// stack; `v1` versions the identity function.
pub const PUBLIC_CONTEXT: &str = "systemic.engineering://visibility/public/v1";

static PUBLIC_KEY: LazyLock<[u8; 32]> = LazyLock::new(|| blake3::derive_key(PUBLIC_CONTEXT, b""));

/// The context of a scope's public id: `ScopeId = BLAKE3-derive-key(
/// SCOPE_ID_CONTEXT, K_scope)`. A derive-key context, so no content address
/// (a keyed hash) can ever equal a scope id.
pub const SCOPE_ID_CONTEXT: &str = "systemic.engineering://scope-id/v1";

/// The context of a slot's address: `Oid::slot(scope, label) =
/// BLAKE3-derive-key(SLOT_CONTEXT, K_scope ‖ label)`. A derive-key context of
/// its own, so a slot is never a content address (a keyed hash) or a scope
/// id (brain.md, "Slots").
pub const SLOT_CONTEXT: &str = "systemic.engineering://slot/v1";

/// A held scope's key: the holder's capability (prism-026 item 1).
///
/// prism holds no keys. The caller passes one in for the duration of a call;
/// prism never stores, derives, persists or logs it. So a `ScopeKey` is
/// **move-only** (no `Clone`, no `Copy`), never printed (`Debug` says
/// `ScopeKey(held)`, and there is no `Display`), and its bytes are **zeroed
/// when it drops**. [`ScopeKey::take`] is its only constructor, and there is
/// none for the public scope.
///
/// The control compiles: a key is taken, lent to a scope, and moved.
///
/// ```
/// use prism_laws::oid::{Oid, Scope, ScopeKey};
/// let key = ScopeKey::take(&mut [7; 32]).unwrap();
/// let _ = Oid::hash(&Scope::keyed(&key), b"x");
/// let moved = key;
/// let _ = format!("{moved:?}");
/// ```
///
/// A key can't be cloned:
///
/// ```compile_fail,E0599
/// use prism_laws::oid::{Oid, Scope, ScopeKey};
/// let key = ScopeKey::take(&mut [7; 32]).unwrap();
/// let _ = Oid::hash(&Scope::keyed(&key), b"x");
/// let moved = key.clone();
/// let _ = format!("{moved:?}");
/// ```
///
/// nor copied (a move leaves nothing behind):
///
/// ```compile_fail,E0382
/// use prism_laws::oid::{Oid, Scope, ScopeKey};
/// let key = ScopeKey::take(&mut [7; 32]).unwrap();
/// let _ = Oid::hash(&Scope::keyed(&key), b"x");
/// let moved = key;
/// let _ = format!("{key:?}");
/// ```
///
/// nor displayed:
///
/// ```compile_fail,E0277
/// use prism_laws::oid::{Oid, Scope, ScopeKey};
/// let key = ScopeKey::take(&mut [7; 32]).unwrap();
/// let _ = Oid::hash(&Scope::keyed(&key), b"x");
/// let moved = key;
/// let _ = format!("{moved}");
/// ```
///
/// nor built from its bytes except by taking them:
///
/// ```compile_fail,E0423
/// use prism_laws::oid::{Oid, Scope, ScopeKey};
/// let key = ScopeKey([7; 32]);
/// let _ = Oid::hash(&Scope::keyed(&key), b"x");
/// let moved = key;
/// let _ = format!("{moved:?}");
/// ```
///
/// and a keyed scope can't outlive its key:
///
/// ```compile_fail,E0505
/// use prism_laws::oid::{Oid, Scope, ScopeKey};
/// let key = ScopeKey::take(&mut [7; 32]).unwrap();
/// let scope = Scope::keyed(&key);
/// let moved = key;
/// let _ = Oid::hash(&scope, b"x");
/// ```
#[repr(transparent)]
pub struct ScopeKey([u8; 32]);

/// Zeroed on drop: `zeroize` writes the zeros with `write_volatile` and a
/// compiler fence, so the store isn't elided as dead. (prism-laws forbids
/// `unsafe`, so the volatile write is zeroize's, not hand-rolled here.)
impl Drop for ScopeKey {
    fn drop(&mut self) {
        self.0.zeroize();
    }
}

impl std::fmt::Debug for ScopeKey {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("ScopeKey(held)")
    }
}

/// The bytes offered as a key were `K_public`'s: anyone can derive them, so
/// they are nobody's capability, and there is no [`ScopeKey`] for them.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct IsPublic;

impl ScopeKey {
    /// Take a held key out of `source`, which is zeroed: the caller's copy
    /// doesn't outlive the handover. Refused, and `source` left as it was,
    /// if the bytes are the public scope's key.
    pub fn take(source: &mut [u8; 32]) -> Result<ScopeKey, IsPublic> {
        if *source == *PUBLIC_KEY {
            return Err(IsPublic);
        }
        let key = ScopeKey(*source);
        source.zeroize();
        Ok(key)
    }

    /// The scope's public id. Safe to store; it doesn't reveal the key.
    pub fn id(&self) -> ScopeId {
        ScopeId::of(&self.0)
    }
}

/// A scope's public id: `BLAKE3-derive-key(SCOPE_ID_CONTEXT, K_scope)`. It
/// is what a store, an erasure or a receipt holds for a scope: 32 bytes,
/// one-way, never the key.
#[derive(Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct ScopeId([u8; 32]);

impl ScopeId {
    fn of(key: &[u8; 32]) -> Self {
        ScopeId(blake3::derive_key(SCOPE_ID_CONTEXT, key))
    }

    /// The id's 32 bytes.
    pub fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }

    /// The id as an [`Oid`]: 64 lowercase hex characters. Not a content
    /// address: it is in no scope's keyed-hash domain.
    pub fn oid(&self) -> Oid {
        Oid(blake3::Hash::from(self.0).to_hex().to_string())
    }
}

impl std::fmt::Display for ScopeId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.oid().as_str())
    }
}

impl std::fmt::Debug for ScopeId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "ScopeId({})", &self.oid().as_str()[..12])
    }
}

/// The scope an address is taken in: a 32-byte BLAKE3 key.
///
/// [`Scope::public`] derives its key from [`PUBLIC_CONTEXT`], so anyone can
/// recompute a public address. [`Scope::keyed`] borrows a held
/// [`ScopeKey`]; where held keys come from is outside prism. A scope holds a
/// reference to its key, never a copy, so a keyed scope lives no longer than
/// its key. The key is never exposed, and `Debug` doesn't print it.
#[derive(Clone)]
pub struct Scope<'k> {
    key: &'k [u8; 32],
    public: bool,
}

impl Scope<'static> {
    /// The public scope: `K_public = BLAKE3-derive-key(PUBLIC_CONTEXT, "")`.
    pub fn public() -> Self {
        Scope {
            key: LazyLock::force(&PUBLIC_KEY),
            public: true,
        }
    }
}

impl<'k> Scope<'k> {
    /// A held scope, under the key it borrows.
    pub fn keyed(key: &'k ScopeKey) -> Self {
        Scope {
            key: &key.0,
            public: false,
        }
    }

    /// The scope's public id.
    pub fn id(&self) -> ScopeId {
        ScopeId::of(self.key)
    }
}

impl std::fmt::Debug for Scope<'_> {
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
pub struct Oid(String);

impl Oid {
    /// Wrap an existing string as an `Oid` without hashing. Use
    /// [`Self::hash`] to produce an Oid from raw bytes.
    pub fn new(s: impl Into<String>) -> Self {
        Oid(s.into())
    }

    /// The address of `bytes` in `scope`: `BLAKE3-keyed(K_scope, bytes)`,
    /// 64 lowercase hex characters. Every caller names its scope.
    pub fn hash(scope: &Scope<'_>, bytes: &[u8]) -> Self {
        Oid(blake3::keyed_hash(scope.key, bytes).to_hex().to_string())
    }

    /// The address of the slot `label` in `scope`: a named place for state
    /// that changes (a list that grows, a root that advances), which only
    /// the scope's key holder can compute. `BLAKE3-derive-key(SLOT_CONTEXT,
    /// K_scope ‖ label)`, 64 lowercase hex: one-way like a [`ScopeId`], and
    /// in neither the content domain nor the scope-id domain, so no crystal's
    /// bytes can land on a slot.
    pub fn slot(scope: &Scope<'_>, label: &[u8]) -> Self {
        let mut hasher = blake3::Hasher::new_derive_key(SLOT_CONTEXT);
        hasher.update(scope.key);
        hasher.update(label);
        Oid(hasher.finalize().to_hex().to_string())
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

    /// A throwaway key (synthetic bytes, never a real secret).
    fn throwaway(byte: u8) -> ScopeKey {
        ScopeKey::take(&mut [byte; 32]).expect("not the public key")
    }

    #[test]
    fn oid_hash_empty_bytes_is_not_dark() {
        assert_ne!(Oid::hash(&Scope::public(), b""), Oid::dark());
        let key = throwaway(0);
        assert_ne!(Oid::hash(&Scope::keyed(&key), b""), Oid::dark());
    }

    #[test]
    fn same_content_two_scopes_two_addresses() {
        let public = Oid::hash(&Scope::public(), b"hello");
        let key = throwaway(1);
        let held = Oid::hash(&Scope::keyed(&key), b"hello");
        assert_ne!(public, held);
    }

    #[test]
    fn scope_debug_does_not_print_the_key() {
        let key = throwaway(0xab);
        let shown = format!("{:?} {:?}", Scope::keyed(&key), Scope::public());
        assert_eq!(shown, "Scope(held) Scope(public)");
        assert!(!shown.contains("ab"));
        assert!(!shown.contains("171"));
    }

    #[test]
    fn scope_clone_keeps_the_key() {
        let key = throwaway(7);
        let scope = Scope::keyed(&key);
        assert_eq!(Oid::hash(&scope.clone(), b"x"), Oid::hash(&scope, b"x"));
    }

    // --- The key is the caller's capability (prism-026 item 1) ---

    #[test]
    fn a_key_never_prints_its_bytes() {
        let key = throwaway(0xab);
        let shown = format!("{key:?} {key:#?}");
        assert_eq!(shown, "ScopeKey(held) ScopeKey(held)");
        assert!(!shown.contains("ab"));
        assert!(!shown.contains("171"));
    }

    #[test]
    fn taking_a_key_zeroes_the_callers_copy() {
        let mut source = [0x5a; 32];
        let key = ScopeKey::take(&mut source).expect("not the public key");
        assert_eq!(source, [0; 32], "the caller's buffer is wiped");
        assert_eq!(
            Oid::hash(&Scope::keyed(&key), b"x"),
            Oid(blake3::keyed_hash(&[0x5a; 32], b"x").to_hex().to_string()),
            "the key kept the bytes it took"
        );
    }

    #[test]
    fn there_is_no_key_for_the_public_scope() {
        let mut public = *PUBLIC_KEY;
        assert_eq!(ScopeKey::take(&mut public).err(), Some(IsPublic));
        assert_eq!(public, *PUBLIC_KEY, "a refusal leaves the bytes alone");
        assert_eq!(format!("{IsPublic:?}"), "IsPublic");
    }

    #[test]
    fn a_scope_id_is_derived_one_way_from_its_key_never_the_key() {
        let key = throwaway(9);
        let id = key.id();
        // Byte for byte: BLAKE3's derive-key mode over the key, under the
        // scope-id context. Nothing else is in it.
        assert_eq!(
            id.as_bytes(),
            &blake3::derive_key("systemic.engineering://scope-id/v1", &[9; 32])
        );
        assert_eq!(
            std::mem::size_of::<ScopeId>(),
            32,
            "the id and nothing else"
        );
        assert_ne!(id.as_bytes(), &[9; 32]);
        assert_eq!(Scope::keyed(&key).id(), id, "the scope and its key agree");
        assert_ne!(Scope::public().id(), id);
        assert_eq!(
            Scope::public().id().as_bytes(),
            &blake3::derive_key(SCOPE_ID_CONTEXT, &*PUBLIC_KEY)
        );
        // Domain-separated from content: no keyed hash names it.
        assert_ne!(id.oid(), Oid::hash(&Scope::keyed(&key), b"scope"));
        assert_ne!(id.oid(), Oid::hash(&Scope::keyed(&key), id.as_bytes()));
    }

    #[test]
    fn a_slot_is_derived_one_way_in_its_own_domain() {
        let key = throwaway(9);
        let scope = Scope::keyed(&key);
        let slot = Oid::slot(&scope, b"manifest");
        // Byte for byte: derive-key over the key, then the label.
        let mut material = [9u8; 32].to_vec();
        material.extend_from_slice(b"manifest");
        assert_eq!(
            slot.as_str(),
            blake3::Hash::from(blake3::derive_key(SLOT_CONTEXT, &material))
                .to_hex()
                .as_str()
        );
        assert_eq!(slot.as_str().len(), 64);
        assert_eq!(slot, Oid::slot(&scope, b"manifest"), "deterministic");
        // Never a content address, never the scope id.
        assert_ne!(slot, Oid::hash(&scope, b"manifest"));
        assert_ne!(slot, key.id().oid());
        // Another label, another slot; another scope, another slot.
        assert_ne!(slot, Oid::slot(&scope, b"light"));
        let other = throwaway(10);
        assert_ne!(slot, Oid::slot(&Scope::keyed(&other), b"manifest"));
        assert_ne!(slot, Oid::slot(&Scope::public(), b"manifest"));
    }

    #[test]
    fn a_scope_id_prints_as_hex() {
        let id = throwaway(9).id();
        let hex = blake3::Hash::from(*id.as_bytes()).to_hex().to_string();
        assert_eq!(id.to_string(), hex);
        assert_eq!(id.oid().as_str(), hex);
        assert_eq!(format!("{id:?}"), format!("ScopeId({})", &hex[..12]));
    }
}
