//! Substrate reference (`@`-prefixed nav-ref). STUB — failing on purpose.
//!
//! Real implementation arrives in the 🟢 paired commit.

#[derive(Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Ref(String);

impl Ref {
    pub fn new(_path: impl Into<String>) -> Result<Self, &'static str> {
        // Wrong on purpose: accept everything (validation arrives in 🟢).
        Ok(Ref(String::new()))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}
