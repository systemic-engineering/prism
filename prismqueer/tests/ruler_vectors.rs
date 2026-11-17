//! The canonical ruler address is a law (`docs/spec/ruler.md`). The vectors
//! in `docs/spec/ruler-vectors.json` were built from the spec by
//! `ruler-vectors.py` and hashed with Python's `blake3` and the `b3sum` CLI,
//! never with prism; this test holds prism to them.

use num_rational::BigRational;
use prismqueer::algedonic::{
    Calibration, Clock, Draft, Edge, IndependenceDraft, Question, Region, Wald,
};
use prismqueer::Oid;
use serde_json::Value;

fn vectors() -> Value {
    let path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../docs/spec/ruler-vectors.json"
    );
    let text = std::fs::read_to_string(path).expect("ruler vectors");
    serde_json::from_str(&text).expect("ruler vectors parse")
}

fn str_of<'a>(v: &'a Value, key: &str) -> &'a str {
    v[key].as_str().unwrap_or_else(|| panic!("{key}"))
}

fn rational(v: &Value, key: &str) -> BigRational {
    str_of(v, key).parse().expect("n/d")
}

fn edge(v: &Value) -> Edge {
    match str_of(v, "edge") {
        "open" => Edge::Open(rational(v, "value")),
        "closed" => Edge::Closed(rational(v, "value")),
        "unbounded" => Edge::Unbounded,
        other => panic!("edge {other}"),
    }
}

fn region(v: &Value) -> Region {
    Region::new(edge(&v["lo"]), edge(&v["hi"])).expect("non-empty region")
}

fn draft(case: &Value) -> Draft {
    let region = region(&case["region"]);
    let question = match str_of(case, "question") {
        "any" => Question::Any,
        "max" => Question::Max,
        "product" => Question::Product,
        "sum" => Question::Sum,
        other => panic!("question {other}"),
    };
    let wald = Wald::new(rational(case, "alpha"), rational(case, "beta")).expect("valid rates");
    let c = &case["clock"];
    let tick = |k: &str| c[k].as_u64().unwrap_or_else(|| panic!("{k}"));
    Draft::new(
        region,
        question,
        wald,
        Clock::agreed(tick("reaction"), tick("importance"), tick("delay")),
    )
}

#[test]
fn every_ruler_vector_matches_prism() {
    let v = vectors();
    let cases = v["rulers"].as_array().expect("rulers");
    assert_eq!(cases.len(), 4);
    for case in cases {
        let name = str_of(case, "name");
        let draft = draft(case);
        assert_eq!(
            hex::encode(draft.preimage()),
            str_of(case, "preimage_hex"),
            "{name}: preimage"
        );
        let ruler = draft.seal(Oid::new("engineer"));
        assert_eq!(ruler.oid().as_str(), str_of(case, "address"), "{name}");
    }
}

#[test]
fn every_calibration_vector_matches_prism() {
    let v = vectors();
    let cases = v["calibrations"].as_array().expect("calibrations");
    assert_eq!(cases.len(), 4);
    for case in cases {
        let name = str_of(case, "name");
        let cal = Calibration::new(
            Oid::new(str_of(case, "instrument")),
            region(&case["region"]),
            rational(case, "p0"),
            rational(case, "p1"),
            str_of(case, "method"),
        )
        .expect("valid calibration");
        assert_eq!(
            hex::encode(cal.preimage()),
            str_of(case, "preimage_hex"),
            "{name}: preimage"
        );
        assert_eq!(cal.oid().as_str(), str_of(case, "address"), "{name}");
    }
}

#[test]
fn every_independence_vector_matches_prism() {
    let v = vectors();
    let cases = v["independences"].as_array().expect("independences");
    assert_eq!(cases.len(), 2);
    for case in cases {
        let name = str_of(case, "name");
        let instruments = case["instruments"]
            .as_array()
            .expect("instruments")
            .iter()
            .map(|i| Oid::new(i.as_str().expect("instrument")));
        let draft =
            IndependenceDraft::new(instruments, region(&case["region"])).expect("independent");
        assert_eq!(
            hex::encode(draft.preimage()),
            str_of(case, "preimage_hex"),
            "{name}: preimage"
        );
        let sealed = draft.seal(Oid::new("engineer"));
        assert_eq!(sealed.oid().as_str(), str_of(case, "address"), "{name}");
    }
}
