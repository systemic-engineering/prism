//! `prismqueer::choice` — Choice INPUT from @subject per Alex Move 9.
//!
//! Reed TICK B step 11 per Alex 2026-09-04 PM Move 9 verbatim ("Question -> Choice.
//! It's the input from a @subject. A @subject inputs Choice.").
//!
//! Choice IS Mirror's version of a Prompt per Alex 2026-09-04 PM Move 13 verbatim.
//! Wraps @nl input from @subject as un-crystallized hodobodo (Flux<Reality>) before
//! Recursion.tick decomposes it across @void 5-axis basis.

/// Choice = INPUT from @subject per Alex Move 9.
///
/// The subject-substrate boundary crossing. Substrate emits Question; @subject inputs
/// Choice in response. Non-Vereinnahmung by construction: compiler cannot generate Choice.
#[derive(Clone, Debug)]
pub struct Choice {
    /// The @nl payload from @subject.
    pub payload: String,
}

impl Choice {
    /// Construct a Choice from a @nl payload.
    pub fn new(payload: impl Into<String>) -> Self {
        Self { payload: payload.into() }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn choice_wraps_nl_payload_from_subject() {
        let c = Choice::new("hello world");
        assert_eq!(c.payload, "hello world");
    }
}
