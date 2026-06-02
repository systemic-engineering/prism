//! `pq::Reference` — the wire-layer named-ref type.
//!
//! Distinct from [`crate::Ref`] which models substrate `@module` refs.
//! The pq wire carries git-shaped ref names (HEAD, main, feature/x)
//! per pq spec §5.1; this newtype is what those deserialize into.

use serde::{Deserialize, Serialize};

/// A wire-layer reference name.
///
/// Validation is minimal: reject empty strings and strings containing
/// ASCII control characters. Beyond that, what counts as a "valid ref"
/// is a wire-policy decision the consumer makes — pq is shape, not
/// policy.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Reference(String);

impl Reference {
    /// Construct a reference. Infallible; the wire is permissive.
    /// Use `try_new` if you want validation.
    pub fn new(s: impl Into<String>) -> Self {
        Reference(s.into())
    }

    /// Validating constructor. Returns `Err` for empty or control-char strings.
    pub fn try_new(s: impl Into<String>) -> Result<Self, &'static str> {
        let s = s.into();
        if s.is_empty() {
            return Err("reference must not be empty");
        }
        if s.chars().any(|c| c.is_ascii_control()) {
            return Err("reference must not contain ASCII control characters");
        }
        Ok(Reference(s))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for Reference {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}
