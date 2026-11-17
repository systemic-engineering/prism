//! Halevi–Micali commitments over SHA-256, k = 256: opening checks only.
//!
//! RED STUB: every opening is refused as the wrong length.

/// The security parameter k: SHA-256's output length, and the hiding bound 2^−k.
pub const K: usize = 256;
/// The first scheme's message length n.
pub const N: usize = K;
/// L = 4k + 2n + 4.
pub const L: usize = 4 * K + 2 * N + 4;
/// The Toeplitz matrix's diagonals: n + L − 1 bits.
pub const DIAGONALS: usize = N + L - 1;
/// The scheme byte (provisional).
pub const SCHEME: u8 = 0x01;
/// The commitment's length in bytes.
pub const COMMITMENT: usize = 1 + DIAGONALS.div_ceil(8) + N / 8 + K / 8;
/// The opening's length in bytes.
pub const OPENING: usize = L.div_ceil(8);

/// Why an opening did not verify.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OpeningError {
    /// The commitment or the opening has the wrong length.
    Length,
    /// The commitment names another scheme.
    Scheme,
    /// A pad bit is set.
    Padding,
    /// y ≠ MD(r).
    Randomness,
    /// h(r) ≠ MD(m).
    Message,
}

/// Check an opening (m, r) against a commitment.
pub fn verify_opening(
    _commitment: &[u8],
    _message: &[u8],
    _opening: &[u8],
) -> Result<(), OpeningError> {
    Err(OpeningError::Length)
}
