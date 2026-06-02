//! RED: prism_core::pq::schema doesn't exist yet.
//!
//! T12.1 — JSON Schema derivation from the pq typed DSL. Three public
//! emitters (`target_schema`, `filter_schema`, `output_schema`) produce
//! `serde_json::Value` JSON Schemas (draft 2020-12) that:
//!
//! 1. Discriminate every variant of their source enum.
//! 2. Validate every constructible variant of the DSL (round-trip test).
//! 3. Reject shapes the typed `Deserialize` impl would reject.
//! 4. Are deterministic (idempotent under repeated emission).
//!
//! The schema is derived FROM the Rust types, per pq spec §5.4:
//! "The wire schema (JSON Schema) is derived from the typed DSL; the
//!  schema is not the source of truth — the Rust types are."

#![cfg(feature = "pq")]

use jsonschema::Validator;
use prism_core::pq::{
    schema::{filter_schema, output_schema, target_schema},
    CasUpdate, Direction, Filter, OrderSpec, Output, Reference, Target, WalkDirection, WhereClause,
    WhereOp,
};
use prism_core::Oid;
use serde_json::{json, Value};

// ── helpers ─────────────────────────────────────────────────────────────────────

fn compile(schema: &Value) -> Validator {
    Validator::new(schema).expect("emitted schema must compile as a JSON Schema")
}

fn assert_oneof_len(schema: &Value, expected: usize) {
    let one_of = schema
        .get("oneOf")
        .expect("schema must have a `oneOf` discriminator")
        .as_array()
        .expect("`oneOf` must be an array");
    assert_eq!(
        one_of.len(),
        expected,
        "expected {} branches in `oneOf`, got {}",
        expected,
        one_of.len()
    );
}

// ── Target schema ───────────────────────────────────────────────────────────────

#[test]
fn target_schema_emits_oneof_for_seven_variants() {
    let schema = target_schema();
    assert_eq!(schema.get("type").and_then(Value::as_str), Some("object"));
    assert!(schema.get("title").is_some(), "Target schema needs a title");
    assert_oneof_len(&schema, 7);
}

#[test]
fn target_schema_validates_every_variant() {
    let schema = target_schema();
    let v = compile(&schema);

    let samples: Vec<(&str, Target)> = vec![
        ("empty", Target::Empty {}),
        (
            "oid",
            Target::Oid {
                oid: Oid::new("abc123"),
            },
        ),
        (
            "path",
            Target::Path {
                path: "src/foo.rs".into(),
            },
        ),
        (
            "ref",
            Target::Ref {
                ref_: Reference::new("HEAD"),
            },
        ),
        (
            "pair",
            Target::Pair {
                pair: Box::new([
                    Target::Oid { oid: Oid::new("a") },
                    Target::Oid { oid: Oid::new("b") },
                ]),
            },
        ),
        ("refs", Target::Refs { refs: true }),
        ("shard", Target::Shard { shard: true }),
    ];

    for (label, t) in samples {
        let j = serde_json::to_value(&t).unwrap();
        assert!(
            v.is_valid(&j),
            "Target::{} ({}) must validate against target_schema",
            label,
            j
        );
    }
}

#[test]
fn target_schema_rejects_unknown_key() {
    let schema = target_schema();
    let v = compile(&schema);
    let bad = json!({"unknown": "key"});
    assert!(
        !v.is_valid(&bad),
        "target_schema must reject {{\"unknown\": ...}} — the typed Deserialize rejects it too"
    );
}

#[test]
fn target_schema_rejects_two_keys() {
    let schema = target_schema();
    let v = compile(&schema);
    // The manual Deserialize for Target requires exactly one key. The schema
    // must mirror that — a two-key object that names two known variants must
    // still fail (not stringly-coerce into a neighbour).
    let bad = json!({"oid": "abc", "path": "src/foo.rs"});
    assert!(
        !v.is_valid(&bad),
        "target_schema must reject objects carrying two variant keys"
    );
}

#[test]
fn target_schema_is_deterministic() {
    assert_eq!(
        target_schema(),
        target_schema(),
        "schema emission must be deterministic"
    );
}

// ── Filter schema ───────────────────────────────────────────────────────────────

#[test]
fn filter_schema_emits_oneof_for_eight_variants() {
    let schema = filter_schema();
    assert_eq!(schema.get("type").and_then(Value::as_str), Some("object"));
    assert!(schema.get("title").is_some(), "Filter schema needs a title");
    // Variants per filter.rs: Prefix, Match, Walk, Compare, Kintsugi, Order, Limit, Where.
    assert_oneof_len(&schema, 8);
}

