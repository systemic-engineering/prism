//! T4 of `fragmentation-mcp` — RED tests for [`SpectralUuid`].
//!
//! Per `/Users/alexwolf/dev/projects/mirror/docs/specs/reality-shard-as-crdt.md`.
//! `SpectralUuid` is the 128-bit content-addressed identifier with
//! navigable spectral structure: 48 bits ACTIVE (the quantized
//! `SpectralCoordinate<5>`; navigable) + 80 bits DARK (the BLAKE3-
//! truncated content hash; identity). The split is golden-ratio:
//! 48/128 ≈ 0.375 ≈ 1/φ² (the spec's leading-dim allocation).
//!
//! These tests pin the type's substrate contract:
//!
//! 1. The 16-byte newtype `SpectralUuid([u8; 16])` exists at the
//!    crate root.
//! 2. The `EMPTY` constant is byte-stable across calls (the canonical
//!    address of the empty shard — `⊥` per the CRDT spec §2).
//! 3. `from_parts(active, content_hash)` is deterministic: same
//!    inputs → same `SpectralUuid`.
//! 4. `active()` recovers the lower 48 bits passed in via `from_parts`.
//! 5. `dark()` recovers the first 10 bytes of the content hash.
//! 6. `to_string()` produces the standard UUID-hyphenated 36-char
//!    form; `parse()` round-trips it byte-for-byte.
//! 7. Two `SpectralUuid`s built from the same content hash share
//!    their dark portion (the identity property).
//!
//! Substrate-pull: `[substrate-pull:realize]` — prismqueer stays
//! deps-free. The BLAKE3 computation lives in fragmentation; this
//! crate takes the hash bytes as an opaque parameter. The 48-bit
//! quantization of `SpectralCoordinate<5>` lives at fragmentation's
//! boundary; this crate takes a pre-quantized `u64` (lower 48 bits).

use prismqueer::SpectralUuid;

// ---------------------------------------------------------------------------
// EMPTY — the bottom of the semilattice.
// ---------------------------------------------------------------------------

#[test]
fn empty_is_byte_stable() {
    // The CRDT spec's §2: `fixed empty` is THE canonical address.
    // Two reads must produce identical bytes. The constant must be
    // compile-time-evaluable (the `const EMPTY: Self = ...` form
    // in the spec).
    let a = SpectralUuid::EMPTY;
    let b = SpectralUuid::EMPTY;
    assert_eq!(a, b);
    assert_eq!(a.as_bytes(), b.as_bytes());
}

#[test]
fn empty_active_is_zero() {
    // λ₀ = 0 — the void axis is the origin in spectral space.
    assert_eq!(SpectralUuid::EMPTY.active(), 0);
}

#[test]
fn empty_dark_is_blake3_of_empty_input() {
    // BLAKE3 of empty input is well-known and stable across all
    // BLAKE3 implementations:
    //   af1349b9f5f9a1a6a0404dea36dcc9499bcb25c9adc112b7cc9a93cae41f3262
    // First 10 bytes = af1349b9f5f9a1a6a040.
    let expected: [u8; 10] = [0xaf, 0x13, 0x49, 0xb9, 0xf5, 0xf9, 0xa1, 0xa6, 0xa0, 0x40];
    assert_eq!(SpectralUuid::EMPTY.dark(), expected);
}

// ---------------------------------------------------------------------------
// from_parts — derive from a quantized spectral coord + content hash.
// ---------------------------------------------------------------------------

#[test]
fn from_parts_is_deterministic() {
    let active: u64 = 0x0001_2345_6789_ABCD; // 48 bits used; upper 16 ignored
    let hash: [u8; 32] = [
        0x11, 0x22, 0x33, 0x44, 0x55, 0x66, 0x77, 0x88, 0x99, 0xaa, 0xbb, 0xcc, 0xdd, 0xee, 0xff,
        0x00, 0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08, 0x09, 0x0a, 0x0b, 0x0c, 0x0d, 0x0e,
        0x0f, 0x10,
    ];
    let a = SpectralUuid::from_parts(active, &hash);
    let b = SpectralUuid::from_parts(active, &hash);
    assert_eq!(a, b);
    assert_eq!(a.as_bytes(), b.as_bytes());
}

#[test]
fn from_parts_recovers_active_bits() {
    let active: u64 = 0x0000_DEAD_BEEF_CAFE; // 48 bits (upper 16 zeros)
    let hash = [0u8; 32];
    let u = SpectralUuid::from_parts(active, &hash);
    assert_eq!(u.active(), active & 0x0000_FFFF_FFFF_FFFF);
}

#[test]
fn from_parts_truncates_active_to_48_bits() {
    // Upper 16 bits must be ignored by from_parts so the
    // homomorphism is well-defined on the 48-bit subspace.
    let with_high_bits: u64 = 0xAAAA_DEAD_BEEF_CAFE;
    let without_high: u64 = 0x0000_DEAD_BEEF_CAFE;
    let hash = [0u8; 32];
    let a = SpectralUuid::from_parts(with_high_bits, &hash);
    let b = SpectralUuid::from_parts(without_high, &hash);
    assert_eq!(a, b);
    assert_eq!(a.active(), 0x0000_DEAD_BEEF_CAFE);
}

