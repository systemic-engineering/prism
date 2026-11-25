//! `pq` — the typed DSL for the prism-query wire alphabet.
//!
//! Per the [pq spec](../../../docs/specs/pq.md) §5, three discriminated
//! unions plus supporting types describe every shape that crosses the
//! pq wire: `Target` (focus), `Filter` (project), `Output` (settle).
//!
//! The types are gated by the `pq` feature so prismqueer's core API
//! stays serde-free for consumers that don't need wire shapes.

mod filter;
mod output;
mod reference;
pub mod schema;
mod target;

/// An [`Oid`](crate::Oid) on the pq wire: its string, as it is. `Oid` lives
/// in prism-laws, which carries no serde (docs/spec/laws.md), so the lab
/// says how it crosses its own wire: what `#[serde(transparent)]` on `Oid`
/// said before the move.
mod oid_wire {
    use serde::{Deserialize, Deserializer, Serializer};

    use crate::Oid;

    pub fn serialize<S: Serializer>(oid: &Oid, s: S) -> Result<S::Ok, S::Error> {
        s.serialize_str(oid.as_str())
    }

    pub fn deserialize<'de, D: Deserializer<'de>>(d: D) -> Result<Oid, D::Error> {
        String::deserialize(d).map(Oid::from)
    }
}

pub use filter::{Direction, Filter, OrderSpec, WalkDirection, WhereClause, WhereOp};
pub use output::{CasUpdate, Output};
pub use reference::Reference;
pub use target::Target;
