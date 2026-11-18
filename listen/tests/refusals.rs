//! P1, what doesn't read: every unknown form is refused at its line and
//! column (reflection seam A1.4: "a refusal with its location, never a
//! silent drop"). Columns count bytes (fixpoint §16.2.1).

use listen::{read, read_reflection, Position, Refusal, Unreadable};
use Unreadable::*;

const HEX: &str = "0976ca1179bfa9425af4522e8796cd7d1bef52992c68ce6a5a1e31c123c91bfa";

fn refused(src: &[u8]) -> (usize, usize, Unreadable) {
    match read(src) {
        Ok(f) => panic!("read {:?}: {f:?}", String::from_utf8_lossy(src)),
        Err(Refusal { at, what }) => (at.line, at.column, what),
    }
}

fn check(cases: &[(&[u8], usize, usize, Unreadable)]) {
    for (src, line, column, what) in cases {
        assert_eq!(
            refused(src),
            (*line, *column, *what),
            "{:?}",
            String::from_utf8_lossy(src)
        );
    }
}

/// A1.4: `rule` and `functional` never appear below the line; pact-eval
/// §1.2: a record starts at column 1, a comment only at column 1.
#[test]
fn unknown_line_forms() {
    check(&[
        (b"rule r p requires q\n", 1, 1, Form),
        (b"query p\n", 1, 1, Form),
        (b"functional p(1, 2) -> 3\n", 1, 1, Form),
        (b"facts p\n", 1, 1, Form),
        (b"Fact p\n", 1, 1, Form),
        (b" fact p\n", 1, 1, Form),
        (b"  # indented\n", 1, 1, Form),
        (b"reflect above 12 grammar 34\n", 1, 1, Form),
        (b"(p)\n", 1, 1, Form),
        (b"fact p\nfact q\nnonsense\n", 3, 1, Form),
        (b"# root 97c6\nfact p\n\nmeaning 12", 4, 1, Form),
    ]);
}

/// pact-eval §1.2: printable ASCII and LF only (CR, TAB, NUL, non-ASCII:
/// dark); fixpoint §16.2.1: a tab or CR doesn't read.
#[test]
fn bytes_outside_the_grammar() {
    check(&[
        (b"fact p\n\tfact q\n", 2, 1, Byte(0x09)),
        (b"fact p\r\n", 1, 7, Byte(0x0d)),
        (b"fact p(a)\r\n", 1, 10, Byte(0x0d)),
        (b"fact p(a,\tb)\n", 1, 10, Byte(0x09)),
        (b"fact\x00p\n", 1, 5, Byte(0x00)),
        (b"# caf\xc3\xa9\n", 1, 6, Byte(0xc3)),
        (b"# ok\n#\x7f\n", 2, 2, Byte(0x7f)),
    ]);
}

/// pact-eval §1.2: `record`, `ground-atom` and `name`.
#[test]
fn malformed_records() {
    check(&[
        (b"fact\n", 1, 5, Expected("a space")),
        (b"fact(p)\n", 1, 5, Expected("a space")),
        (b"fact \n", 1, 6, Expected("a predicate name")),
        (b"fact P\n", 1, 6, Expected("a predicate name")),
        (b"fact 9p\n", 1, 6, Expected("a predicate name")),
        (b"fact not\n", 1, 6, Keyword),
        (b"fact requires(a)\n", 1, 6, Keyword),
        (b"fact p()\n", 1, 8, Expected("an argument")),
        (b"fact p( a)\n", 1, 8, Expected("an argument")),
        (b"fact p(a,)\n", 1, 10, Expected("an argument")),
        (b"fact p(a b)\n", 1, 9, Expected("`,` or `)`")),
        (b"fact p(a ,b)\n", 1, 9, Expected("`,` or `)`")),
        (b"fact p(a\n", 1, 9, Expected("`,` or `)`")),
        (b"fact p(a) x\n", 1, 11, Expected("the end of the line")),
        (b"fact p(a)(b)\n", 1, 10, Expected("the end of the line")),
        (b"fact p-q\n", 1, 7, Expected("the end of the line")),
        (b"fact p q\n", 1, 8, Expected("the end of the line")),
        (b"fact p(A)\n", 1, 8, Expected("an argument")),
        (b"fact p(_a)\n", 1, 8, Expected("an argument")),
    ]);
}

/// §16.10 item 1: `int` and `ratio` are reserved, refused until pinned. A
/// type word without its value, and `not` / `requires`, are not names
/// (pact-eval §1.2; the type words: docs/spec/listen.md §7 Q5).
#[test]
fn keywords_and_reserved_types() {
    check(&[
        (b"fact p(int 3)\n", 1, 8, Reserved),
        (b"fact p(int)\n", 1, 8, Reserved),
        (b"fact p(ratio 1/2)\n", 1, 8, Reserved),
        (b"fact p(a, not)\n", 1, 11, Keyword),
        (b"fact p(requires)\n", 1, 8, Keyword),
        (b"fact p(addr)\n", 1, 8, Keyword),
        (b"fact p(sym, a)\n", 1, 8, Keyword),
        (b"fact p(nat)\n", 1, 8, Keyword),
        (b"fact p(bytes)\n", 1, 8, Keyword),
    ]);
}

