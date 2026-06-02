//! RED: prism_core::pq doesn't exist yet.
//!
//! T11.1 — typed DSL for pq's wire alphabet. Three discriminated unions
//! (Target / Filter / Output) plus supporting types. Each variant
//! corresponds to a key-shape on the wire per pq spec §5.
//!
//! Tests assert:
//! 1. Every variant constructs.
//! 2. Every variant serialises to the JSON shape declared in pq §5.
//! 3. Every variant round-trips Rust → JSON → Rust by value.
//! 4. Cross-variant disambiguation is correct (untagged + deny_unknown_fields).
//! 5. The DSL types implement the required trait bounds.

#![cfg(feature = "pq")]

use prism_core::pq::{
    CasUpdate, Direction, Filter, OrderSpec, Output, Reference, Target, WalkDirection, WhereClause,
    WhereOp,
};
use prism_core::Oid;
use serde_json::json;

// ── Target round-trips ──────────────────────────────────────────────────────────

#[test]
fn target_empty_round_trips() {
    let t = Target::Empty {};
    let j = serde_json::to_value(&t).unwrap();
    assert_eq!(j, json!({}));
    let back: Target = serde_json::from_value(j).unwrap();
    assert_eq!(back, t);
}

#[test]
fn target_oid_round_trips() {
    let t = Target::Oid {
        oid: Oid::new("abc123"),
    };
    let j = serde_json::to_value(&t).unwrap();
    assert_eq!(j, json!({"oid": "abc123"}));
    let back: Target = serde_json::from_value(j).unwrap();
    assert_eq!(back, t);
}

#[test]
fn target_path_round_trips() {
    let t = Target::Path {
        path: "src/foo.rs".into(),
    };
    let j = serde_json::to_value(&t).unwrap();
    assert_eq!(j, json!({"path": "src/foo.rs"}));
    let back: Target = serde_json::from_value(j).unwrap();
    assert_eq!(back, t);
}

#[test]
fn target_ref_round_trips() {
    let t = Target::Ref {
        ref_: Reference::new("HEAD"),
    };
    let j = serde_json::to_value(&t).unwrap();
    assert_eq!(j, json!({"ref": "HEAD"}));
    let back: Target = serde_json::from_value(j).unwrap();
    assert_eq!(back, t);
}

#[test]
fn target_pair_round_trips() {
    let t = Target::Pair {
        pair: Box::new([
            Target::Oid { oid: Oid::new("a") },
            Target::Oid { oid: Oid::new("b") },
        ]),
    };
    let j = serde_json::to_value(&t).unwrap();
    assert_eq!(j, json!({"pair": [{"oid": "a"}, {"oid": "b"}]}));
    let back: Target = serde_json::from_value(j).unwrap();
    assert_eq!(back, t);
}

#[test]
fn target_refs_round_trips() {
    let t = Target::Refs { refs: true };
    let j = serde_json::to_value(&t).unwrap();
    assert_eq!(j, json!({"refs": true}));
    let back: Target = serde_json::from_value(j).unwrap();
    assert_eq!(back, t);
}

#[test]
fn target_shard_round_trips() {
    let t = Target::Shard { shard: true };
    let j = serde_json::to_value(&t).unwrap();
    assert_eq!(j, json!({"shard": true}));
    let back: Target = serde_json::from_value(j).unwrap();
    assert_eq!(back, t);
}

#[test]
fn target_rejects_unknown_keys() {
    // deny_unknown_fields per variant; no variant accepts `{"unknown": ...}`.
    let result: Result<Target, _> = serde_json::from_value(json!({"unknown": "key"}));
    assert!(
        result.is_err(),
        "unknown key must not stringly-coerce into a variant"
    );
}

// ── Filter round-trips ──────────────────────────────────────────────────────────

#[test]
fn filter_prefix_round_trips() {
    let f = Filter::Prefix {
        prefix: "src/".into(),
    };
    assert_eq!(serde_json::to_value(&f).unwrap(), json!({"prefix": "src/"}));
    let back: Filter = serde_json::from_value(json!({"prefix": "src/"})).unwrap();
    assert_eq!(back, f);
}

