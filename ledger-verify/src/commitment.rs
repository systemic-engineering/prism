//! Halevi–Micali commitments over SHA-256, k = 256: opening checks only.
//!
//! S. Halevi and S. Micali, "Practical and Provably-Secure Commitment
//! Schemes from Collision-Free Hashing", CRYPTO '96. Page numbers are those
//! of `shaih.github.io/pubs/hm96.pdf`. MD is SHA-256 and k = 256 (Alex,
//! 2026-11-17). The scheme, as the paper builds it:
//!
//! - **§4 (p.15):** to commit to m, compute s = MD(m), a k-bit string, and
//!   commit to s with the first scheme. So that scheme's message length is
//!   n = k, and the commitment's length doesn't depend on |m|.
//! - **§3 (p.11):** L = 4k + 2n + 4. Pick r ∈ {0,1}^L at random, y = MD(r),
//!   then a random h from a universal family with h(r) = s. The commit-string
//!   is c = ⟨h, y⟩; the de-commit string is r.
//! - **§2.1 (p.9):** the family is h_{A,b}(r) = A r + b over GF(2), with A an
//!   n × L Toeplitz matrix ("fixed on the diagonals"), so h takes
//!   2n + L − 1 bits.
//! - **Footnote 4 (p.11):** a random h with h(r) = s is a random A and
//!   b = s − A r.
//! - **Receive (§3 p.11, §4 p.15):** check y = MD(r), compute h(r), check
//!   that it is the committed s and that MD(m) = s.
//!
//! **Guarantees.** Hiding: Theorem 1 (p.12; proof pp.12–15),
//! ‖C_k(m₁) − C_k(m₂)‖ < 2^−k for *every* MD: information-theoretic, so no
//! computer, quantum or not, changes it. Binding: two openings to different
//! messages give a SHA-256 collision (p.12; §4's step "omitted from this
//! extended abstract", p.15, is one line).
//!
//! **prism's encodings** (the paper works on bit strings; docs/spec/ledger.md
//! §5 has them, and they are provisional):
//! - bit strings are packed most significant bit first, zero-padded;
//! - MD on {0,1}^L is SHA-256 of r's 193-byte packing, which is injective,
//!   so a collision of it is a SHA-256 collision; the hiding proof holds
//!   for any function MD, so it holds for this one;
//! - A\[i\]\[j\] = t\[i − j + L − 1\], with t the n + L − 1 diagonal bits;
//! - commitment = [`SCHEME`] ‖ t (225 bytes) ‖ b (32) ‖ y (32): 290 bytes.
//!   The paper's "9k" (p.15) is 9k + 3 bits exactly, which with byte
//!   alignment and the scheme byte is 290, not 288;
//! - opening = r (193 bytes), with m given beside it.
//!
//! Erasure deletes r and m. The commitment alone is then within 2^−256 of
//! independent of m.

use sha2::{Digest, Sha256};

/// The security parameter k: SHA-256's output length, and the hiding bound
/// 2^−k. One k for both (Remark 1, p.11, allows two).
pub const K: usize = 256;
/// The first scheme's message length n: §4 (p.15) applies it to s = MD(m).
pub const N: usize = K;
/// L = 4k + 2n + 4 (§3, p.11): |r| = 6k + 4 = 1540 bits.
pub const L: usize = 4 * K + 2 * N + 4;
/// The n × L Toeplitz matrix's diagonals: n + L − 1 = 1795 bits (§2.1, p.9).
pub const DIAGONALS: usize = N + L - 1;
/// The scheme byte: Halevi–Micali, SHA-256, k = 256, this layout.
/// **Provisional**: no registry assigns it; it lets a leaf name the scheme
/// it is checked under (prism-019).
pub const SCHEME: u8 = 0x01;
/// The commitment's length in bytes: 1 + 225 + 32 + 32.
pub const COMMITMENT: usize = 1 + T_BYTES + N / 8 + K / 8;
/// The opening's length in bytes: r, 193.
pub const OPENING: usize = L.div_ceil(8);

const T_BYTES: usize = DIAGONALS.div_ceil(8);
const T: core::ops::Range<usize> = 1..1 + T_BYTES;
const B: core::ops::Range<usize> = T.end..T.end + N / 8;
const Y: core::ops::Range<usize> = B.end..B.end + K / 8;

/// Why an opening did not verify.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OpeningError {
    /// The commitment or the opening has the wrong length.
    Length,
    /// The commitment names another scheme.
    Scheme,
    /// A pad bit of t or of r is set: not the canonical encoding.
    Padding,
    /// y ≠ MD(r) (§3, p.11).
    Randomness,
    /// h(r) ≠ MD(m) (§4, p.15).
    Message,
}

/// Check an opening (m, r) against a commitment: Receive (§3 p.11, §4 p.15).
///
/// `Ok(())` iff the lengths are exact, the scheme byte is [`SCHEME`], the pad
/// bits are zero, y = SHA-256(r) and A r + b = SHA-256(m).
pub fn verify_opening(
    commitment: &[u8],
    message: &[u8],
    opening: &[u8],
) -> Result<(), OpeningError> {
    if commitment.len() != COMMITMENT || opening.len() != OPENING {
        return Err(OpeningError::Length);
    }
    if commitment[0] != SCHEME {
        return Err(OpeningError::Scheme);
    }
    let t = &commitment[T];
    if !pad_is_zero(t, DIAGONALS) || !pad_is_zero(opening, L) {
        return Err(OpeningError::Padding);
    }
    // §3 (p.11): "the Receiver ... verifies that y = MD(r)".
    let y: [u8; 32] = Sha256::digest(opening).into();
    if y[..] != commitment[Y] {
        return Err(OpeningError::Randomness);
    }
    // §3 (p.11): "and computes m = h(r)"; §4 (p.15): "checks that s is the
    // string being committed to ... and that MD(m) = s".
    let s: [u8; 32] = Sha256::digest(message).into();
    if apply(t, &commitment[B], opening) != s {
        return Err(OpeningError::Message);
    }
    Ok(())
}

/// The bits of `bytes` past the first `bits` are all zero.
fn pad_is_zero(bytes: &[u8], bits: usize) -> bool {
    let used = bits % 8;
    used == 0 || bytes[bits / 8] & (0xff >> used) == 0
}

/// h_{A,b}(r) = A r + b over GF(2) (§2.1, p.9), with A[i][j] = t[i − j + L − 1].
///
/// Column j of A is t[L − 1 − j .. L − 1 − j + n], read downwards, so A r is
/// the XOR of the n-bit windows of t at offsets L − 1 − j for every j with
/// r_j = 1.
fn apply(t: &[u8], b: &[u8], r: &[u8]) -> [u8; 32] {
    let mut acc: [u8; 32] = b.try_into().unwrap_or([0; 32]);
    for j in 0..L {
        if (r[j / 8] >> (7 - j % 8)) & 1 == 1 {
            let window = window(t, L - 1 - j);
            for (a, w) in acc.iter_mut().zip(window) {
                *a ^= w;
            }
        }
    }
    acc
}

/// The 256 bits of `t` starting at bit `offset`, MSB first. Needs
/// `offset / 8 + 32 < t.len()` when `offset % 8 ≠ 0`; the largest offset is
/// L − 1 = 1539, which reads up to byte 224 of t's 225.
fn window(t: &[u8], offset: usize) -> [u8; 32] {
    let (byte, shift) = (offset / 8, offset % 8);
    core::array::from_fn(|q| {
        if shift == 0 {
            t[byte + q]
        } else {
            (t[byte + q] << shift) | (t[byte + q + 1] >> (8 - shift))
        }
    })
}
