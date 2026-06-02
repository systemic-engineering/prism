//! `SpectralUuid` — the 128-bit content-addressed identifier with
//! navigable spectral structure.
//!
//! Per `/Users/alexwolf/dev/projects/mirror/docs/specs/reality-shard-as-crdt.md`.
//! `SpectralUuid` is the substrate-pull-honest realisation of the
//! shard's algebraic identity: 48 bits ACTIVE (the quantized
//! `SpectralCoordinate<5>`; leading; navigable) + 80 bits DARK
//! (the BLAKE3-truncated content hash; trailing; identity). The
//! 48/128 split is the golden-ratio allocation (≈1/φ²) the spec
//! reserves for the navigable dim.
//!
//! # The monoid homomorphism
//!
//! ```text
//! SpectralUuid(merge(a, b)) = combine(SpectralUuid(a), SpectralUuid(b))
//! ```
//!
//! T4 of fragmentation-mcp lands only the carrier + the `EMPTY`
//! constant (the semilattice's bottom). The `combine` function on
//! the 128-bit space is defined by the spec §3 but its quantized
//! 48-bit arithmetic is not pinned yet (§11 open question 1); the
//! arithmetic lands when SpectralCoordinate's eigenvalue
//! composition rules quantize cleanly. Until then, `from_parts`
//! re-derives the identifier from fresh inputs and the `EMPTY`
//! constant carries the bottom-element semantics.
//!
//! # Substrate-pull
//!
//! `[substrate-pull:realize]` — prism_core stays deps-free. The
//! BLAKE3 computation lives in fragmentation; this crate takes the
//! hash bytes as an opaque `&[u8; 32]` parameter. Quantization of
//! `SpectralCoordinate<5>` into 48 bits lives at fragmentation's
//! boundary; this crate takes a pre-quantized `u64` (lower 48 bits
//! used; upper 16 bits ignored).
//!
//! # Layout
//!
//! 16 bytes, big-endian for the active portion to preserve
//! lexicographic sort order on the navigable prefix (range scans
//! on the 48-bit space compose with byte-wise sort):
//!
//! ```text
//! byte index:  0 1 2 3 4 5  6 7 8 9 10 11 12 13 14 15
//!              [ active 48b ] [   dark 80 bits         ]
//! ```
//!
//! The standard UUID hyphenated form (8-4-4-4-12) layers over this
//! layout directly: positions 4, 6, 8, 10 carry the hyphens.

use std::fmt;

/// Content-addressed identifier with navigable spectral structure.
///
/// 128 bits, golden-ratio split:
///   48 bits ACTIVE — quantized `SpectralCoordinate<5>` (leading; navigable)
///   80 bits DARK   — BLAKE3-truncated content hash (trailing; identity)
///
/// `SpectralUuid` is a monoid homomorphism w.r.t. shard merge
/// (per the CRDT spec §3). The [`Self::EMPTY`] constant is the
/// canonical address of the empty shard (`fixed empty` per
/// `@mirror/reality/shard`) — the bottom element of the semilattice,
/// at λ₀ = 0 in spectral space and the BLAKE3-of-empty-input hash
/// in content space.
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct SpectralUuid([u8; 16]);

impl SpectralUuid {
    /// The canonical empty-shard address. Bottom of the lattice.
    ///
    /// - Active = 0 (origin in spectral space; λ₀ = 0, the void axis).
    /// - Dark = first 10 bytes of BLAKE3 of empty input, the
    ///   well-known constant `af1349b9f5f9a1a6a040`. Stable across
    ///   all BLAKE3 implementations.
    ///
    /// Compile-time-evaluable: usable in `const` contexts and in
    /// pattern matches (via `Eq`).
    pub const EMPTY: Self = SpectralUuid([
        // active (6 bytes; all zero — λ₀ = 0)
        0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        // dark (10 bytes; first 10 of BLAKE3 of empty input)
        0xaf, 0x13, 0x49, 0xb9, 0xf5, 0xf9, 0xa1, 0xa6, 0xa0, 0x40,
    ]);

    /// Derive a `SpectralUuid` from the quantized spectral active
    /// portion and an opaque content hash.
    ///
    /// - `active` carries the 48-bit quantized `SpectralCoordinate<5>`
    ///   in its lower 48 bits. Upper 16 bits are ignored — the
    ///   homomorphism is well-defined only on the 48-bit subspace.
    /// - `content_hash` carries the 32-byte content address (the
    ///   spec assumes BLAKE3; this crate is hash-agnostic). The
    ///   first 10 bytes form the dark portion; the rest is ignored.
    pub fn from_parts(active: u64, content_hash: &[u8; 32]) -> Self {
        let active_48 = active & 0x0000_FFFF_FFFF_FFFF;
        let active_be = active_48.to_be_bytes(); // 8 bytes, big-endian
        let mut bytes = [0u8; 16];
        // active_be[0..2] are the two zero high bytes; active_be[2..8]
        // is the 48-bit active portion, big-endian.
        bytes[0..6].copy_from_slice(&active_be[2..8]);
        bytes[6..16].copy_from_slice(&content_hash[0..10]);
        SpectralUuid(bytes)
    }

    /// Construct from raw bytes. Used by [`Self::parse`] and by
    /// callers that already have a canonical 16-byte representation.
    pub const fn from_bytes(bytes: [u8; 16]) -> Self {
        SpectralUuid(bytes)
    }

