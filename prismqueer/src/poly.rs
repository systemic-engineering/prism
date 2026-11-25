//! `poly`: a law of prism's floor, re-exported at its old path. It lives in
//! `prism_laws::poly` (docs/spec/laws.md), judges and all.

pub use prism_laws::poly::*;

// The lab's evaluation budget converts into poly's (`bundle`). A `From` impl
// for a law type from a lab type: it lives on the lab's side.
#[cfg(feature = "bundle")]
impl From<crate::flux::pillar::Budget> for Budget {
    fn from(budget: crate::flux::pillar::Budget) -> Self {
        Budget(budget.0)
    }
}