#[test]
fn filter_match_round_trips() {
    let f = Filter::Match {
        match_: "auth".into(),
    };
    assert_eq!(serde_json::to_value(&f).unwrap(), json!({"match": "auth"}));
    let back: Filter = serde_json::from_value(json!({"match": "auth"})).unwrap();
    assert_eq!(back, f);
}

#[test]
fn filter_walk_back_round_trips() {
    let f = Filter::Walk {
        walk: WalkDirection::Back,
    };
    assert_eq!(serde_json::to_value(&f).unwrap(), json!({"walk": "back"}));
    let back: Filter = serde_json::from_value(json!({"walk": "back"})).unwrap();
    assert_eq!(back, f);
}

#[test]
fn filter_walk_forward_round_trips() {
    let f = Filter::Walk {
        walk: WalkDirection::Forward,
    };
    assert_eq!(
        serde_json::to_value(&f).unwrap(),
        json!({"walk": "forward"})
    );
    let back: Filter = serde_json::from_value(json!({"walk": "forward"})).unwrap();
    assert_eq!(back, f);
}

#[test]
fn filter_compare_round_trips() {
    let f = Filter::Compare { compare: true };
    assert_eq!(serde_json::to_value(&f).unwrap(), json!({"compare": true}));
    let back: Filter = serde_json::from_value(json!({"compare": true})).unwrap();
    assert_eq!(back, f);
}

#[test]
fn filter_kintsugi_round_trips() {
    let f = Filter::Kintsugi { kintsugi: true };
    assert_eq!(serde_json::to_value(&f).unwrap(), json!({"kintsugi": true}));
    let back: Filter = serde_json::from_value(json!({"kintsugi": true})).unwrap();
    assert_eq!(back, f);
}

#[test]
fn filter_order_round_trips() {
    let f = Filter::Order {
        order: vec![OrderSpec {
            field: "name".into(),
            direction: Direction::Asc,
        }],
    };
    let j = serde_json::to_value(&f).unwrap();
    assert_eq!(j, json!({"order": [{"field": "name", "direction": "asc"}]}));
    let back: Filter = serde_json::from_value(j).unwrap();
    assert_eq!(back, f);
}

#[test]
fn filter_order_direction_desc() {
    let spec = OrderSpec {
        field: "timestamp".into(),
        direction: Direction::Desc,
    };
    assert_eq!(
        serde_json::to_value(&spec).unwrap(),
        json!({"field": "timestamp", "direction": "desc"})
    );
}

#[test]
fn filter_limit_round_trips() {
    let f = Filter::Limit { limit: 50 };
    assert_eq!(serde_json::to_value(&f).unwrap(), json!({"limit": 50}));
    let back: Filter = serde_json::from_value(json!({"limit": 50})).unwrap();
    assert_eq!(back, f);
}

#[test]
fn filter_where_round_trips() {
    let f = Filter::Where {
        where_: vec![WhereClause {
            field: "kind".into(),
            op: WhereOp::Eq,
            value: json!("commit"),
        }],
    };
    let j = serde_json::to_value(&f).unwrap();
    assert_eq!(
        j,
        json!({"where": [{"field": "kind", "op": "eq", "value": "commit"}]})
    );
    let back: Filter = serde_json::from_value(j).unwrap();
    assert_eq!(back, f);
}

#[test]
fn where_op_serializes_each_variant() {
    assert_eq!(serde_json::to_value(WhereOp::Eq).unwrap(), json!("eq"));
    assert_eq!(serde_json::to_value(WhereOp::Neq).unwrap(), json!("neq"));
    assert_eq!(serde_json::to_value(WhereOp::Gt).unwrap(), json!("gt"));
    assert_eq!(serde_json::to_value(WhereOp::Lt).unwrap(), json!("lt"));
    assert_eq!(serde_json::to_value(WhereOp::Gte).unwrap(), json!("gte"));
    assert_eq!(serde_json::to_value(WhereOp::Lte).unwrap(), json!("lte"));
    assert_eq!(
        serde_json::to_value(WhereOp::Contains).unwrap(),
        json!("contains")
    );
    assert_eq!(
        serde_json::to_value(WhereOp::Matches).unwrap(),
        json!("matches")
    );
}

