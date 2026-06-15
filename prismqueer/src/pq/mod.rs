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

pub use filter::{Direction, Filter, OrderSpec, WalkDirection, WhereClause, WhereOp};
pub use output::{CasUpdate, Output};
pub use reference::Reference;
pub use target::Target;