#[test]
fn filter_schema_validates_every_variant() {
    let schema = filter_schema();
    let v = compile(&schema);

    let samples: Vec<(&str, Filter)> = vec![
        (
            "prefix",
            Filter::Prefix {
                prefix: "src/".into(),
            },
        ),
        (
            "match",
            Filter::Match {
                match_: "auth".into(),
            },
        ),
        (
            "walk-back",
            Filter::Walk {
                walk: WalkDirection::Back,
            },
        ),
        (
            "walk-forward",
            Filter::Walk {
                walk: WalkDirection::Forward,
            },
        ),
        ("compare", Filter::Compare { compare: true }),
        ("kintsugi", Filter::Kintsugi { kintsugi: true }),
        (
            "order",
            Filter::Order {
                order: vec![OrderSpec {
                    field: "name".into(),
                    direction: Direction::Desc,
                }],
            },
        ),
        ("limit", Filter::Limit { limit: 25 }),
        (
            "where",
            Filter::Where {
                where_: vec![WhereClause {
                    field: "kind".into(),
                    op: WhereOp::Contains,
                    value: json!("commit"),
                }],
            },
        ),
    ];

    for (label, f) in samples {
        let j = serde_json::to_value(&f).unwrap();
        assert!(
            v.is_valid(&j),
            "Filter::{} ({}) must validate against filter_schema",
            label,
            j
        );
    }
}

#[test]
fn filter_schema_rejects_unknown_key() {
    let schema = filter_schema();
    let v = compile(&schema);
    assert!(
        !v.is_valid(&json!({"squelch": true})),
        "filter_schema must reject unknown variant keys"
    );
}

#[test]
fn filter_schema_walk_constrains_to_back_or_forward() {
    let schema = filter_schema();
    let v = compile(&schema);
    assert!(!v.is_valid(&json!({"walk": "sideways"})));
    assert!(v.is_valid(&json!({"walk": "back"})));
    assert!(v.is_valid(&json!({"walk": "forward"})));
}

#[test]
fn filter_schema_is_deterministic() {
    assert_eq!(filter_schema(), filter_schema());
}

// ── Output schema ───────────────────────────────────────────────────────────────

#[test]
fn output_schema_emits_oneof_for_six_variants() {
    let schema = output_schema();
    assert_eq!(schema.get("type").and_then(Value::as_str), Some("object"));
    assert!(schema.get("title").is_some(), "Output schema needs a title");
    // Variants per output.rs: ToPath, Ref, Cas, ToRef, Snapshot, Flush.
    assert_oneof_len(&schema, 6);
}

#[test]
fn output_schema_validates_every_variant() {
    let schema = output_schema();
    let v = compile(&schema);

    let samples: Vec<(&str, Output)> = vec![
        (
            "to_path-with-message",
            Output::ToPath {
                to_path: "notes/a.md".into(),
                message: Some("first commit".into()),
            },
        ),
        (
            "to_path-bare",
            Output::ToPath {
                to_path: "scratch".into(),
                message: None,
            },
        ),
        (
            "ref",
            Output::Ref {
                ref_: Reference::new("feature/x"),
            },
        ),
        (
            "cas",
            Output::Cas {
                cas: CasUpdate {
                    old: Oid::new("abc"),
                    new: Oid::new("def"),
                },
            },
        ),
        (
            "to_ref",
            Output::ToRef {
                to_ref: Reference::new("main"),
            },
        ),
        ("snapshot", Output::Snapshot { snapshot: true }),
        ("flush", Output::Flush { flush: true }),
    ];

    for (label, o) in samples {
        let j = serde_json::to_value(&o).unwrap();
        assert!(
            v.is_valid(&j),
            "Output::{} ({}) must validate against output_schema",
            label,
            j
        );
    }
}

#[test]
fn output_schema_rejects_unknown_key() {
    let schema = output_schema();
    let v = compile(&schema);
    assert!(
        !v.is_valid(&json!({"discard": true})),
        "output_schema must reject unknown variant keys"
    );
}

#[test]
fn output_to_path_allows_optional_message() {
    let schema = output_schema();
    let v = compile(&schema);
    // message is skipped when None on serialize; the schema must accept both.
    assert!(v.is_valid(&json!({"to_path": "x"})));
    assert!(v.is_valid(&json!({"to_path": "x", "message": "y"})));
}

#[test]
fn output_schema_is_deterministic() {
    assert_eq!(output_schema(), output_schema());
}

// ── Cross-schema invariants ────────────────────────────────────────────────────

#[test]
fn schemas_declare_draft_2020_12() {
    for (name, s) in [
        ("target", target_schema()),
        ("filter", filter_schema()),
        ("output", output_schema()),
    ] {
        let dollar_schema = s.get("$schema").and_then(Value::as_str).unwrap_or_else(|| {
            panic!("{} schema must declare $schema", name);
        });
        assert!(
            dollar_schema.contains("2020-12"),
            "{} schema must declare draft 2020-12, got {}",
            name,
            dollar_schema
        );
    }
}

#[test]
fn schemas_carry_distinct_titles() {
    let t = target_schema()
        .get("title")
        .and_then(Value::as_str)
        .unwrap_or("")
        .to_string();
    let f = filter_schema()
        .get("title")
        .and_then(Value::as_str)
        .unwrap_or("")
        .to_string();
    let o = output_schema()
        .get("title")
        .and_then(Value::as_str)
        .unwrap_or("")
        .to_string();
    assert_ne!(t, f);
    assert_ne!(f, o);
    assert_ne!(t, o);
    assert!(!t.is_empty() && !f.is_empty() && !o.is_empty());
}
