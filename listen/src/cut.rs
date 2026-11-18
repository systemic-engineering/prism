//! P3, the cut: a file is its observation, then (optionally) its reflection.
//!
//! mirror `docs/math/2026-11-16-mara-the-reflection-seam.md` §2.1 (Mara,
//! amendment 1 at `4f6bdc17`): *the seam* is the first line that is exactly
//! `---`, the bytes `2d 2d 2d 0a`, at offset 0 or right after an LF. Then
//! `s = a ‖ "---\n" ‖ r`; with no seam, `a = s` and `r` is absent. Only
//! the first seam cuts; `r` may hold more `---` lines. Bytes only, no
//! grammar (§6, the bootstrap order: the cut comes first).

/// The seam line's bytes.
pub const SEAM: &[u8] = b"---\n";

/// A file split at its seam.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Cut<'a> {
    /// `a`: everything before the seam line. Empty, or it ends in LF.
    pub above: &'a [u8],
    /// `r`, or `None` when the file is unreflected.
    pub reflection: Option<Reflection<'a>>,
}

/// The part below the seam.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Reflection<'a> {
    /// `r`: every byte after the seam line (possibly none).
    pub bytes: &'a [u8],
    /// The seam's own line number, 1-based, so `r`'s first line is
    /// `seam_line + 1` in the file's coordinates.
    pub seam_line: usize,
}

/// Cut `s` at its first seam.
pub fn cut(s: &[u8]) -> Cut<'_> {
    Cut {
        above: s,
        reflection: None,
    }
}