/// §5.0.1 items 6–7: `addr <64 lowercase hex>`; an uppercase digit doesn't
/// read, so every address has one text.
#[test]
fn malformed_addrs() {
    let upper = format!("fact p(addr 0976CA{})\n", &HEX[6..]);
    let short = format!("fact p(addr {})\n", &HEX[1..]);
    let long = format!("fact p(addr {HEX}0)\n");
    let twice = format!("fact p(addr  {HEX})\n");
    check(&[
        (upper.as_bytes(), 1, 17, Uppercase),
        (short.as_bytes(), 1, 13, AddrLength),
        (long.as_bytes(), 1, 13, AddrLength),
        (b"fact p(addr xyz)\n", 1, 13, AddrLength),
        (twice.as_bytes(), 1, 13, AddrLength),
    ]);
}

/// §16.10 item 1: `bytes "<lowercase hex>"`, an even number of digits.
#[test]
fn malformed_bytes() {
    check(&[
        (b"fact p(bytes \"abc\")\n", 1, 18, OddHex),
        (b"fact p(bytes \"a\")\n", 1, 16, OddHex),
        (b"fact p(bytes \"0A\")\n", 1, 16, Uppercase),
        (
            b"fact p(bytes \"0g\")\n",
            1,
            16,
            Expected("a lowercase hex digit or `\"`"),
        ),
        (
            b"fact p(bytes \"00\n",
            1,
            17,
            Expected("a lowercase hex digit or `\"`"),
        ),
        (b"fact p(bytes 00)\n", 1, 14, Expected("`\"`")),
    ]);
}

/// §5.0.1 item 6: `nat <decimal>`, no sign, no leading zero; its encoding
/// is `u64(n)` (item 11), so 2⁶⁴ doesn't fit.
#[test]
fn malformed_nats() {
    check(&[
        (b"fact p(nat 007)\n", 1, 12, LeadingZero),
        (b"fact p(nat 00)\n", 1, 12, LeadingZero),
        (b"fact p(nat -1)\n", 1, 12, Expected("a decimal")),
        (b"fact p(nat +1)\n", 1, 12, Expected("a decimal")),
        (b"fact p(nat 18446744073709551616)\n", 1, 12, NatOverflow),
        (b"fact p(nat 99999999999999999999)\n", 1, 12, NatOverflow),
    ]);
}

/// §5.0.1 item 6: `sym "<utf-8>"`, only `\"` and `\\` escape.
#[test]
fn malformed_syms() {
    check(&[
        (b"fact p(sym \"a\\nb\")\n", 1, 14, Escape),
        (b"fact p(sym \"a\\", 1, 14, Escape),
        (b"fact p(sym \"ab\n", 1, 15, Expected("a closing `\"`")),
        (b"fact p(sym \"a\tb\")\n", 1, 14, Byte(0x09)),
        (b"fact p(sym \"a\x7f\")\n", 1, 14, Byte(0x7f)),
        (b"fact p(sym \"\xff\")\n", 1, 13, Utf8),
        (b"fact p(sym \"\xce\")\n", 1, 13, Utf8),
        (b"fact p(sym  \"a\")\n", 1, 12, Expected("`\"`")),
        (b"fact p(sym a)\n", 1, 12, Expected("`\"`")),
    ]);
}

/// Seam's trace audit (mirror `1971b5a1` §4.2): records print an addr as
/// bare hex. A digit-led one is no name, so it doesn't read at all.
#[test]
fn bare_hex_led_by_a_digit_is_no_name() {
    let src = format!("fact both({HEX})\n");
    check(&[(src.as_bytes(), 1, 11, Expected("an argument"))]);
}

/// The reflection's refusals are located in the whole file.
#[test]
fn reflection_refusals_are_located_in_the_file() {
    assert_eq!(
        read_reflection(b"rule r p requires q\n\n---\nfact p\nquery p\n"),
        Err(Refusal {
            at: Position { line: 5, column: 1 },
            what: Form
        })
    );
}

#[test]
fn refusals_print_where_and_why() {
    let r = Refusal {
        at: Position { line: 3, column: 7 },
        what: Form,
    };
    assert_eq!(r.to_string(), "3:7 unknown line form");
    let all = [
        Byte(0x0d),
        Form,
        Expected("x"),
        Keyword,
        Reserved,
        Uppercase,
        AddrLength,
        OddHex,
        LeadingZero,
        NatOverflow,
        Escape,
        Utf8,
    ];
    let texts: std::collections::BTreeSet<String> = all.iter().map(|w| w.to_string()).collect();
    assert_eq!(texts.len(), all.len(), "every reason prints distinctly");
    assert_eq!(Byte(0x0d).to_string(), "byte 0x0d");
    assert_eq!(Expected("x").to_string(), "expected x");
}
