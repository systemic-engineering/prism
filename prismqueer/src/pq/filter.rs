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
    /// Walk toward the past (parents).
    Back,
    /// Walk toward the future (children).
    Forward,
}

/// Sort direction for `OrderSpec`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Direction {
    /// Ascending order.
    Asc,
    /// Descending order.
    Desc,
}

/// Comparison operator for `WhereClause`. The string forms are the
/// wire-canonical names per pq §5.2 / mq's where-clause vocabulary.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum WhereOp {
    /// `==`
    Eq,
    /// `!=`
    Neq,
    /// `>`
    Gt,
    /// `<`
    Lt,
    /// `>=`
    Gte,
    /// `<=`
    Lte,
    /// Substring / element containment.
    Contains,
    /// Pattern match (regex/glob — the consumer decides the dialect).
    Matches,
}

/// A single ordering spec — `{"field": "...", "direction": "asc"|"desc"}`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OrderSpec {
    /// The field name to order by.
    pub field: String,
    /// The sort direction.
    pub direction: Direction,
}

/// A single where-clause predicate.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WhereClause {
    /// The field name to test.
    pub field: String,
    /// The comparison operator.
    pub op: WhereOp,
    /// The RHS value, free-form JSON.
    pub value: Value,
}

/// Project DSL. Each variant maps to a distinct JSON key-shape.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Filter {
    /// `{"prefix": "..."}` — string-prefix narrowing.
    Prefix {
        /// The string prefix to narrow by.
        prefix: String,
    },
    /// `{"match": "..."}` — pattern/substring narrowing. `match` is
    /// a Rust keyword, hence the trailing-underscore field name.
    Match {
        /// The pattern (substring or regex — the consumer decides).
        #[serde(rename = "match")]
        match_: String,
    },
    /// `{"walk": "back"|"forward"}` — DAG walk direction.
    Walk {
        /// Which direction to walk the DAG.
        walk: WalkDirection,
    },
    /// `{"compare": true}` — structural diff of a focused pair.
    Compare {
        /// Always `true` on the wire — the variant tag IS the action.
        compare: bool,
    },
    /// `{"kintsugi": true}` — tournament merge of a focused pair.
    Kintsugi {
        /// Always `true` on the wire — the variant tag IS the action.
        kintsugi: bool,
    },
    /// `{"order": [...]}` — ordering.
    Order {
        /// The ordering specs, applied lexicographically.
        order: Vec<OrderSpec>,
    },
    /// `{"limit": N}` — bounded results.
    Limit {
        /// Maximum number of results.
        limit: u32,
    },
    /// `{"where": [...]}` — typed predicate. `where` is a Rust keyword,
    /// hence the trailing-underscore field name.
    Where {
        /// The conjunction of predicates that must hold.
        #[serde(rename = "where")]
        where_: Vec<WhereClause>,
    },
}
