//! JSON Schema derivation for the pq typed DSL.
//!
//! Per pq spec §5.4: "The wire schema (JSON Schema) is **derived** from
//! the typed DSL; the schema is not the source of truth — the Rust
//! types are." This module is the derivation.
//!
//! ## Why hand-rolled
//!
//! The three discriminated unions ([`super::Target`], [`super::Filter`],
//! [`super::Output`]) use `#[serde(untagged)]` with per-variant
//! `deny_unknown_fields`. `Target` additionally implements `Deserialize`
//! manually to enforce exactly-one-key dispatch (so `{}` is `Empty`,
//! `{"unknown": ...}` fails). A derive-macro path (e.g. `schemars`)
//! would derive a schema from the `Serialize`/`Deserialize` annotations,
//! not the manual impl — it cannot see the runtime semantics. To stay
//! faithful to the type contract, we hand-roll the schema here, one
//! variant at a time, mirroring the Rust types.
//!
//! Hand-rolling also keeps `prism_core` deps-free per the kernel
//! discipline (pq spec §12.1: "Zero new deps").
//!
//! ## Drift protection
//!
//! Every variant in the source enums has a corresponding round-trip
//! test in `tests/pq_schema.rs` that constructs the variant, serialises
//! it, and validates the JSON against the emitted schema. If a new
//! variant is added without updating the schema, the round-trip test
//! fails — the test suite is the contract that the schema stays in
//! sync with the types.

use serde_json::{json, Value};

/// JSON Schema draft URI declared at the root of every emitted schema.
const DRAFT_2020_12: &str = "https://json-schema.org/draft/2020-12/schema";

// ─── Target ─────────────────────────────────────────────────────────────────

/// Emit a JSON Schema for [`super::Target`].
///
/// Mirrors the manual `Deserialize` impl in `target.rs`: the empty object
/// `{}` is its own variant, every other variant is dispatched by a single
/// canonical key, and unknown keys are rejected. The schema enforces
/// `maxProperties: 1` at the root so two-key shapes (which the typed
/// dispatch also rejects) cannot stringly-coerce.
pub fn target_schema() -> Value {
    let one_of = vec![
        // {} → Target::Empty
        json!({
            "type": "object",
            "title": "Empty",
            "description": "The shard's current focus.",
            "additionalProperties": false,
            "maxProperties": 0,
        }),
        // {"oid": "..."} → Target::Oid
        single_key_variant("Oid", "Focus by content address.", "oid", oid_schema()),
        // {"path": "..."} → Target::Path
        single_key_variant(
            "Path",
            "Focus by working-tree path.",
            "path",
            json!({ "type": "string" }),
        ),
        // {"ref": "..."} → Target::Ref
        single_key_variant("Ref", "Focus by named ref.", "ref", reference_schema()),
        // {"pair": [Target, Target]} → Target::Pair
        single_key_variant(
            "Pair",
            "Focus on a pair (diff/merge inputs).",
            "pair",
            json!({
                "type": "array",
                "minItems": 2,
                "maxItems": 2,
                // Recursive reference — Pair's items are Targets again.
                "items": { "$ref": "#" },
            }),
        ),
        // {"refs": true} → Target::Refs
        single_key_variant(
            "Refs",
            "Focus the ref-set.",
            "refs",
            json!({ "type": "boolean" }),
        ),
        // {"shard": true} → Target::Shard
        single_key_variant(
            "Shard",
            "Focus the shard's summary.",
            "shard",
            json!({ "type": "boolean" }),
        ),
    ];

    json!({
        "$schema": DRAFT_2020_12,
        "$id": "https://prism.engineer/pq/target.schema.json",
        "title": "pq.Target",
        "description": "Focus DSL — per pq spec §5.1.",
        "type": "object",
        // The manual Deserialize for Target requires exactly one key.
        // Mirror that here so the schema agrees with the typed dispatch.
        "maxProperties": 1,
        "oneOf": one_of,
    })
}

// ─── Filter ─────────────────────────────────────────────────────────────────

/// Emit a JSON Schema for [`super::Filter`].
///
/// Mirrors `Filter`'s `#[serde(untagged)]` + `deny_unknown_fields`
/// pattern. Each variant carries exactly one discriminating key.
pub fn filter_schema() -> Value {
    let one_of = vec![
        single_key_variant(
            "Prefix",
            "String-prefix narrowing.",
            "prefix",
            json!({ "type": "string" }),
        ),
        single_key_variant(
            "Match",
            "Pattern/substring narrowing.",
            "match",
            json!({ "type": "string" }),
        ),
        single_key_variant(
            "Walk",
            "DAG walk direction (history traversal).",
            "walk",
            json!({ "type": "string", "enum": ["back", "forward"] }),
        ),
        single_key_variant(
            "Compare",
            "Structural diff of a focused pair.",
            "compare",
            json!({ "type": "boolean" }),
        ),
        single_key_variant(
            "Kintsugi",
            "Tournament merge of a focused pair.",
            "kintsugi",
            json!({ "type": "boolean" }),
        ),
        single_key_variant(
            "Order",
            "Ordering.",
            "order",
            json!({
                "type": "array",
                "items": order_spec_schema(),
            }),
        ),
        single_key_variant(
            "Limit",
            "Bounded results.",
            "limit",
            json!({
                "type": "integer",
                "minimum": 0,
                "maximum": u32::MAX,
            }),
        ),
        single_key_variant(
            "Where",
            "Typed predicate.",
            "where",
            json!({
                "type": "array",
                "items": where_clause_schema(),
            }),
        ),
    ];

    json!({
        "$schema": DRAFT_2020_12,
        "$id": "https://prism.engineer/pq/filter.schema.json",
        "title": "pq.Filter",
        "description": "Project DSL — per pq spec §5.2.",
        "type": "object",
        "oneOf": one_of,
    })
}

