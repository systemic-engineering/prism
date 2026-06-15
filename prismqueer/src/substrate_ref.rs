//! Substrate reference (`@`-prefixed nav-ref).
//!
//! `Ref` is a universal substrate-reference primitive — the path-shaped
//! type any prism consumer uses to name a substrate location. The name is
//! drawn from mirror's nav-ref vocabulary (`.`, `..`, `...`, `~`, `@`,
//! `^`, `HEAD`); `@`-prefixed refs name substrate actions, paths, or
//! addressable locations (e.g. `@kintsugi/fracture/rename`,
//! `@quantize`, `@cli/new`).
//!
//! ## Hoisting from mirror
//!
//! `Ref` previously lived in `mirror::bootstrap::crystallize` and was
//! mirror-specific. It moved to `prismqueer` per the
//! `[substrate-pull:realize]` discipline: every consumer of `prismqueer`
//! (mirror, cosmos-mirror, spectral-db, future engines) needs the same
//! `@`-prefixed substrate reference. The validating constructor stays
//! verbatim — non-empty, `@`-prefixed, no whitespace.
//!
//! ## No `Default`
//!
//! There is deliberately no `Default` impl. An "empty" `@`-prefixed ref
//! is meaningless. The structural payoff: [`Transparency<Ref>`] does not
//! require `Ref : Default` because `Transparency`'s identity element is
//! structural (the `Clear` variant), not synthesised from `P::default()`.
//!
//! ## Validator hardening
//!
//! The constructor rejects empty input, missing `@` prefix, the bare
//! `@` (no path), ASCII whitespace AND the full control-character range
//! (`char::is_control()` — C0 + DEL + C1). Per Seam I2 (pre-merge
//! adversarial review, 2026-05-30): the validator IS the hardening
//! boundary; it must reject obvious shenanigans (terminal escape
//! sequences, null bytes, DEL) even when no downstream consumer is
//! exploitable today.
//!
//! [`Transparency<Ref>`]: terni::Transparency

/// A substrate reference, like `"@kintsugi/fracture/rename"`. Newtype
/// with `@`-prefix, non-empty, no-whitespace validation at construction.
///
/// Hash-blind by design; carries no OID, performs no hash computation.
/// Suitable as a `BTreeMap` key (derives `Ord`) and as a `HashMap` key
/// (derives `Hash`). No `Default` — see the module docs.
#[derive(Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Ref(String);

impl Ref {
    /// Construct. Returns `Err` if `path`:
    ///
    /// - is empty;
    /// - has no path after the `@` prefix (`"@"` alone is meaningless);
    /// - lacks a `@` prefix;
    /// - contains whitespace; or
    /// - contains a control character (the C0 range U+0000..=U+001F
    ///   plus U+007F, and the C1 range U+0080..=U+009F).
    ///
    /// The control-character rejection is defence-in-depth per Seam I2
    /// (pre-merge adversarial review, 2026-05-30). `char::is_whitespace`
    /// catches ASCII whitespace but leaves the rest of the C0 range
    /// open; a `Ref("@evil\x1b[2J")` could carry a terminal-clear
    /// escape into any `Display`/`Debug` consumer. The validator IS
    /// the boundary and must reject obvious shenanigans even if no
    /// downstream consumer is exploitable today.
    pub fn new(path: impl Into<String>) -> Result<Self, &'static str> {
        let s = path.into();
        if s.is_empty() {
            return Err("Ref must be non-empty");
        }
        if !s.starts_with('@') {
            return Err("Ref must start with '@'");
        }
        if s.len() <= 1 {
            return Err("Ref must have a path after '@'");
        }
        if s.chars().any(|c| c.is_whitespace()) {
            return Err("Ref must not contain whitespace");
        }
        if s.chars().any(|c| c.is_control()) {
            return Err("Ref must not contain control characters");
        }
        Ok(Ref(s))
    }

    /// Borrow the underlying `&str`.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}
