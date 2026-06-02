//! `pq::Filter` — the project DSL.
//!
//! Per pq spec §5.2, `Filter` is a discriminated union over narrowing
//! criteria. Each variant is a distinct JSON key-shape.

use serde::{Deserialize, Serialize};
use serde_json::Value;

/// DAG walk direction for `Filter::Walk` (history traversal).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum WalkDirection {
    Back,
    Forward,
}

/// Sort direction for `OrderSpec`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Direction {
    Asc,
    Desc,
}

/// Comparison operator for `WhereClause`. The string forms are the
/// wire-canonical names per pq §5.2 / mq's where-clause vocabulary.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum WhereOp {
    Eq,
    Neq,
    Gt,
    Lt,
    Gte,
    Lte,
    Contains,
    Matches,
}

/// A single ordering spec — `{"field": "...", "direction": "asc"|"desc"}`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OrderSpec {
    pub field: String,
    pub direction: Direction,
}

/// A single where-clause predicate.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WhereClause {
    pub field: String,
    pub op: WhereOp,
    pub value: Value,
}

/// Project DSL. Each variant maps to a distinct JSON key-shape.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Filter {
    /// `{"prefix": "..."}` — string-prefix narrowing.
    Prefix { prefix: String },
    /// `{"match": "..."}` — pattern/substring narrowing. `match` is
    /// a Rust keyword.
    Match {
        #[serde(rename = "match")]
        match_: String,
    },
    /// `{"walk": "back"|"forward"}` — DAG walk direction.
    Walk { walk: WalkDirection },
    /// `{"compare": true}` — structural diff of a focused pair.
    Compare { compare: bool },
    /// `{"kintsugi": true}` — tournament merge of a focused pair.
    Kintsugi { kintsugi: bool },
    /// `{"order": [...]}` — ordering.
    Order { order: Vec<OrderSpec> },
    /// `{"limit": N}` — bounded results.
    Limit { limit: u32 },
    /// `{"where": [...]}` — typed predicate. `where` is a Rust keyword.
    Where {
        #[serde(rename = "where")]
        where_: Vec<WhereClause>,
    },
}
