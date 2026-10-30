//! Identity is scoped: `Oid::hash` is BLAKE3 keyed by the scope's key. The
//! vectors in `docs/spec/identity-vectors.json` were computed with the `b3sum`
//! CLI and Python's `blake3`, never with prism; this test holds prism to them.

use prismqueer::Oid;
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

/// RED against today's API: `Oid::hash(bytes)` has no scope yet, so only the
/// public cases can be named, and they must already equal the scoped address.
#[test]
fn public_addresses_match_the_vectors() {
    let v = vectors();
    let mut checked = 0;
    for case in v["cases"].as_array().expect("cases") {
        if case["scope"] != "public" {
            continue;
        }
        let input = bytes(case["input_hex"].as_str().expect("input_hex"));
        let want = case["address"].as_str().expect("address");
        assert_eq!(
            Oid::hash(&input).as_str(),
            want,
            "{}",
            case["name"]
        );
        checked += 1;
    }
    assert_eq!(checked, 4);
}
