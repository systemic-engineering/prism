//! S4: the type of every argument is kept. Fixpoint §16.10 item 1:
//! "Equality is by the §5.0 bytes", and every argument is `tag ‖ enc(bytes)`
//! (§5.0.1 item 11), so an `addr` and a `sym` (or a plain name) with the same
//! hex are different constants. Seam's trace audit (mirror `1971b5a1` §4.2)
//! found records print both the same way; this reader never conflates them.

use std::collections::BTreeSet;

use listen::{read, Atom, Value};

/// 64 hex digits led by a letter: also a valid pact name.
const HEX: &str = "abcdef0123456789abcdef0123456789abcdef0123456789abcdef0123456789";
/// 64 hex digits led by a digit.
const DIGIT_LED: &str = "0976ca1179bfa9425af4522e8796cd7d1bef52992c68ce6a5a1e31c123c91bfa";

fn one(src: &str) -> Atom {
    let f = read(src.as_bytes()).expect("reads");
    assert_eq!(f.facts.len(), 1);
    f.facts[0].atom.clone()
}

fn arg(src: &str) -> Value {
    one(src).args[0].clone()
}

fn assert_distinct(a: &Value, b: &Value) {
    assert_ne!(a, b);
    assert_ne!(a.cmp(b), std::cmp::Ordering::Equal);
    assert_eq!(BTreeSet::from([a.clone(), b.clone()]).len(), 2);
    let (mut ea, mut eb) = (Vec::new(), Vec::new());
    a.encode(&mut ea);
    b.encode(&mut eb);
    assert_ne!(ea, eb, "the encodings differ by the tag");
    assert_ne!(a.to_string(), b.to_string(), "the canonical texts differ");
}

#[test]
fn a_typed_addr_and_a_plain_name_with_the_same_hex_are_distinct() {
    let addr = arg(&format!("fact p(addr {HEX})\n"));
    let name = arg(&format!("fact p({HEX})\n"));
    assert!(matches!(addr, Value::Addr(_)));
    assert_eq!(name, Value::Sym(HEX.into()));
    assert_distinct(&addr, &name);
    assert_ne!(
        one(&format!("fact p(addr {HEX})\n")),
        one(&format!("fact p({HEX})\n"))
    );
}

#[test]
fn a_typed_addr_and_a_typed_sym_with_the_same_hex_are_distinct() {
    for hex in [HEX, DIGIT_LED] {
        let addr = arg(&format!("fact p(addr {hex})\n"));
        let sym = arg(&format!("fact p(sym \"{hex}\")\n"));
        assert_distinct(&addr, &sym);
    }
}

/// The same 32 bytes as `addr` and as `bytes`: one tag apart.
#[test]
fn addr_and_bytes_with_the_same_bytes_are_distinct() {
    let addr = arg(&format!("fact p(addr {DIGIT_LED})\n"));
    let bytes = arg(&format!("fact p(bytes \"{DIGIT_LED}\")\n"));
    assert_distinct(&addr, &bytes);
}

#[test]
fn a_nat_and_a_sym_with_the_same_digits_are_distinct() {
    assert_distinct(&arg("fact p(nat 7)\n"), &arg("fact p(sym \"7\")\n"));
    assert_distinct(&arg("fact p(nat 0)\n"), &arg("fact p(bytes \"\")\n"));
}

/// §16.10 item 1: "the bare `seq` and `sym "seq"` are one constant."
#[test]
fn a_bare_name_is_the_sym_of_its_text() {
    assert_eq!(arg("fact p(seq)\n"), arg("fact p(sym \"seq\")\n"));
    assert_eq!(one("fact p(seq)\n"), one("fact p(sym \"seq\")\n"));
}

/// §16.10 item 1 (`bytes` is the `0x03` argument) with §5.0.1 item 11
/// (`tag ‖ enc(bytes)`), and §5.0.1 item 10 (content = `enc(p) ‖ u64(k) ‖
/// args`). mirror ships no `bytes` vector; these are written from the text.
#[test]
fn bytes_arguments_encode_with_tag_three() {
    let hexs = |s: &str| -> Vec<u8> {
        (0..s.len())
            .step_by(2)
            .map(|i| u8::from_str_radix(&s[i..i + 2], 16).unwrap())
            .collect()
    };
    assert_eq!(
        one("fact t(bytes \"2f\")\n").content(),
        hexs("000000000000000174" /* enc(t) */)
            .into_iter()
            .chain(hexs("0000000000000001" /* k */))
            .chain(hexs("03" /* tag */))
            .chain(hexs("0000000000000001" /* len */))
            .chain(hexs("2f"))
            .collect::<Vec<u8>>()
    );
    assert_eq!(
        one("fact t(bytes \"\")\n").content(),
        hexs("000000000000000174" /* enc(t) */)
            .into_iter()
            .chain(hexs("0000000000000001" /* k */))
            .chain(hexs("03" /* tag */))
            .chain(hexs("0000000000000000" /* len 0 */))
            .collect::<Vec<u8>>()
    );
    assert_eq!(Value::Bytes(vec![]).tag(), 3);
    assert_eq!(
        [
            Value::Sym(String::new()).kind(),
            Value::Nat(0).kind(),
            Value::Addr([0; 32]).kind(),
            Value::Bytes(vec![]).kind()
        ],
        ["sym", "nat", "addr", "bytes"]
    );
}
