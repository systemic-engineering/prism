//! Identity is scoped: `Oid::hash` is BLAKE3 keyed by the scope's key. The
//! vectors in `docs/spec/identity-vectors.json` were computed with the `b3sum`
//! CLI and Python's `blake3`, never with prism; this test holds prism to them.

use prismqueer::{Oid, Scope};
use serde_json::Value;

fn vectors() -> Value {
    let path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../docs/spec/identity-vectors.json"
    );
    let text = std::fs::read_to_string(path).expect("identity vectors");
    serde_json::from_str(&text).expect("identity vectors parse")
}

fn bytes(hex_str: &str) -> Vec<u8> {
    hex::decode(hex_str).expect("input_hex")
}

fn key(hex_str: &str) -> [u8; 32] {
    hex::decode(hex_str)
        .expect("key hex")
        .try_into()
        .expect("32-byte key")
}

fn scope(name: &Value) -> Scope {
    match name.as_str().expect("scope") {
        "public" => Scope::public(),
        held => Scope::keyed(key(held)),
    }
}

fn address(scope_name: &Value, input_hex: &Value) -> String {
    let input = bytes(input_hex.as_str().expect("input_hex"));
    Oid::hash(&scope(scope_name), &input).as_str().to_owned()
}

#[test]
fn the_public_scope_is_keyed_by_k_public() {
    let v = vectors();
    assert_eq!(
        v["k_public"]["context"],
        prismqueer::oid::PUBLIC_CONTEXT,
        "the context string"
    );
    let k_public = Scope::keyed(key(v["k_public"]["key"].as_str().expect("key")));
    for case in v["cases"].as_array().expect("cases") {
        let input = bytes(case["input_hex"].as_str().expect("input_hex"));
        assert_eq!(
            Oid::hash(&Scope::public(), &input),
            Oid::hash(&k_public, &input),
            "{}",
            case["name"]
        );
    }
}

#[test]
fn every_address_matches_the_vectors() {
    let v = vectors();
    let cases = v["cases"].as_array().expect("cases");
    for case in cases {
        assert_eq!(
            address(&case["scope"], &case["input_hex"]),
            case["address"].as_str().expect("address"),
            "{}",
            case["name"]
        );
    }
    assert_eq!(cases.len(), 7);
}

#[test]
fn the_same_content_in_two_scopes_has_two_addresses() {
    let v = vectors();
    for pair in v["scopes_separate"].as_array().expect("scopes_separate") {
        let under = pair["under"].as_array().expect("under");
        let got: Vec<String> = under
            .iter()
            .map(|u| address(&u["scope"], &pair["input_hex"]))
            .collect();
        for (u, g) in under.iter().zip(&got) {
            assert_eq!(
                g,
                u["address"].as_str().expect("address"),
                "{}",
                pair["name"]
            );
        }
        assert_ne!(got[0], got[1], "{}", pair["name"]);
    }
}

#[test]
fn utf8_inputs_are_their_bytes() {
    let v = vectors();
    for case in v["cases"].as_array().expect("cases") {
        let text = case["input_utf8"].as_str().expect("input_utf8");
        assert_eq!(
            hex::encode(text.as_bytes()),
            case["input_hex"].as_str().expect("input_hex"),
            "{}",
            case["name"]
        );
    }
}