#[test]
fn from_parts_recovers_dark_bytes() {
    let active: u64 = 0;
    let mut hash = [0u8; 32];
    // First 10 bytes are the dark portion; the rest must be ignored.
    for (i, byte) in hash.iter_mut().enumerate() {
        *byte = i as u8;
    }
    let u = SpectralUuid::from_parts(active, &hash);
    let expected: [u8; 10] = [0, 1, 2, 3, 4, 5, 6, 7, 8, 9];
    assert_eq!(u.dark(), expected);
}

#[test]
fn from_parts_same_hash_same_dark() {
    // The identity property: two SpectralUuids derived from the same
    // content_hash share their dark portion regardless of active.
    let hash = [0x42u8; 32];
    let a = SpectralUuid::from_parts(0, &hash);
    let b = SpectralUuid::from_parts(0xFFFF_FFFF_FFFF, &hash);
    assert_eq!(a.dark(), b.dark());
    assert_ne!(a.active(), b.active());
    assert_ne!(a, b); // different active → different uuid overall
}

// ---------------------------------------------------------------------------
// to_string / parse — wire-altitude string form.
// ---------------------------------------------------------------------------

#[test]
fn to_string_is_36_chars_hyphenated() {
    // Standard UUID form: 8-4-4-4-12 hex chars with hyphens.
    // Total: 32 hex + 4 hyphens = 36.
    let u = SpectralUuid::EMPTY;
    let s = u.to_string();
    assert_eq!(s.len(), 36, "got: {s}");
    assert_eq!(s.as_bytes()[8], b'-');
    assert_eq!(s.as_bytes()[13], b'-');
    assert_eq!(s.as_bytes()[18], b'-');
    assert_eq!(s.as_bytes()[23], b'-');
    // All non-hyphen chars are lowercase hex.
    for (i, c) in s.chars().enumerate() {
        if matches!(i, 8 | 13 | 18 | 23) {
            continue;
        }
        let is_hex = c.is_ascii_hexdigit();
        let is_lower_ok = !c.is_ascii_alphabetic() || c.is_ascii_lowercase();
        assert!(
            is_hex && is_lower_ok,
            "non-hex or uppercase char `{c}` at position {i} in `{s}`"
        );
    }
}

#[test]
fn parse_round_trips_to_string() {
    let hash: [u8; 32] = {
        let mut h = [0u8; 32];
        for (i, byte) in h.iter_mut().enumerate() {
            *byte = (i as u8).wrapping_mul(7);
        }
        h
    };
    let orig = SpectralUuid::from_parts(0x0001_2345_6789_ABCD, &hash);
    let s = orig.to_string();
    let parsed = SpectralUuid::parse(&s).expect("parse round-trip");
    assert_eq!(orig, parsed);
    assert_eq!(orig.as_bytes(), parsed.as_bytes());
    assert_eq!(parsed.to_string(), s);
}

#[test]
fn parse_rejects_garbage() {
    assert!(SpectralUuid::parse("").is_err());
    assert!(SpectralUuid::parse("not-a-uuid").is_err());
    assert!(SpectralUuid::parse("12345").is_err());
    // 35 chars (one too few)
    assert!(SpectralUuid::parse("00000000-0000-0000-0000-00000000000").is_err());
    // 37 chars (one too many)
    assert!(SpectralUuid::parse("00000000-0000-0000-0000-0000000000000").is_err());
    // Non-hex char
    assert!(SpectralUuid::parse("0000000z-0000-0000-0000-000000000000").is_err());
    // Missing hyphens (32 hex chars but no hyphens)
    assert!(SpectralUuid::parse("00000000000000000000000000000000").is_err());
}

#[test]
fn parse_empty_round_trips() {
    let s = SpectralUuid::EMPTY.to_string();
    let parsed = SpectralUuid::parse(&s).expect("parse EMPTY");
    assert_eq!(parsed, SpectralUuid::EMPTY);
}

// ---------------------------------------------------------------------------
// Layout sanity — active is leading, dark is trailing.
// ---------------------------------------------------------------------------

#[test]
fn active_lives_in_the_leading_bytes() {
    // The 48-bit active portion is leading per the CRDT spec
    // (navigable; sorted/scanned first). Two values whose active
    // differs but dark matches must differ in the first 6 bytes.
    let hash = [0u8; 32];
    let a = SpectralUuid::from_parts(0x0000_0000_0000_0000, &hash);
    let b = SpectralUuid::from_parts(0x0000_FFFF_FFFF_FFFF, &hash);
    let abytes = a.as_bytes();
    let bbytes = b.as_bytes();
    // The first 6 bytes (48 bits) must differ; the last 10 must match.
    assert_ne!(&abytes[..6], &bbytes[..6]);
    assert_eq!(&abytes[6..], &bbytes[6..]);
}

#[test]
fn dark_lives_in_the_trailing_bytes() {
    // Two values whose dark differs but active matches must differ
    // only in the last 10 bytes.
    let h1 = [0x00u8; 32];
    let h2 = {
        let mut h = [0u8; 32];
        h[0] = 0xFF;
        h
    };
    let a = SpectralUuid::from_parts(42, &h1);
    let b = SpectralUuid::from_parts(42, &h2);
    let abytes = a.as_bytes();
    let bbytes = b.as_bytes();
    assert_eq!(&abytes[..6], &bbytes[..6]); // active matches
    assert_ne!(&abytes[6..], &bbytes[6..]); // dark differs
}