// ── Output round-trips ──────────────────────────────────────────────────────────

#[test]
fn output_to_path_with_message_round_trips() {
    let o = Output::ToPath {
        to_path: "notes/a.md".into(),
        message: Some("first commit".into()),
    };
    let j = serde_json::to_value(&o).unwrap();
    assert_eq!(
        j,
        json!({"to_path": "notes/a.md", "message": "first commit"})
    );
    let back: Output = serde_json::from_value(j).unwrap();
    assert_eq!(back, o);
}

#[test]
fn output_to_path_without_message_omits_field() {
    let o = Output::ToPath {
        to_path: "scratch".into(),
        message: None,
    };
    let j = serde_json::to_value(&o).unwrap();
    // skip_serializing_if = "Option::is_none" — message must NOT appear in JSON.
    assert_eq!(j, json!({"to_path": "scratch"}));
    let back: Output = serde_json::from_value(j).unwrap();
    assert_eq!(back, o);
}

#[test]
fn output_ref_round_trips() {
    let o = Output::Ref {
        ref_: Reference::new("feature/x"),
    };
    let j = serde_json::to_value(&o).unwrap();
    assert_eq!(j, json!({"ref": "feature/x"}));
    let back: Output = serde_json::from_value(j).unwrap();
    assert_eq!(back, o);
}

#[test]
fn output_cas_round_trips() {
    let o = Output::Cas {
        cas: CasUpdate {
            old: Oid::new("abc"),
            new: Oid::new("def"),
        },
    };
    let j = serde_json::to_value(&o).unwrap();
    assert_eq!(j, json!({"cas": {"old": "abc", "new": "def"}}));
    let back: Output = serde_json::from_value(j).unwrap();
    assert_eq!(back, o);
}

#[test]
fn output_to_ref_round_trips() {
    let o = Output::ToRef {
        to_ref: Reference::new("main"),
    };
    let j = serde_json::to_value(&o).unwrap();
    assert_eq!(j, json!({"to_ref": "main"}));
    let back: Output = serde_json::from_value(j).unwrap();
    assert_eq!(back, o);
}

#[test]
fn output_snapshot_round_trips() {
    let o = Output::Snapshot { snapshot: true };
    assert_eq!(serde_json::to_value(&o).unwrap(), json!({"snapshot": true}));
    let back: Output = serde_json::from_value(json!({"snapshot": true})).unwrap();
    assert_eq!(back, o);
}

#[test]
fn output_flush_round_trips() {
    let o = Output::Flush { flush: true };
    assert_eq!(serde_json::to_value(&o).unwrap(), json!({"flush": true}));
    let back: Output = serde_json::from_value(json!({"flush": true})).unwrap();
    assert_eq!(back, o);
}

// ── Compile-time trait bounds ──────────────────────────────────────────────────────

#[test]
fn types_implement_required_traits() {
    fn assert_bounds<T>()
    where
        T: Clone + std::fmt::Debug + PartialEq + serde::Serialize + serde::de::DeserializeOwned,
    {
    }
    assert_bounds::<Target>();
    assert_bounds::<Filter>();
    assert_bounds::<Output>();
    assert_bounds::<CasUpdate>();
    assert_bounds::<OrderSpec>();
    assert_bounds::<WhereClause>();
    assert_bounds::<WalkDirection>();
    assert_bounds::<Direction>();
    assert_bounds::<WhereOp>();
}

// ── Cross-variant disambiguation ───────────────────────────────────────────────────────

#[test]
fn target_oid_does_not_match_empty() {
    // `{"oid": "..."}` must resolve to Target::Oid, not fall through to Target::Empty.
    let j = json!({"oid": "abc"});
    let t: Target = serde_json::from_value(j).unwrap();
    assert!(matches!(t, Target::Oid { .. }));
    assert!(!matches!(t, Target::Empty {}));
}

#[test]
fn empty_json_resolves_to_target_empty_only() {
    let j = json!({});
    let t: Target = serde_json::from_value(j).unwrap();
    assert!(matches!(t, Target::Empty {}));
}