    /// Borrow the raw 16-byte representation.
    pub const fn as_bytes(&self) -> &[u8; 16] {
        &self.0
    }

    /// The 48-bit active portion as a `u64` (lower 48 bits).
    ///
    /// This is the quantized `SpectralCoordinate<5>` — the
    /// navigable coordinate in spectral space.
    pub fn active(&self) -> u64 {
        let mut be = [0u8; 8];
        be[2..8].copy_from_slice(&self.0[0..6]);
        u64::from_be_bytes(be)
    }

    /// The 80-bit dark portion as a 10-byte array.
    ///
    /// This is the content-hash prefix — the identity component.
    pub fn dark(&self) -> [u8; 10] {
        let mut out = [0u8; 10];
        out.copy_from_slice(&self.0[6..16]);
        out
    }

    /// Parse from the standard UUID-hyphenated 36-char form
    /// (e.g. `00000000-0000-0000-0000-000000000000`).
    pub fn parse(s: &str) -> Result<Self, ParseError> {
        let bytes = s.as_bytes();
        if bytes.len() != 36 {
            return Err(ParseError::WrongLength(bytes.len()));
        }
        // Hyphens at positions 8, 13, 18, 23.
        for pos in [8usize, 13, 18, 23] {
            if bytes[pos] != b'-' {
                return Err(ParseError::MissingHyphen(pos));
            }
        }
        let mut out = [0u8; 16];
        let hex_positions: [(usize, usize); 16] = [
            (0, 2),
            (2, 4),
            (4, 6),
            (6, 8),
            (9, 11),
            (11, 13),
            (14, 16),
            (16, 18),
            (19, 21),
            (21, 23),
            (24, 26),
            (26, 28),
            (28, 30),
            (30, 32),
            (32, 34),
            (34, 36),
        ];
        for (i, (lo, hi)) in hex_positions.iter().enumerate() {
            let hi_n = hex_digit(bytes[*lo]).ok_or(ParseError::NonHexDigit(*lo))?;
            let lo_n = hex_digit(bytes[*lo + 1]).ok_or(ParseError::NonHexDigit(*lo + 1))?;
            debug_assert_eq!(*hi, *lo + 2);
            out[i] = (hi_n << 4) | lo_n;
        }
        Ok(SpectralUuid(out))
    }
}

impl fmt::Display for SpectralUuid {
    /// Standard UUID hyphenated form (lowercase hex, 8-4-4-4-12).
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // The standard UUID positions split the 16 bytes as 4-2-2-2-6.
        // Hand-write the hex to keep prism_core deps-free.
        let b = &self.0;
        write!(
            f,
            "{:02x}{:02x}{:02x}{:02x}-{:02x}{:02x}-{:02x}{:02x}-{:02x}{:02x}-{:02x}{:02x}{:02x}{:02x}{:02x}{:02x}",
            b[0], b[1], b[2], b[3],
            b[4], b[5],
            b[6], b[7],
            b[8], b[9],
            b[10], b[11], b[12], b[13], b[14], b[15],
        )
    }
}

impl fmt::Debug for SpectralUuid {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "SpectralUuid({self})")
    }
}

/// Parse error for [`SpectralUuid::parse`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ParseError {
    /// Input was not 36 bytes long.
    WrongLength(usize),
    /// A required hyphen was missing at the given byte position.
    MissingHyphen(usize),
    /// A non-hex digit appeared at the given byte position.
    NonHexDigit(usize),
}

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ParseError::WrongLength(n) => {
                write!(f, "SpectralUuid::parse: wrong length {n} (expected 36)")
            }
            ParseError::MissingHyphen(pos) => {
                write!(f, "SpectralUuid::parse: missing hyphen at position {pos}")
            }
            ParseError::NonHexDigit(pos) => {
                write!(f, "SpectralUuid::parse: non-hex digit at position {pos}")
            }
        }
    }
}

impl std::error::Error for ParseError {}

/// Map a single ASCII byte to its hex-digit value, or `None` if
/// it's not a hex digit. Lowercase + uppercase both accepted on
/// parse; output is always lowercase.
fn hex_digit(b: u8) -> Option<u8> {
    match b {
        b'0'..=b'9' => Some(b - b'0'),
        b'a'..=b'f' => Some(b - b'a' + 10),
        b'A'..=b'F' => Some(b - b'A' + 10),
        _ => None,
    }
}

// ---------------------------------------------------------------------------
// Module-local sanity tests.
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_is_const() {
        // The const-evaluation check: a static binding of EMPTY
        // must compile (proves `const EMPTY: Self = ...` parsed).
        const _E: SpectralUuid = SpectralUuid::EMPTY;
    }

    #[test]
    fn hex_digit_round_trips() {
        for n in 0u8..=15 {
            let c = if n < 10 { b'0' + n } else { b'a' + n - 10 };
            assert_eq!(hex_digit(c), Some(n));
        }
        assert_eq!(hex_digit(b' '), None);
        assert_eq!(hex_digit(b'g'), None);
    }

    #[test]
    fn display_is_36_chars() {
        let s = format!("{}", SpectralUuid::EMPTY);
        assert_eq!(s.len(), 36);
    }
}
