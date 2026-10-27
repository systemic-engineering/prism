//! prism-001: the convergence gate's worked examples, checked against a
//! reference reading of the rule in `docs/spec/gate.md`.
//!
//! This evaluator is not a prism feature. It exists so the examples in
//! `docs/spec/gate-vectors.json` cannot quietly contradict the rule as
//! written.

use std::collections::BTreeSet;

use serde_json::{json, Value};

/// One round's state: the draft's address and the sorted set of open items.
type State = (String, BTreeSet<String>);

fn open_items(observers: &serde_json::Map<String, Value>) -> (BTreeSet<String>, usize) {
    let mut open = BTreeSet::new();
    let mut failed = 0;
    for (name, result) in observers {
        match result["status"].as_str().expect("status") {
            "success" => {}
            "partial" => {
                for item in result["loss"].as_array().expect("loss list") {
                    open.insert(format!("{name}: {}", item.as_str().expect("string")));
                }
            }
            "failure" => {
                failed += 1;
                let error = result["error"].as_str().expect("error");
                open.insert(format!("{name}: did not review ({error})"));
            }
            other => panic!("unknown status {other}"),
        }
    }
    (open, failed)
}

fn gate(budget: usize, rounds: &[Value]) -> Value {
    let mut previous: Option<State> = None;
    let mut last: Option<State> = None;
    for round in rounds.iter().take(budget) {
        let observers = round["observers"].as_object().expect("observers");
        let (open, failed) = open_items(observers);
        if failed == observers.len() {
            return json!({ "status": "failure", "error": "unmeasured", "loss": { "unmeasured": open } });
        }
        let state = (round["draft"].as_str().expect("draft").to_string(), open);
        if previous.as_ref() == Some(&state) {
            let (draft, open) = state;
            return if open.is_empty() {
                json!({ "status": "success", "value": draft })
            } else {
                json!({ "status": "partial", "value": draft, "loss": { "kept": open } })
            };
        }
        previous = Some(state.clone());
        last = Some(state);
    }
    let (draft, open) = last.expect("at least one round");
    json!({ "status": "partial", "value": draft, "loss": { "moving": open } })
}

#[test]
fn gate_vectors_agree_with_the_rule() {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../docs/spec/gate-vectors.json");
    let doc: Value = serde_json::from_str(
        &std::fs::read_to_string(&path).expect("docs/spec/gate-vectors.json must exist"),
    )
    .expect("valid json");
    let cases = doc["cases"].as_array().expect("cases");
    assert!(!cases.is_empty());
    for c in cases {
        let budget = c["budget"].as_u64().expect("budget") as usize;
        let rounds = c["rounds"].as_array().expect("rounds");
        assert_eq!(gate(budget, rounds), c["expected"], "case {}", c["name"]);
    }
}

#[test]
fn the_gate_is_never_green_with_an_open_item() {
    // Any open item in the final state means not Success, whatever the rounds.
    let round = |draft: &str, a: Value| json!({ "draft": draft, "observers": { "a": a, "b": { "status": "success", "value": null } } });
    let objection = json!({ "status": "partial", "value": null, "loss": ["x"] });
    let timeout = json!({ "status": "failure", "error": "timeout", "loss": [] });
    for a in [objection, timeout] {
        let rounds = vec![round("d1", a.clone()), round("d1", a)];
        assert_ne!(gate(4, &rounds)["status"], "success");
    }
}