// ─── Output ─────────────────────────────────────────────────────────────────

/// Emit a JSON Schema for [`super::Output`].
///
/// Mirrors `Output`'s `#[serde(untagged)]` + `deny_unknown_fields`
/// pattern. `ToPath` is the only variant with an optional second field
/// (`message`); the rest are single-key dispatches.
pub fn output_schema() -> Value {
    let one_of = vec![
        // {"to_path": "...", "message"?: "..."} → Output::ToPath
        // Two-field variant — uses `required` + explicit `properties`
        // rather than the single_key helper.
        json!({
            "type": "object",
            "title": "ToPath",
            "description": "Commit to a path.",
            "additionalProperties": false,
            "required": ["to_path"],
            "properties": {
                "to_path": { "type": "string" },
                "message": { "type": "string" },
            },
        }),
        single_key_variant(
            "Ref",
            "Advance a ref (branch creation / update).",
            "ref",
            reference_schema(),
        ),
        single_key_variant(
            "Cas",
            "CAS-safe ref update.",
            "cas",
            json!({
                "type": "object",
                "additionalProperties": false,
                "required": ["old", "new"],
                "properties": {
                    "old": oid_schema(),
                    "new": oid_schema(),
                },
            }),
        ),
        single_key_variant(
            "ToRef",
            "Write merged result to a ref.",
            "to_ref",
            reference_schema(),
        ),
        single_key_variant(
            "Snapshot",
            "Crystallize without committing.",
            "snapshot",
            json!({ "type": "boolean" }),
        ),
        single_key_variant(
            "Flush",
            "Shard flush to disk.",
            "flush",
            json!({ "type": "boolean" }),
        ),
    ];

    json!({
        "$schema": DRAFT_2020_12,
        "$id": "https://prism.engineer/pq/output.schema.json",
        "title": "pq.Output",
        "description": "Refract DSL — per pq spec §5.3.",
        "type": "object",
        "oneOf": one_of,
    })
}

// ─── helpers ─────────────────────────────────────────────────────────────────

/// Build a single-key, deny-unknown-fields object schema for a one-field
/// variant. The shape is:
/// ```jsonc
/// {
///   "type": "object",
///   "title": "<VariantName>",
///   "description": "<doc>",
///   "additionalProperties": false,
///   "required": ["<key>"],
///   "properties": { "<key>": <value_schema> }
/// }
/// ```
fn single_key_variant(title: &str, description: &str, key: &str, value_schema: Value) -> Value {
    json!({
        "type": "object",
        "title": title,
        "description": description,
        "additionalProperties": false,
        "required": [key],
        "properties": {
            key: value_schema,
        },
    })
}

/// Schema for [`crate::Oid`] — a hex string on the wire (via
/// `#[serde(transparent)]`).
fn oid_schema() -> Value {
    json!({
        "type": "string",
        "description": "Content-addressed identifier (hex).",
    })
}

/// Schema for [`super::Reference`] — a non-empty string with no ASCII
/// control characters. The validating constructor is `Reference::try_new`;
/// the bare `new` is infallible, so the wire side enforces only what the
/// type's invariants require for round-trip safety.
fn reference_schema() -> Value {
    json!({
        "type": "string",
        "description": "Wire-layer reference name (HEAD, main, feature/x, ...).",
    })
}

/// Schema for [`super::OrderSpec`] — `{"field": "...", "direction": "asc"|"desc"}`.
fn order_spec_schema() -> Value {
    json!({
        "type": "object",
        "additionalProperties": false,
        "required": ["field", "direction"],
        "properties": {
            "field": { "type": "string" },
            "direction": { "type": "string", "enum": ["asc", "desc"] },
        },
    })
}

/// Schema for [`super::WhereClause`] — `{"field": "...", "op": ..., "value": <any>}`.
///
/// The `value` field is `serde_json::Value` in the Rust type — its JSON
/// Schema branch is the unconstrained `true` schema. Tightening to a
/// per-`op` type lattice is deferred to T12.2 (when the impl knows
/// which substrate types each `op` is meaningful for).
fn where_clause_schema() -> Value {
    json!({
        "type": "object",
        "additionalProperties": false,
        "required": ["field", "op", "value"],
        "properties": {
            "field": { "type": "string" },
            "op": {
                "type": "string",
                "enum": ["eq", "neq", "gt", "lt", "gte", "lte", "contains", "matches"],
            },
            // serde_json::Value — accept anything until T12.2 tightens.
            "value": true,
        },
    })
}
